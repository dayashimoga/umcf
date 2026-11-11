//! # UMCF Chaos — Fault Injection & Invariant Verification Harness
//!
//! Provides deterministic and randomized failure injection (Jepsen-style)
//! for testing network partitions, packet impairments, node churn,
//! thermal storms, and memory pressure across heterogeneous clusters.

#![forbid(unsafe_code)]

pub mod campaign;
pub mod error;
pub mod invariants;
pub mod nemesis;

pub use campaign::{ChaosCampaignReport, ChaosCampaignRunner};
pub use error::ChaosError;
pub use invariants::ClusterInvariantMonitor;
pub use nemesis::{
    ChaosRng, MemoryPressureOomNemesis, NetworkPartitionNemesis, NodeChurnNemesis,
    NodeLifecycleState, PacketImpairmentNemesis, RoutingMatrix, ThermalAndBatteryStormNemesis,
};
