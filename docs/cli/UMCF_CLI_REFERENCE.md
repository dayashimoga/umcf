# Universal Mobile Compute Fabric — `umcf` CLI Reference

The `umcf` binary is the unified operator and developer command-line interface for the Universal Mobile Compute Fabric (UMCF). It provides complete control over node daemons, cluster topologies, distributed job scheduling, GGUF AI model inference, GPU render farm workloads, segment-parallel media transcoding, remote desktop/gaming streams, FastCDC content-addressed storage, compute credit accounting, and hardware pre-flight diagnostics.

---

## 1. Global Execution Flags & Environment Variables

All `umcf` subcommands honor a standardized set of global flags and environment variables:

| Flag | Short | Environment Variable | Default | Description |
| :--- | :---: | :--- | :--- | :--- |
| `--endpoint <URL>` | - | `UMCF_ENDPOINT` | `http://127.0.0.1:8080` | Target API gateway or coordinator base URL. |
| `--api-key <TOKEN>` | - | `UMCF_API_KEY` | None | Scoped Bearer capability token for authenticated requests. |
| `--format <text\|json>` | `-f`, `--json` | - | `text` | Output format: human-readable tabular text or strict machine-parseable JSON. |
| `--dry-run` | - | - | `false` | Validates command parameters, schemas, and resource budgets without executing. |
| `--quiet` | `-q` | - | `false` | Suppresses non-essential banners, headers, and spinner progress bars. |
| `--verbose` | `-v` | - | `false` | Emits verbose HTTP/QUIC wire traces and sub-millisecond execution timestamps. |
| `--help` | `-h` | - | - | Displays command help and usage syntax. |
| `--version` | `-V` | - | - | Displays CLI version, protocol version, and target triple. |

> **Strict JSON Mode Invariant**: When `--format json` (or `-f`, `--json`) is passed, stdout **strictly** contains valid JSON. Diagnostic progress logs and errors are routed to stderr or structured in RFC 9457 JSON envelopes.

---

## 2. Standardized Deterministic Exit Codes

In accordance with POSIX, BSD sysexits, and UMCF engineering governance, `umcf` returns deterministic exit codes:

| Exit Code | Name | Meaning |
| :---: | :--- | :--- |
| **`0`** | `Ok` | The command completed successfully. |
| **`1`** | `RuntimeError` | Internal execution error, panic catch, or backend service failure. |
| **`2`** | `UsageError` | Missing required argument, unrecognized flag, or schema validation failure. |
| **`3`** | `AuthError` | Missing Bearer token, expired token, or insufficient capability scope. |
| **`4`** | `QuotaExhausted` | Host storage floor reached, Podman 30GB ceiling hit, or credit balance zero. |
| **`5`** | `NetworkTimeout` | Gateway unreachable, connection refused, or QUIC handshake timeout. |
| **`130`** | `SigintInterrupt` | Interrupted by user (`Ctrl+C` / `SIGINT`) with graceful in-flight cancellation. |

---

## 3. Subcommand Tree Reference

### 3.1 `umcf daemon`
Manages the background node daemon (`umcfd`).
- `umcf daemon status`: Checks daemon process health, PID, and uptime.
- `umcf daemon start`: Starts the local daemon and QUIC UDP listener.
- `umcf daemon stop`: Gracefully stops the daemon, unpinning ephemeral shards.
- `umcf daemon logs`: Tails recent daemon journal logs.
- `umcf daemon doctor`: Runs the hardware and container quota diagnostic suite.

### 3.2 `umcf cluster` & `umcf top`
Manages mesh topologies, device pairing, and live node resource monitoring.
- `umcf cluster nodes`: Lists all active coordinators and worker nodes with CPU/GPU/RAM specs.
- `umcf cluster pair [--qr]`: Generates a short-lived pairing URI and 6-digit PIN. With `--qr`, renders a UTF-8 half-block QR code directly in the terminal for instant mobile enrollment.
- `umcf cluster join <INVITE_CODE>`: Enrolls the local node into an existing mesh using a pairing invitation.
- `umcf cluster top` (or `umcf top`): Launches a live terminal UI monitor tracking cluster-wide CPU cores, RAM, storage, network P95 RTT, and per-node status.

### 3.3 `umcf job`
Submits, tracks, and manages distributed compute tasks (WASM / native).
- `umcf job submit --manifest <FILE>`: Submits a declarative workload manifest to the scheduler DAG.
- `umcf job list`: Lists active and completed jobs with progress percentages.
- `umcf job status <JOB_ID>`: Queries real-time execution status and assigned node DID.
- `umcf job logs <JOB_ID>`: Streams captured stdout/stderr logs from worker sandboxes.
- `umcf job watch <JOB_ID>`: Blocks and streams progress until completion or error.
- `umcf job cancel <JOB_ID>`: Sends an authenticated cancellation token to terminate the remote job.

### 3.4 `umcf ai`
Executes distributed LLM and audio transcription inference using `llama.cpp`.
- `umcf ai models list`: Lists locally resident and cluster-available GGUF models.
- `umcf ai models pull <MODEL_NAME>`: Pre-caches model weights into local NVMe/SSD storage.
- `umcf ai run --model <MODEL> --prompt <TEXT>`: Submits a prompt to the distributed inference pipeline and streams generated tokens with TTFT and tokens/sec telemetry.
- `umcf ai chat --model <MODEL>`: Starts an interactive multi-turn terminal chat session.

### 3.5 `umcf render` & `umcf media`
Distributed GPU rendering and parallel media transcoding.
- `umcf render submit --scene <SCENE>`: Distributes 3D tile/frame rendering across cluster GPUs.
- `umcf media transcode --input <FILE>`: Splits video into GOP chunks for segment-parallel transcoding with Annex-B bitstream restitching.

### 3.6 `umcf stream`
Remote desktop and game streaming session control.
- `umcf stream start --target <NODE_ID>`: Establishes an ultra-low-latency QUIC video/audio stream.
- `umcf stream status`: Displays active stream bitrate, FPS, and frame drop metrics.
- `umcf stream stop --session <SESSION_ID>`: Terminates the remote stream session.

### 3.7 `umcf storage`
FastCDC content-addressed distributed storage with Reed-Solomon erasure coding.
- `umcf storage put <FILE>`: Ingests a file, chunks via FastCDC, computes BLAKE3 root CID, and distributes RS(4,2) shards.
- `umcf storage get <CID>`: Reassembles and downloads a file by its content identifier.
- `umcf storage pin <CID>`: Pins content for guaranteed offline availability.
- `umcf storage quota`: Displays partition free space, cache utilization, and reclaimable bytes.
- `umcf storage gc`: Executes concurrent W-TinyLFU garbage collection and scrubs corrupted blocks.

### 3.8 `umcf market`
Decentralized compute credit accounting and billing.
- `umcf market balance`: Queries available UMCF-CREDITS balance and pending settlements.
- `umcf market buy <CREDITS>`: Purchases compute credits through authenticated payment provider.
- `umcf market history`: Lists cryptographic micro-billing receipts.

### 3.9 `umcf doctor`
Pre-flight verification of host hardware and container quotas:
- CPU topology and SIMD extensions (AVX2, FMA, NEON, SVE).
- Host partition 15% safety floor.
- Podman strict 30 GiB storage ceiling enforcement.
- UDP QUIC socket creation and MTU discovery.
- GPU/NPU driver probing (Vulkan, Metal, DirectX 12, CUDA).

### 3.10 `umcf completions`
Generates native shell completion scripts:
- `umcf completions bash`
- `umcf completions zsh`
- `umcf completions fish`
- `umcf completions powershell`
