//! # UMCF Soak Suite — Long-Haul Testing & Zero-Leak Memory/FD Profiler
//!
//! Provides automated OLS regression profiling for continuous workload execution,
//! detecting leaks down to bytes per cycle across heap, VRAM, FDs, and temp files.

#![forbid(unsafe_code)]

pub mod error;
pub mod leak_profiler;
pub mod soak_runner;

pub use error::SoakError;
pub use leak_profiler::{LeakProfiler, QuiescenceBarrier, RegressionResult, ResourceSnapshot};
pub use soak_runner::{SoakReport, SoakTestCoordinator};

