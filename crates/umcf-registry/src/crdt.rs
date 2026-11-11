//! Observed-Remove Set (OR-Set) and LWW-Element-Set CRDTs for cluster registry state.
//! Mathematically guarantees commutative, associative, and idempotent state synchronization.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use umcf_core::capabilities::HardwareCapabilityDescriptor;
use umcf_core::{NodeId, NodeRole};
use umcf_enrollment::ResourceConsentPolicy;
use umcf_identity::RevokedCertificateEntry;

use crate::hlc::HlcTimestamp;

/// Operational status of a cluster node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Actively contributing resources and exchanging heartbeats.
    Active,
    /// Gracefully finishing current jobs before planned shutdown.
    Draining,
    /// Unresponsive or temporarily disconnected from mesh.
    Offline,
    /// Explicitly revoked or expelled from the cluster.
    Revoked,
}

/// Metadata record for an enrolled node in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub node_id: NodeId,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub endpoints: Vec<String>,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
    pub hlc: HlcTimestamp,
}

/// Observed-Remove Set / LWW CRDT entry for a node membership record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCrdtEntry {
    pub record: NodeRecord,
    pub add_hlc: HlcTimestamp,
    pub remove_hlc: Option<HlcTimestamp>,
}

impl NodeCrdtEntry {
    /// Returns true if this node is actively enrolled and not removed/tombstoned.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.remove_hlc.is_none_or(|rem| self.add_hlc > rem)
    }

    /// Merges two entries following commutative join-semilattice laws.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let add_hlc = self.add_hlc.max(other.add_hlc);
        let remove_hlc = match (self.remove_hlc, other.remove_hlc) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let record = if self.add_hlc > other.add_hlc {
            self.record.clone()
        } else if other.add_hlc > self.add_hlc {
            other.record.clone()
        } else {
            // Deterministic tie-breaker on serialized payload bytes
            let self_bytes = bincode::serialize(&self.record).unwrap_or_default();
            let other_bytes = bincode::serialize(&other.record).unwrap_or_default();
            if self_bytes >= other_bytes {
                self.record.clone()
            } else {
                other.record.clone()
            }
        };

        Self {
            record,
            add_hlc,
            remove_hlc,
        }
    }
}

/// Last-Write-Wins CRDT entry for hardware capability descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCrdtEntry {
    pub node_id: NodeId,
    pub descriptor: HardwareCapabilityDescriptor,
    pub hlc: HlcTimestamp,
}

impl CapabilityCrdtEntry {
    /// Merges two capability entries deterministically.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        if self.hlc > other.hlc {
            self.clone()
        } else if other.hlc > self.hlc {
            other.clone()
        } else {
            let self_bytes = bincode::serialize(&self.descriptor).unwrap_or_default();
            let other_bytes = bincode::serialize(&other.descriptor).unwrap_or_default();
            if self_bytes >= other_bytes {
                self.clone()
            } else {
                other.clone()
            }
        }
    }
}

/// Last-Write-Wins CRDT entry for explicit resource consent policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentCrdtEntry {
    pub node_id: NodeId,
    pub policy: ResourceConsentPolicy,
    pub hlc: HlcTimestamp,
}

impl ConsentCrdtEntry {
    /// Merges two consent entries deterministically.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        if self.hlc > other.hlc {
            self.clone()
        } else if other.hlc > self.hlc {
            other.clone()
        } else {
            let self_bytes = bincode::serialize(&self.policy).unwrap_or_default();
            let other_bytes = bincode::serialize(&other.policy).unwrap_or_default();
            if self_bytes >= other_bytes {
                self.clone()
            } else {
                other.clone()
            }
        }
    }
}

/// Last-Write-Wins CRDT entry for revoked certificate records (CRL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrlCrdtEntry {
    pub serial_number: String,
    pub entry: RevokedCertificateEntry,
    pub hlc: HlcTimestamp,
}

impl CrlCrdtEntry {
    /// Merges two CRL entries deterministically.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        if self.hlc > other.hlc {
            self.clone()
        } else if other.hlc > self.hlc {
            other.clone()
        } else {
            let self_bytes = bincode::serialize(&self.entry).unwrap_or_default();
            let other_bytes = bincode::serialize(&other.entry).unwrap_or_default();
            if self_bytes >= other_bytes {
                self.clone()
            } else {
                other.clone()
            }
        }
    }
}

/// Summary of mutated and stale elements after merging a delta or remote state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeSummary {
    pub nodes_added: usize,
    pub nodes_updated: usize,
    pub nodes_removed: usize,
    pub capabilities_updated: usize,
    pub consent_updated: usize,
    pub crl_updated: usize,
    pub ignored_stale: usize,
}

