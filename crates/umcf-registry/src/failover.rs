//! Multi-endpoint failover and circuit breaker client for UMCF Control Plane.
//! Priority tiers: Primary (Cloudflare Workers) -> Secondary (Google Cloud Run) -> Tertiary (LAN Autonomous Master).
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::time::{SystemTime, UNIX_EPOCH};

use umcf_core::{NodeId, NodeRole};

use crate::cloud_sync::{
    CloudPeerRecord, CloudflareControlPlaneClient, RegisterResponse, SyncDeltaResponse,
};
use crate::crdt::RegistryDelta;
use crate::error::RegistryError;

/// Operational tier currently serving control plane coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlPlaneTier {
    /// Primary edge coordinator: Cloudflare Workers + Durable Objects.
    Primary,
    /// Stateless container fallback: Google Cloud Run.
    Secondary,
    /// Local LAN-only autonomous master mode (offline / partitioned).
    AutonomousLan,
}

/// State of a circuit breaker protecting an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation: requests pass through.
    Closed,
    /// Tripped after threshold failures: fast-fails immediately to next tier.
    Open,
    /// Testing health after cooldown timeout.
    HalfOpen,
}

/// Circuit breaker implementation tracking consecutive failures and cooldown timeouts.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub failure_threshold: usize,
    pub reset_timeout_ms: u64,
    state: CircuitState,
    consecutive_failures: usize,
    last_failure_ms: u64,
}

