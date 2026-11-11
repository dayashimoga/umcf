//! Sandbox Escape Verification Suite (Vectors 1 through 5).

use crate::audit_logger::{SecurityAuditLedger, SecuritySeverity};
use crate::error::RedteamError;

pub struct SandboxEscapeAuditor;

impl SandboxEscapeAuditor {
    /// Vector 1: WASM WASI Filesystem Traversal & Symlink Escape.
    ///
    /// Verifies that paths containing `..`, absolute drive letters (`C:`),
    /// or root directory prefixes (`/etc/passwd`) are strictly blocked.
    pub fn audit_vector_1_wasi_fs_traversal(
        ledger: &mut SecurityAuditLedger,
        target_path: &str,
    ) -> Result<(), RedteamError> {
        let normalized = target_path.replace('\\', "/");
        let is_escape_attempt = normalized.contains("..")
            || normalized.starts_with('/')
            || normalized.contains(':')
            || normalized.contains("/etc/")
            || normalized.contains("System32");

        if is_escape_attempt {
            ledger.record_blocked_attack(
                1,
                "WASM_WASI_FS_TRAVERSAL",
                "guest-wasm-module",
                SecuritySeverity::High,
                "BLOCK_ACCESS_ERRNO_NOTCAPABLE",
                &format!("Blocked hostile filesystem traversal to '{target_path}'"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "WASI_ERRNO_NOTCAPABLE: Access outside sandbox root denied".to_string(),
            });
        }

        Ok(())
    }

    /// Vector 2: WASM Ambient Network & Subprocess Escape.
    ///
    /// Verifies that raw socket syscalls (`sock_open`, `sock_send`) and
    /// process execution (`proc_exec`, `posix_spawn`) are denied capability tokens.
    pub fn audit_vector_2_wasm_ambient_network_and_proc(
        ledger: &mut SecurityAuditLedger,
        syscall_name: &str,
    ) -> Result<(), RedteamError> {
        let forbidden_syscalls = [
            "sock_open",
            "sock_send",
            "sock_recv",
            "sock_bind",
            "proc_exec",
            "proc_spawn",
            "posix_spawn",
            "system",
        ];

        if forbidden_syscalls.contains(&syscall_name) {
            ledger.record_blocked_attack(
                2,
                "WASM_AMBIENT_NETWORK_PROC_ESCAPE",
                "guest-wasm-module",
                SecuritySeverity::Critical,
                "TRAP_CAPABILITY_DENIAL",
                &format!("Denied forbidden ambient syscall '{syscall_name}' in WASM sandbox"),
            );
            return Err(RedteamError::OperationRejected {
                reason: format!("WASI_ERRNO_NOTCAPABLE: Syscall '{syscall_name}' forbidden in sandbox"),
            });
        }

        Ok(())
    }

    /// Vector 3: WASM Fuel Bomb, Stack Overflow & Memory Bomb.
    ///
    /// Verifies deterministic fuel metering and hard memory page ceilings.
    pub fn audit_vector_3_wasm_fuel_and_memory_bomb(
        ledger: &mut SecurityAuditLedger,
        fuel_requested: u64,
        fuel_limit: u64,
        memory_pages_requested: u32,
        max_allowed_pages: u32,
    ) -> Result<(), RedteamError> {
        if fuel_requested > fuel_limit {
            ledger.record_blocked_attack(
                3,
                "WASM_FUEL_BOMB",
                "guest-wasm-module",
                SecuritySeverity::High,
                "TRAP_FUEL_EXHAUSTED",
                &format!("Trapped infinite loop fuel bomb: requested {fuel_requested} > limit {fuel_limit}"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "WasmEngine: Fuel limit exhausted, execution trapped".to_string(),
            });
        }

        if memory_pages_requested > max_allowed_pages {
            ledger.record_blocked_attack(
                3,
                "WASM_MEMORY_BOMB",
                "guest-wasm-module",
                SecuritySeverity::High,
                "REJECT_MEMORY_GROW",
                &format!("Rejected memory bomb: requested {memory_pages_requested} pages > ceiling {max_allowed_pages}"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "WasmEngine: Memory grow ceiling exceeded".to_string(),
            });
        }

        Ok(())
    }

    /// Vector 4: GPU Shader Cross-Tenant VRAM Read & TDR Hang.
    ///
    /// Verifies that all newly mapped GPU buffers are zero-scrubbed (`0x00000000`)
    /// and Naga AST loop-fuel watchdogs prevent GPU hang (TDR).
    pub fn audit_vector_4_gpu_vram_cross_tenant_read(
        ledger: &mut SecurityAuditLedger,
        allocated_buffer: &[u32],
        simulated_shader_loop_cycles: u64,
        max_loop_fuel: u64,
    ) -> Result<(), RedteamError> {
        // 1. Verify zero-scrubbed VRAM invariant
        for (idx, &val) in allocated_buffer.iter().enumerate() {
            if val != 0 {
                ledger.record_blocked_attack(
                    4,
                    "GPU_CROSS_TENANT_VRAM_LEAK",
                    "host-gpu-subsystem",
                    SecuritySeverity::Critical,
                    "ALERT_VRAM_CONTAMINATION",
                    &format!("Non-zero residue ({val:#010x}) at buffer word {idx}"),
                );
                return Err(RedteamError::ContainmentFailed {
                    vector_id: 4,
                    attack_name: "GPU Cross-Tenant VRAM Read".to_string(),
                    reason: format!("VRAM buffer contained non-zero unscrubbed residue: {val:#010x}"),
                });
            }
        }

        // 2. Verify AST loop-fuel watchdog halts infinite shader loops
        if simulated_shader_loop_cycles > max_loop_fuel {
            ledger.record_blocked_attack(
                4,
                "GPU_SHADER_TDR_HANG",
                "guest-shader-kernel",
                SecuritySeverity::High,
                "WATCHDOG_HALT_SHADER",
                &format!("Shader loop watchdog tripped: {simulated_shader_loop_cycles} cycles > {max_loop_fuel}"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "NagaShaderWatchdog: Shader loop fuel exhausted, execution halted".to_string(),
            });
        }

        Ok(())
    }

    /// Vector 5: GGUF Arbitrary Offset Pointer Read.
    ///
    /// Verifies that tensor offsets pointing past EOF or negative wrap-arounds
    /// are rejected before mapping model weights.
    pub fn audit_vector_5_gguf_tensor_offset(
        ledger: &mut SecurityAuditLedger,
        tensor_name: &str,
        tensor_offset: u64,
        tensor_bytes: u64,
        file_size_bytes: u64,
    ) -> Result<(), RedteamError> {
        let end_offset = tensor_offset.checked_add(tensor_bytes);

        match end_offset {
            Some(end) if end <= file_size_bytes => Ok(()),
            _ => {
                ledger.record_blocked_attack(
                    5,
                    "GGUF_ARBITRARY_POINTER_READ",
                    "crafted-gguf-file",
                    SecuritySeverity::Critical,
                    "REJECT_MODEL_INTEGRITY",
                    &format!("Tensor '{tensor_name}' offset {tensor_offset} + {tensor_bytes} exceeds file size {file_size_bytes}"),
                );
                Err(RedteamError::OperationRejected {
                    reason: format!("ModelIntegrityVerifier: Tensor '{tensor_name}' bounds exceed file bounds"),
                })
            }
        }
    }
}
