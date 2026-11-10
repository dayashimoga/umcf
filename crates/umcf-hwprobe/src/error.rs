//! `umcf-hwprobe` error taxonomy.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Hardware probing and sensor evaluation errors.
#[derive(Debug, Error)]
pub enum HwProbeError {
    #[error("I/O error during hardware probe: {0}")]
    Io(#[from] std::io::Error),

    #[error("CPU probe failed: {0}")]
    CpuProbe(String),

    #[error("Memory probe failed: {0}")]
    MemoryProbe(String),

    #[error("Storage probe failed: {0}")]
    StorageProbe(String),

    #[error("Power/thermal probe failed: {0}")]
    PowerThermalProbe(String),

    #[error("Cgroup limit parse error: {0}")]
    CgroupParse(String),
}