impl CircuitBreaker {
    /// Creates a new `CircuitBreaker`.
    #[must_use]
    pub fn new(failure_threshold: usize, reset_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            reset_timeout_ms,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            last_failure_ms: 0,
        }
    }

    /// Returns the current circuit state given the current wall-clock time in ms.
    pub fn state(&mut self, now_ms: u64) -> CircuitState {
        match self.state {
            CircuitState::Open => {
                if now_ms.saturating_sub(self.last_failure_ms) >= self.reset_timeout_ms {
                    self.state = CircuitState::HalfOpen;
                }
                self.state
            }
            s => s,
        }
    }

    /// Whether an attempt through this breaker is allowed.
    pub fn can_attempt(&mut self, now_ms: u64) -> bool {
        match self.state(now_ms) {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    /// Records a successful operation, resetting consecutive failures and closing the circuit.
    pub fn record_success(&mut self) {
        self.state = CircuitState::Closed;
        self.consecutive_failures = 0;
    }

    /// Records an operation failure, tripping to `Open` when threshold is exceeded.
    pub fn record_failure(&mut self, now_ms: u64) {
        self.consecutive_failures += 1;
        self.last_failure_ms = now_ms;
        if self.consecutive_failures >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    /// Forcefully trips the circuit breaker to `Open`.
    pub fn trip(&mut self, now_ms: u64) {
        self.state = CircuitState::Open;
        self.consecutive_failures = self.failure_threshold;
        self.last_failure_ms = now_ms;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(2, 2_000) // 2 consecutive failures trip circuit; 2.0s cooldown
    }
}

/// Multi-endpoint failover client managing graceful degradation across Cloudflare, Cloud Run, and LAN.
#[derive(Debug)]
pub struct FailoverControlPlaneClient {
    primary: Option<CloudflareControlPlaneClient>,
    secondary: Option<CloudflareControlPlaneClient>,
    primary_breaker: CircuitBreaker,
    secondary_breaker: CircuitBreaker,
    active_tier: ControlPlaneTier,
    pending_deltas: Vec<RegistryDelta>,
    cluster_id: String,
    node_id: NodeId,
}

impl FailoverControlPlaneClient {
    /// Creates a new `FailoverControlPlaneClient`.
    #[must_use]
    pub fn new(
        primary: Option<CloudflareControlPlaneClient>,
        secondary: Option<CloudflareControlPlaneClient>,
        cluster_id: String,
        node_id: NodeId,
    ) -> Self {
        let initial_tier = if primary.is_some() {
            ControlPlaneTier::Primary
        } else if secondary.is_some() {
            ControlPlaneTier::Secondary
        } else {
            ControlPlaneTier::AutonomousLan
        };

        Self {
            primary,
            secondary,
            primary_breaker: CircuitBreaker::default(),
            secondary_breaker: CircuitBreaker::default(),
            active_tier: initial_tier,
            pending_deltas: Vec::new(),
            cluster_id,
            node_id,
        }
    }

    /// Returns the currently active operational tier.
    #[must_use]
    pub fn active_tier(&self) -> ControlPlaneTier {
        self.active_tier
    }

    /// Returns the count of pending deltas queued for synchronization.
    #[must_use]
    pub fn pending_delta_count(&self) -> usize {
        self.pending_deltas.len()
    }

    /// Forces the primary circuit breaker to trip (useful for testing failover).
    pub fn trip_primary(&mut self) {
        self.primary_breaker.trip(current_time_ms());
    }

    /// Forces the secondary circuit breaker to trip (useful for testing LAN fallback).
    pub fn trip_secondary(&mut self) {
        self.secondary_breaker.trip(current_time_ms());
    }

    /// Synchronizes a registry delta with automatic failover and queue buffering.
    ///
    /// # Errors
    /// Returns `RegistryError` only if local queuing or irreversible serialization fails.
    pub async fn sync_delta(
        &mut self,
        delta: &RegistryDelta,
    ) -> Result<SyncDeltaResponse, RegistryError> {
        let now_ms = current_time_ms();

        // 1. Try Primary (Cloudflare Workers)
        if let Some(ref client) = self.primary {
            if self.primary_breaker.can_attempt(now_ms) {
                match client.sync_delta(delta).await {
                    Ok(resp) => {
                        self.primary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Primary;
                        return Ok(resp);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Primary control plane sync failed: {e}; attempting fallback"
                        );
                        self.primary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        // 2. Try Secondary (Google Cloud Run fallback)
        if let Some(ref client) = self.secondary {
            if self.secondary_breaker.can_attempt(now_ms) {
                match client.sync_delta(delta).await {
                    Ok(resp) => {
                        self.secondary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Secondary;
                        return Ok(resp);
                    }
                    Err(e) => {
                        tracing::warn!("Secondary control plane sync failed: {e}; falling back to LAN autonomous mode");
                        self.secondary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        // 3. Tertiary Fallback: LAN Autonomous Master mode
        self.active_tier = ControlPlaneTier::AutonomousLan;
        self.pending_deltas.push(delta.clone());

        Ok(SyncDeltaResponse {
            accepted: true,
            current_hlc: delta.delta_hlc,
            peer_count: 1,
        })
    }

    /// Flushes queued deltas accumulated during offline / autonomous operation once connectivity is restored.
    ///
    /// # Errors
    /// Returns `RegistryError` if remote synchronization fails.
    pub async fn flush_pending_deltas(&mut self) -> Result<usize, RegistryError> {
        if self.pending_deltas.is_empty() {
            return Ok(0);
        }

        let now_ms = current_time_ms();
        let target_client = if let Some(ref p) = self.primary {
            if self.primary_breaker.can_attempt(now_ms) {
                Some(p)
            } else if let Some(ref s) = self.secondary {
                if self.secondary_breaker.can_attempt(now_ms) {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(ref s) = self.secondary {
            if self.secondary_breaker.can_attempt(now_ms) {
                Some(s)
            } else {
                None
            }
        } else {
            None
        };

        let Some(client) = target_client else {
            return Ok(0);
        };

        let count = self.pending_deltas.len();
        for d in &self.pending_deltas {
            client.sync_delta(d).await?;
        }

        self.pending_deltas.clear();
        Ok(count)
    }

    /// Registers this node through the active control plane tier.
    ///
    /// # Errors
    /// Returns `RegistryError` on network or HTTP failure across available tiers.
    pub async fn register(
        &mut self,
        role: NodeRole,
        rendezvous_endpoints: Vec<String>,
    ) -> Result<RegisterResponse, RegistryError> {
        let now_ms = current_time_ms();

        // 1. Try Primary
        if let Some(ref client) = self.primary {
            if self.primary_breaker.can_attempt(now_ms) {
                match client.register(role, rendezvous_endpoints.clone()).await {
                    Ok(resp) => {
                        self.primary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Primary;
                        return Ok(resp);
                    }
                    Err(e) => {
                        tracing::warn!("Primary register failed: {e}");
                        self.primary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        // 2. Try Secondary
        if let Some(ref client) = self.secondary {
            if self.secondary_breaker.can_attempt(now_ms) {
                match client.register(role, rendezvous_endpoints.clone()).await {
                    Ok(resp) => {
                        self.secondary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Secondary;
                        return Ok(resp);
                    }
                    Err(e) => {
                        tracing::warn!("Secondary register failed: {e}");
                        self.secondary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        // 3. Fallback to LAN autonomous registration
        self.active_tier = ControlPlaneTier::AutonomousLan;
        Ok(RegisterResponse {
            status: "registered_lan_autonomous".into(),
            cluster_id: self.cluster_id.clone(),
            node_id: self.node_id.to_string(),
            active_peer_count: 1,
            peer: CloudPeerRecord {
                node_id: self.node_id.to_string(),
                role: format!("{role:?}"),
                status: "Active".into(),
                rendezvous_endpoints,
                last_seen_ms: now_ms,
            },
        })
    }

    /// Fetches cluster peers through the active tier.
    ///
    /// # Errors
    /// Returns `RegistryError` on failure.
    pub async fn fetch_peers(&mut self) -> Result<Vec<CloudPeerRecord>, RegistryError> {
        let now_ms = current_time_ms();

        if let Some(ref client) = self.primary {
            if self.primary_breaker.can_attempt(now_ms) {
                match client.fetch_peers().await {
                    Ok(peers) => {
                        self.primary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Primary;
                        return Ok(peers);
                    }
                    Err(_) => {
                        self.primary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        if let Some(ref client) = self.secondary {
            if self.secondary_breaker.can_attempt(now_ms) {
                match client.fetch_peers().await {
                    Ok(peers) => {
                        self.secondary_breaker.record_success();
                        self.active_tier = ControlPlaneTier::Secondary;
                        return Ok(peers);
                    }
                    Err(_) => {
                        self.secondary_breaker.record_failure(now_ms);
                    }
                }
            }
        }

        self.active_tier = ControlPlaneTier::AutonomousLan;
        Ok(vec![CloudPeerRecord {
            node_id: self.node_id.to_string(),
            role: "Master".into(),
            status: "Active".into(),
            rendezvous_endpoints: vec![],
            last_seen_ms: now_ms,
        }])
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
