//! Universal Mobile Compute Fabric (UMCF) Hardware Probe Crate.
//! Real OS hardware querying, cgroup limits awareness, and thermal/power tracking.
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

pub mod accelerator_probe;
pub mod cpu;
pub mod error;
pub mod memory_storage;
pub mod power_thermal;
pub mod system_probe;

pub use accelerator_probe::{AcceleratorProber, CudaRocmProber, GpuEnumerator, NpuEnumerator};
pub use cpu::CpuProber;
pub use error::HwProbeError;
pub use memory_storage::MemoryStorageProber;
pub use power_thermal::PowerThermalProber;
pub use system_probe::SystemProber;
