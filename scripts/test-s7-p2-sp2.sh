#!/usr/bin/env bash
# Test Runner for Subphase S7.P2.SP2: Complete `umcf` CLI
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== [1/5] [Unit] Clap parsing, JSON formatting, QR rendering, exit codes ==="
cargo test -p umcf-cli --test cli_tests test_clap_command_tree_parsing_all_subcommands_and_flags
cargo test -p umcf-cli --test cli_tests test_json_output_mode_produces_valid_parseable_json_for_all_commands
cargo test -p umcf-cli --test cli_tests test_utf8_terminal_qr_code_renderer
cargo test -p umcf-cli --test cli_tests test_deterministic_exit_code_mapping_for_error_variants

echo "=== [2/5] [Integration] E2E CLI workflow, shell completions, SIGINT cancel ==="
cargo test -p umcf-cli --test cli_tests test_cli_e2e_workflow_storage_put_job_submit_ai_run_and_gc
cargo test -p umcf-cli --test cli_tests test_shell_completions_generation_bash_zsh_fish_powershell
cargo test -p umcf-cli --test cli_tests test_cli_sigint_ctrl_c_cancels_remote_in_flight_job

echo "=== [3/5] [Simulation] Top TUI headless layout 80x24 and 160x50 ==="
cargo test -p umcf-cli --test cli_tests test_top_headless_terminal_buffer_layout_80x24_and_160x50

echo "=== [4/5] [Hardware-Required] Hardware doctor diagnostics probe ==="
cargo test -p umcf-cli --test cli_tests test_hardware_doctor_diagnostics_report

echo "=== [5/5] [Manually-Verified] QR schema, top text layout, and AI chat ==="
cargo test -p umcf-cli --test cli_tests test_manual_cluster_pair_qr_ai_chat_and_top_cli_schema

echo "All 5 tiers for S7.P2.SP2 PASSED successfully!"