/// Delta-state envelope carrying causal mutations between nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryDelta {
    pub cluster_id: String,
    pub sender_node_id: NodeId,
    pub delta_hlc: HlcTimestamp,
    pub nodes: Vec<NodeCrdtEntry>,
    pub capabilities: Vec<CapabilityCrdtEntry>,
    pub consent: Vec<ConsentCrdtEntry>,
    pub crl_entries: Vec<CrlCrdtEntry>,
}

/// Complete CRDT state for the cluster registry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterRegistryState {
    pub cluster_id: String,
    pub nodes: BTreeMap<NodeId, NodeCrdtEntry>,
    pub capabilities: BTreeMap<NodeId, CapabilityCrdtEntry>,
    pub consent: BTreeMap<NodeId, ConsentCrdtEntry>,
    pub crl_entries: BTreeMap<String, CrlCrdtEntry>,
}

impl ClusterRegistryState {
    /// Creates an empty registry state for a cluster.
    #[must_use]
    pub fn new(cluster_id: impl Into<String>) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            nodes: BTreeMap::new(),
            capabilities: BTreeMap::new(),
            consent: BTreeMap::new(),
            crl_entries: BTreeMap::new(),
        }
    }

    /// Inserts or updates a node membership record at a specific HLC timestamp.
    pub fn insert_node(&mut self, record: NodeRecord, add_hlc: HlcTimestamp) {
        let node_id = record.node_id.clone();
        let new_entry = NodeCrdtEntry {
            record,
            add_hlc,
            remove_hlc: None,
        };

        let merged = match self.nodes.get(&node_id) {
            Some(existing) => existing.merge(&new_entry),
            None => new_entry,
        };
        self.nodes.insert(node_id, merged);
    }

    /// Tombstones/removes a node membership record at a specific HLC timestamp.
    pub fn remove_node(&mut self, node_id: &NodeId, remove_hlc: HlcTimestamp) {
        if let Some(existing) = self.nodes.get_mut(node_id) {
            let updated_rem = match existing.remove_hlc {
                Some(prev) => prev.max(remove_hlc),
                None => remove_hlc,
            };
            existing.remove_hlc = Some(updated_rem);
        } else {
            // Record tombstone even if node was not previously known locally
            let dummy_record = NodeRecord {
                node_id: node_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Offline,
                endpoints: vec![],
                registered_at_ms: 0,
                last_heartbeat_ms: 0,
                hlc: HlcTimestamp::MIN,
            };
            self.nodes.insert(
                node_id.clone(),
                NodeCrdtEntry {
                    record: dummy_record,
                    add_hlc: HlcTimestamp::MIN,
                    remove_hlc: Some(remove_hlc),
                },
            );
        }
    }

    /// Updates probed hardware capabilities for a node.
    pub fn update_capabilities(
        &mut self,
        node_id: NodeId,
        descriptor: HardwareCapabilityDescriptor,
        hlc: HlcTimestamp,
    ) {
        let new_entry = CapabilityCrdtEntry {
            node_id: node_id.clone(),
            descriptor,
            hlc,
        };
        let merged = match self.capabilities.get(&node_id) {
            Some(existing) => existing.merge(&new_entry),
            None => new_entry,
        };
        self.capabilities.insert(node_id, merged);
    }

    /// Updates signed resource consent policy for a node.
    pub fn update_consent(
        &mut self,
        node_id: NodeId,
        policy: ResourceConsentPolicy,
        hlc: HlcTimestamp,
    ) {
        let new_entry = ConsentCrdtEntry {
            node_id: node_id.clone(),
            policy,
            hlc,
        };
        let merged = match self.consent.get(&node_id) {
            Some(existing) => existing.merge(&new_entry),
            None => new_entry,
        };
        self.consent.insert(node_id, merged);
    }

    /// Records a revoked certificate entry into the CRL.
    pub fn revoke_certificate(&mut self, entry: RevokedCertificateEntry, hlc: HlcTimestamp) {
        let serial = entry.serial_number.clone();
        let new_entry = CrlCrdtEntry {
            serial_number: serial.clone(),
            entry,
            hlc,
        };
        let merged = match self.crl_entries.get(&serial) {
            Some(existing) => existing.merge(&new_entry),
            None => new_entry,
        };
        self.crl_entries.insert(serial, merged);
    }

    /// Checks if a certificate serial number is revoked in the CRL.
    #[must_use]
    pub fn is_revoked(&self, serial: &str) -> bool {
        self.crl_entries.contains_key(serial)
    }

    /// Returns all currently active nodes.
    #[must_use]
    pub fn active_nodes(&self) -> Vec<&NodeRecord> {
        self.nodes
            .values()
            .filter(|e| e.is_active())
            .map(|e| &e.record)
            .collect()
    }

    /// Returns a node record by NodeId if active.
    #[must_use]
    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeRecord> {
        self.nodes
            .get(node_id)
            .filter(|e| e.is_active())
            .map(|e| &e.record)
    }

    /// Returns hardware capabilities for a node.
    #[must_use]
    pub fn get_capabilities(&self, node_id: &NodeId) -> Option<&HardwareCapabilityDescriptor> {
        self.capabilities.get(node_id).map(|e| &e.descriptor)
    }

    /// Returns resource consent policy for a node.
    #[must_use]
    pub fn get_consent(&self, node_id: &NodeId) -> Option<&ResourceConsentPolicy> {
        self.consent.get(node_id).map(|e| &e.policy)
    }

    /// Merges an incoming delta into this registry state in accordance with CRDT join-semilattice laws.
    pub fn merge_delta(&mut self, delta: &RegistryDelta) -> MergeSummary {
        let mut summary = MergeSummary::default();

        // 1. Merge nodes
        for remote_node in &delta.nodes {
            let node_id = &remote_node.record.node_id;
            match self.nodes.get(node_id) {
                None => {
                    self.nodes.insert(node_id.clone(), remote_node.clone());
                    if remote_node.is_active() {
                        summary.nodes_added += 1;
                    } else {
                        summary.nodes_removed += 1;
                    }
                }
                Some(existing) => {
                    let merged = existing.merge(remote_node);
                    if merged == *existing {
                        summary.ignored_stale += 1;
                    } else {
                        if existing.is_active() && !merged.is_active() {
                            summary.nodes_removed += 1;
                        } else if !existing.is_active() && merged.is_active() {
                            summary.nodes_added += 1;
                        } else {
                            summary.nodes_updated += 1;
                        }
                        self.nodes.insert(node_id.clone(), merged);
                    }
                }
            }
        }

        // 2. Merge capabilities
        for remote_cap in &delta.capabilities {
            let node_id = &remote_cap.node_id;
            match self.capabilities.get(node_id) {
                None => {
                    self.capabilities
                        .insert(node_id.clone(), remote_cap.clone());
                    summary.capabilities_updated += 1;
                }
                Some(existing) => {
                    let merged = existing.merge(remote_cap);
                    if merged == *existing {
                        summary.ignored_stale += 1;
                    } else {
                        self.capabilities.insert(node_id.clone(), merged);
                        summary.capabilities_updated += 1;
                    }
                }
            }
        }

        // 3. Merge consent
        for remote_consent in &delta.consent {
            let node_id = &remote_consent.node_id;
            match self.consent.get(node_id) {
                None => {
                    self.consent.insert(node_id.clone(), remote_consent.clone());
                    summary.consent_updated += 1;
                }
                Some(existing) => {
                    let merged = existing.merge(remote_consent);
                    if merged == *existing {
                        summary.ignored_stale += 1;
                    } else {
                        self.consent.insert(node_id.clone(), merged);
                        summary.consent_updated += 1;
                    }
                }
            }
        }

        // 4. Merge CRL
        for remote_crl in &delta.crl_entries {
            let serial = &remote_crl.serial_number;
            match self.crl_entries.get(serial) {
                None => {
                    self.crl_entries.insert(serial.clone(), remote_crl.clone());
                    summary.crl_updated += 1;
                }
                Some(existing) => {
                    let merged = existing.merge(remote_crl);
                    if merged == *existing {
                        summary.ignored_stale += 1;
                    } else {
                        self.crl_entries.insert(serial.clone(), merged);
                        summary.crl_updated += 1;
                    }
                }
            }
        }

        summary
    }

    /// Merges an entire other `ClusterRegistryState` symmetrically.
    pub fn merge_state(&mut self, other: &Self) -> MergeSummary {
        let dummy_id = NodeId("did:umcf:sync-state-merge".into());
        let delta = other.generate_delta(None, dummy_id, HlcTimestamp::MIN);
        self.merge_delta(&delta)
    }

    /// Generates a causal delta of all entries mutated strictly after `since`.
    /// If `since` is None, returns the entire state as a delta.
    #[must_use]
    pub fn generate_delta(
        &self,
        since: Option<HlcTimestamp>,
        sender_node_id: NodeId,
        delta_hlc: HlcTimestamp,
    ) -> RegistryDelta {
        let nodes = self
            .nodes
            .values()
            .filter(|e| match since {
                None => true,
                Some(s) => e.add_hlc > s || e.remove_hlc.is_some_and(|rem| rem > s),
            })
            .cloned()
            .collect();

        let capabilities = self
            .capabilities
            .values()
            .filter(|e| match since {
                None => true,
                Some(s) => e.hlc > s,
            })
            .cloned()
            .collect();

        let consent = self
            .consent
            .values()
            .filter(|e| match since {
                None => true,
                Some(s) => e.hlc > s,
            })
            .cloned()
            .collect();

        let crl_entries = self
            .crl_entries
            .values()
            .filter(|e| match since {
                None => true,
                Some(s) => e.hlc > s,
            })
            .cloned()
            .collect();

        RegistryDelta {
            cluster_id: self.cluster_id.clone(),
            sender_node_id,
            delta_hlc,
            nodes,
            capabilities,
            consent,
            crl_entries,
        }
    }
}
