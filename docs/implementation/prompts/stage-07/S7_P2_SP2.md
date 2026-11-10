# Subphase S7.P2.SP2 Execution Prompt: Complete `umcf` CLI for Cluster, Job, Model, Storage & Streaming Control

## 1. Role & Context
You are the Command-Line Interface & Systems Tooling Engineer for UMCF. In Subphase `S7.P2.SP2`, you will implement `crates/umcf-cli` producing the `umcf` binary. Whether a developer is pairing a new mobile phone via QR code in the terminal, submitting a distributed WASM/GPU/Media job from a CI pipeline, pulling and running an LLM in the terminal, or inspecting live cluster telemetry via `umcf top`, the `umcf` CLI is the definitive command-line control plane for the fabric.

## 2. Prerequisites & Dependencies
- `S7.P2.SP1` (`crates/umcf-sdk` client library and API Gateway).
- `clap` (derive + shell completion generation), `ratatui` / `indicatif` (terminal UI & multi-progress bars), `comfy-table` (aligned tables).

## 3. Implementation Tasks
1. **CLI Command Hierarchy (`crates/umcf-cli/src/main.rs`, `src/commands/*.rs`)**:
   - Implement global flags on all commands: `--endpoint <URL>`, `--profile <NAME>`, `--api-key <KEY>`, `--format <text|json|yaml>`, `--quiet`, `--verbose`, `--timeout <DURATION>`, and `--dry-run`.
   - Implement complete subcommand tree:
     - **`umcf daemon`**: `start [--foreground]`, `stop`, `status`, `logs [--follow]`, `doctor` (checks GPU drivers, Vulkan/Metal, FFmpeg, firewall/QUIC UDP ports, storage permissions).
     - **`umcf cluster`**: `nodes` (lists peers, OS, CPU/GPU/NPU, RAM, battery/thermal state, RTT), `pair [--qr]` (renders UTF-8 half-block QR code in terminal + 6-digit PIN), `join <INVITE_CODE>`, `leave`, `top` (interactive `ratatui` real-time cluster dashboard).
     - **`umcf job`**: `submit <MANIFEST_OR_WASM>`, `list [--state running|completed|failed]`, `status <JOB_ID>`, `logs <JOB_ID> [--follow]`, `cancel <JOB_ID>`, `watch <JOB_ID>`.
     - **`umcf ai`**: `models list`, `models pull <MODEL_ID_OR_PATH>`, `models inspect <FILE>`, `models rm <MODEL_ID>`, `run <MODEL> [PROMPT] [--privacy strict-local|trusted-devices|cluster]`, `chat <MODEL>` (interactive multi-turn terminal chat with live tok/s & layer split footer), `serve [--port 11435]`.
     - **`umcf render`** & **`umcf media`**: `render submit <SHADER_WGSL> --resolution 3840x2160 -o out.png`, `media probe`, `media transcode <INPUT> -o <OUTPUT> --codec av1|hevc|h264`.
     - **`umcf stream`**: `host start [--monitor 0]`, `host stop`, `sessions list`, `sessions kick <SESSION_ID>`.
     - **`umcf storage`**: `put <PATH> [--redundancy rs-4-2] [--pin]`, `get <CID> -o <PATH>`, `ls`, `pin add <CID>`, `pin rm <CID>`, `quota get`, `quota set <SIZE>`, `gc [--scrub]`.
     - **`umcf market`**: `balance`, `receipts list`, `offers list`, `offers publish`, `orders list`.
     - **`umcf completions <bash|zsh|fish|powershell>`**: Generates native shell completion scripts.
2. **Deterministic Exit Codes & Structured JSON Mode**:
   - Guarantee that when `--format json` is passed, `stdout` contains **strictly valid JSON** (with progress bars and logs routed exclusively to `stderr` or suppressed via `--quiet`) so CI scripts (`umcf storage put data.bin --format json | jq -r .root_cid`) work reliably.
   - Standardize exit codes: `0` (Ok), `1` (Internal/Runtime Error), `2` (CLI Usage / Validation Error), `3` (Authentication / Permission Denied), `4` (Resource / Quota Exhausted), `5` (Network / Timeout Error), `130` (Ctrl-C Graceful Interrupt — cancels active remote job unless `--detach` was specified).

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-cli`; 100% test pass rate.
- Every subcommand (`daemon`, `cluster`, `job`, `ai`, `render`, `media`, `stream`, `storage`, `market`, `completions`) executes against a live in-process test daemon with both `--format text` and `--format json`.
- Pressing `Ctrl-C` (`SIGINT`) during `umcf job submit --wait`, `umcf ai run`, or `umcf media transcode` sends an explicit cancellation request to the daemon before exiting with code `130` within <= 100 ms.
- `umcf daemon doctor` accurately diagnoses missing optional accelerators or blocked ports and outputs actionable remediation steps.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_clap_command_tree_parsing_all_subcommands_and_flags`
  - `test_json_output_mode_produces_valid_parseable_json_for_all_commands`
  - `test_utf8_terminal_qr_code_renderer`
  - `test_deterministic_exit_code_mapping_for_error_variants`
- **`[INT]`**:
  - `test_cli_e2e_workflow_storage_put_job_submit_ai_run_and_gc`
  - `test_cli_sigint_ctrl_c_cancels_remote_in_flight_job`
  - `test_shell_completions_generation_bash_zsh_fish_powershell`
- **`[SIM]`**:
  - Run `umcf top` headless against a simulated 8-node cluster state buffer verifying `ratatui` terminal buffer layout at `80x24` and `160x50` terminal dimensions without panicking.
- **`[HW]`**:
  - Run `umcf daemon doctor` on host machine and verify hardware detection report.
- **`[MANUAL]`**:
  - Execute `umcf cluster pair --qr`, `umcf ai chat`, and `umcf top` in an interactive terminal session.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p2/sp2_cli_json_schema_outputs.json`
- `artifacts/stage-07/p2/sp2_ratatui_top_snapshot.txt`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/cli/UMCF_CLI_REFERENCE.md` documenting all commands, flags, environment variables (`UMCF_ENDPOINT`, `UMCF_API_KEY`), and exit codes.
- Mark `S7.P2.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If `--format json` ever pollutes `stdout` with non-JSON ANSI spinner sequences, or if `Ctrl-C` leaves orphan running jobs when `--detach` was not passed.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P2.SP3`.
