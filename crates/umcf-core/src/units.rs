//! Checked arithmetic resource units and strongly-typed hardware metrics.
//! Prevents integer overflow, underflow, and truncation across all operations.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// Strongly typed memory or storage capacity in bytes with checked arithmetic.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct ByteSize(pub u64);

impl ByteSize {
    pub const ZERO: Self = Self(0);
    pub const KIB: u64 = 1024;
    pub const MIB: u64 = 1024 * 1024;
    pub const GIB: u64 = 1024 * 1024 * 1024;

    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn from_kib(kib: u64) -> Self {
        Self(kib.saturating_mul(Self::KIB))
    }

    #[must_use]
    pub const fn from_mib(mib: u64) -> Self {
        Self(mib.saturating_mul(Self::MIB))
    }

    #[must_use]
    pub const fn from_gib(gib: u64) -> Self {
        Self(gib.saturating_mul(Self::GIB))
    }

    #[must_use]
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn as_kib_f64(self) -> f64 {
        self.0 as f64 / Self::KIB as f64
    }

    #[must_use]
    pub fn as_mib_f64(self) -> f64 {
        self.0 as f64 / Self::MIB as f64
    }

    #[must_use]
    pub fn as_gib_f64(self) -> f64 {
        self.0 as f64 / Self::GIB as f64
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn checked_div(self, rhs: u64) -> Option<Self> {
        self.0.checked_div(rhs).map(Self)
    }

    #[must_use]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0;
        if b >= Self::GIB {
            write!(f, "{:.2} GiB", self.as_gib_f64())
        } else if b >= Self::MIB {
            write!(f, "{:.2} MiB", self.as_mib_f64())
        } else if b >= Self::KIB {
            write!(f, "{:.2} KiB", self.as_kib_f64())
        } else {
            write!(f, "{} B", b)
        }
    }
}

impl Add for ByteSize {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("ByteSize arithmetic overflow in Add")
    }
}

impl Sub for ByteSize {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
            .expect("ByteSize arithmetic underflow in Sub")
    }
}

/// CPU compute quota measured in milli-cores (1000 milli-cores = 1 full CPU core).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct MilliCores(pub u32);

impl MilliCores {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn from_millis(millis: u32) -> Self {
        Self(millis)
    }

    #[must_use]
    pub const fn from_cores(cores: u32) -> Self {
        Self(cores.saturating_mul(1000))
    }

    #[must_use]
    pub const fn as_millis(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn as_cores_f64(self) -> f64 {
        f64::from(self.0) / 1000.0
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u32) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl fmt::Display for MilliCores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m ({:.2} cores)", self.0, self.as_cores_f64())
    }
}

/// Compute throughput measured in FP16 TeraFlops.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct TeraFlopsFp16(pub u32);

impl TeraFlopsFp16 {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn from_tflops(tflops: u32) -> Self {
        Self(tflops)
    }

    #[must_use]
    pub const fn as_tflops(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl fmt::Display for TeraFlopsFp16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} TFLOPS (FP16)", self.0)
    }
}

/// Network or memory transfer rate in megabytes per second.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct MegabytesPerSec(pub u32);

impl MegabytesPerSec {
    #[must_use]
    pub const fn from_mbps(mbps: u32) -> Self {
        Self(mbps)
    }

    #[must_use]
    pub const fn as_mbps(self) -> u32 {
        self.0
    }
}

impl fmt::Display for MegabytesPerSec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} MB/s", self.0)
    }
}
