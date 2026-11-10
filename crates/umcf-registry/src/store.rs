//! Embedded ACID key-value state store backed by `redb` and synchronized with CRDT registry.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use redb::{Database, Durability, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use umcf_core::capabilities::HardwareCapabilityDescriptor;
use umcf_core::NodeId;
use umcf_enrollment::ResourceConsentPolicy;
use umcf_identity::RevokedCertificateEntry;

use crate::crdt::{
    CapabilityCrdtEntry, ClusterRegistryState, ConsentCrdtEntry, CrlCrdtEntry, MergeSummary,
    NodeCrdtEntry, NodeRecord, NodeStatus, RegistryDelta,
};
use crate::error::RegistryError;
use crate::hlc::{HlcTimestamp, HybridLogicalClock};

// Strongly-typed redb table definitions
pub const NODES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("nodes_v1");
pub const CAPABILITIES_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("capabilities_v1");
pub const CONSENT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("consent_v1");
pub const CERT_CRL_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("cert_crl_v1");
pub const CLUSTER_META_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("cluster_meta_v1");

/// Metadata keys stored in `CLUSTER_META_TABLE`.
pub const META_KEY_CLUSTER_ID: &str = "cluster_id";
pub const META_KEY_NODE_ID: &str = "node_id";
pub const META_KEY_CREATED_AT_MS: &str = "created_at_ms";
pub const META_KEY_LAST_HLC_WALL: &str = "last_hlc_wall";
pub const META_KEY_LAST_HLC_LOGICAL: &str = "last_hlc_logical";

/// Detailed storage usage and telemetry statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub db_path: String,
    pub file_size_bytes: u64,
    pub active_nodes_count: usize,
    pub total_nodes_count: usize,
    pub capabilities_count: usize,
    pub consent_policies_count: usize,
    pub crl_entries_count: usize,
    pub current_hlc: HlcTimestamp,
}

/// Embedded, ACID-compliant local registry store backed by `redb`.
#[derive(Debug, Clone)]
pub struct LocalRegistryStore {
    db: Arc<RwLock<Database>>,
    path: PathBuf,
    cluster_id: String,
    local_node_id: NodeId,
    state: Arc<RwLock<ClusterRegistryState>>,
    hlc: Arc<HybridLogicalClock>,
    durability: Arc<RwLock<Durability>>,
}

impl LocalRegistryStore {
    /// Opens an existing `redb` database or creates a new one, ensuring all tables exist and loading in-memory state.
    pub fn open_or_create(
        path: impl AsRef<Path>,
        cluster_id: &str,
        local_node_id: &NodeId,
    ) -> Result<Self, RegistryError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let db = if path.exists() {
            Database::open(&path)?
        } else {
            Database::create(&path)?
        };

        // Ensure all tables exist in a bootstrapping write transaction
        {
            let write_txn = db.begin_write()?;
            {
                let _ = write_txn.open_table(NODES_TABLE)?;
                let _ = write_txn.open_table(CAPABILITIES_TABLE)?;
                let _ = write_txn.open_table(CONSENT_TABLE)?;
                let _ = write_txn.open_table(CERT_CRL_TABLE)?;
                let mut meta_table = write_txn.open_table(CLUSTER_META_TABLE)?;

                if meta_table.get(META_KEY_CLUSTER_ID)?.is_none() {
                    meta_table.insert(META_KEY_CLUSTER_ID, cluster_id.as_bytes())?;
                    meta_table.insert(META_KEY_NODE_ID, local_node_id.as_str().as_bytes())?;
                    let now_ms = HybridLogicalClock::system_time_ms().to_le_bytes();
                    meta_table.insert(META_KEY_CREATED_AT_MS, now_ms.as_slice())?;
                }
            }
            write_txn.commit()?;
        }

        let mut in_memory_state = ClusterRegistryState::new(cluster_id);
        let mut max_persisted_hlc = HlcTimestamp::MIN;

        // Load persisted entries into in-memory CRDT state
        {
            let read_txn = db.begin_read()?;

            // 1. Nodes table
            {
                let nodes_table = read_txn.open_table(NODES_TABLE)?;
                let iter = nodes_table.iter()?;
                for item in iter {
                    let (k, v) = item?;
                    let node_id = NodeId::parse(k.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "nodes_v1",
                            details: format!("invalid node DID '{}': {e}", k.value()),
                        })?;
                    let entry: NodeCrdtEntry = bincode::deserialize(v.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "nodes_v1",
                            details: format!("failed to deserialize NodeCrdtEntry: {e}"),
                        })?;
                    max_persisted_hlc = max_persisted_hlc.max(entry.add_hlc);
                    if let Some(rem) = entry.remove_hlc {
                        max_persisted_hlc = max_persisted_hlc.max(rem);
                    }
                    in_memory_state.nodes.insert(node_id, entry);
                }
            }

            // 2. Capabilities table
            {
                let caps_table = read_txn.open_table(CAPABILITIES_TABLE)?;
                let iter = caps_table.iter()?;
                for item in iter {
                    let (k, v) = item?;
                    let node_id = NodeId::parse(k.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "capabilities_v1",
                            details: format!("invalid node DID '{}': {e}", k.value()),
                        })?;
                    let entry: CapabilityCrdtEntry = bincode::deserialize(v.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "capabilities_v1",
                            details: format!("failed to deserialize CapabilityCrdtEntry: {e}"),
                        })?;
                    max_persisted_hlc = max_persisted_hlc.max(entry.hlc);
                    in_memory_state.capabilities.insert(node_id, entry);
                }
            }

            // 3. Consent table
            {
                let consent_table = read_txn.open_table(CONSENT_TABLE)?;
                let iter = consent_table.iter()?;
                for item in iter {
                    let (k, v) = item?;
                    let node_id = NodeId::parse(k.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "consent_v1",
                            details: format!("invalid node DID '{}': {e}", k.value()),
                        })?;
                    let entry: ConsentCrdtEntry = bincode::deserialize(v.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "consent_v1",
                            details: format!("failed to deserialize ConsentCrdtEntry: {e}"),
                        })?;
                    max_persisted_hlc = max_persisted_hlc.max(entry.hlc);
                    in_memory_state.consent.insert(node_id, entry);
                }
            }

            // 4. CRL table
            {
                let crl_table = read_txn.open_table(CERT_CRL_TABLE)?;
                let iter = crl_table.iter()?;
                for item in iter {
                    let (k, v) = item?;
                    let serial = k.value().to_string();
                    let entry: CrlCrdtEntry = bincode::deserialize(v.value())
                        .map_err(|e| RegistryError::CorruptedEntry {
                            table: "cert_crl_v1",
                            details: format!("failed to deserialize CrlCrdtEntry: {e}"),
                        })?;
                    max_persisted_hlc = max_persisted_hlc.max(entry.hlc);
                    in_memory_state.crl_entries.insert(serial, entry);
                }
            }
        }

        let hlc = Arc::new(HybridLogicalClock::from_node_id(local_node_id.as_str()));
        hlc.update_floor(max_persisted_hlc);

        Ok(Self {
            db: Arc::new(RwLock::new(db)),
            path,
            cluster_id: cluster_id.to_string(),
            local_node_id: local_node_id.clone(),
            state: Arc::new(RwLock::new(in_memory_state)),
            hlc,
            durability: Arc::new(RwLock::new(Durability::Immediate)),
        })
    }

    /// Sets transaction durability policy (e.g. `Durability::Immediate` or `Durability::Eventual`).
    pub fn set_durability(&self, durability: Durability) {
        let mut d = self.durability.write().unwrap_or_else(|p| p.into_inner());
        *d = durability;
    }

    /// Returns the currently active transaction durability policy.
    #[must_use]
    pub fn durability(&self) -> Durability {
        *self.durability.read().unwrap_or_else(|p| p.into_inner())
    }

    /// Returns cluster identifier.
    #[must_use]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Returns local node DID.
    #[must_use]
    pub fn local_node_id(&self) -> &NodeId {
        &self.local_node_id
    }

    /// Registers or updates a node membership record in an atomic transaction.
    pub fn register_node(&self, record: NodeRecord) -> Result<HlcTimestamp, RegistryError> {
        let ts = self.hlc.tick()?;
        let node_id = record.node_id.clone();
        let new_entry = NodeCrdtEntry {
            record,
            add_hlc: ts,
            remove_hlc: None,
        };

        // Compute merged state first
        let merged_entry = {
            let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
            match state_guard.nodes.get(&node_id) {
                Some(existing) => existing.merge(&new_entry),
                None => new_entry,
            }
        };

        let encoded = bincode::serialize(&merged_entry)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        // Atomic write to redb
        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());
            {
                let mut table = write_txn.open_table(NODES_TABLE)?;
                table.insert(node_id.as_str(), encoded.as_slice())?;
            }
            write_txn.commit()?;
        }

        // Apply to in-memory state
        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());
            state_guard.nodes.insert(node_id, merged_entry);
        }

        Ok(ts)
    }

    /// Tombstones/removes a node membership record in an atomic transaction.
    pub fn evict_node(&self, node_id: &NodeId) -> Result<HlcTimestamp, RegistryError> {
        let ts = self.hlc.tick()?;

        let merged_entry = {
            let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
            match state_guard.nodes.get(node_id) {
                Some(existing) => {
                    let mut cloned = existing.clone();
                    let updated_rem = match cloned.remove_hlc {
                        Some(prev) => prev.max(ts),
                        None => ts,
                    };
                    cloned.remove_hlc = Some(updated_rem);
                    cloned
                }
                None => NodeCrdtEntry {
                    record: NodeRecord {
                        node_id: node_id.clone(),
                        role: umcf_core::NodeRole::Worker,
                        status: NodeStatus::Offline,
                        endpoints: vec![],
                        registered_at_ms: 0,
                        last_heartbeat_ms: 0,
                        hlc: HlcTimestamp::MIN,
                    },
                    add_hlc: HlcTimestamp::MIN,
                    remove_hlc: Some(ts),
                },
            }
        };

        let encoded = bincode::serialize(&merged_entry)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());
            {
                let mut table = write_txn.open_table(NODES_TABLE)?;
                table.insert(node_id.as_str(), encoded.as_slice())?;
            }
            write_txn.commit()?;
        }

        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());
            state_guard.nodes.insert(node_id.clone(), merged_entry);
        }

        Ok(ts)
    }

    /// Updates probed hardware capabilities for a node in an atomic transaction.
    pub fn update_capabilities(
        &self,
        descriptor: HardwareCapabilityDescriptor,
    ) -> Result<HlcTimestamp, RegistryError> {
        let ts = self.hlc.tick()?;
        let node_id = descriptor.node_id.clone();
        let new_entry = CapabilityCrdtEntry {
            node_id: node_id.clone(),
            descriptor,
            hlc: ts,
        };

        let merged_entry = {
            let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
            match state_guard.capabilities.get(&node_id) {
                Some(existing) => existing.merge(&new_entry),
                None => new_entry,
            }
        };

        let encoded = bincode::serialize(&merged_entry)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());
            {
                let mut table = write_txn.open_table(CAPABILITIES_TABLE)?;
                table.insert(node_id.as_str(), encoded.as_slice())?;
            }
            write_txn.commit()?;
        }

        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());
            state_guard.capabilities.insert(node_id, merged_entry);
        }

        Ok(ts)
    }

    /// Updates signed resource consent policy for a node in an atomic transaction.
    pub fn update_consent(
        &self,
        policy: ResourceConsentPolicy,
    ) -> Result<HlcTimestamp, RegistryError> {
        let ts = self.hlc.tick()?;
        let node_id = policy.node_id.clone();
        let new_entry = ConsentCrdtEntry {
            node_id: node_id.clone(),
            policy,
            hlc: ts,
        };

        let merged_entry = {
            let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
            match state_guard.consent.get(&node_id) {
                Some(existing) => existing.merge(&new_entry),
                None => new_entry,
            }
        };

        let encoded = bincode::serialize(&merged_entry)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());
            {
                let mut table = write_txn.open_table(CONSENT_TABLE)?;
                table.insert(node_id.as_str(), encoded.as_slice())?;
            }
            write_txn.commit()?;
        }

        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());
            state_guard.consent.insert(node_id, merged_entry);
        }

        Ok(ts)
    }

    /// Records a revoked certificate entry into the CRL in an atomic transaction.
    pub fn revoke_certificate(
        &self,
        entry: RevokedCertificateEntry,
    ) -> Result<HlcTimestamp, RegistryError> {
        let ts = self.hlc.tick()?;
        let serial = entry.serial_number.clone();
        let new_entry = CrlCrdtEntry {
            serial_number: serial.clone(),
            entry,
            hlc: ts,
        };

        let merged_entry = {
            let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
            match state_guard.crl_entries.get(&serial) {
                Some(existing) => existing.merge(&new_entry),
                None => new_entry,
            }
        };

        let encoded = bincode::serialize(&merged_entry)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());
            {
                let mut table = write_txn.open_table(CERT_CRL_TABLE)?;
                table.insert(serial.as_str(), encoded.as_slice())?;
            }
            write_txn.commit()?;
        }

        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());
            state_guard.crl_entries.insert(serial, merged_entry);
        }

        Ok(ts)
    }

    /// Merges an incoming causal delta, persisting mutated entries atomically into `redb`.
    pub fn apply_delta(&self, delta: &RegistryDelta) -> Result<MergeSummary, RegistryError> {
        let _ = self.hlc.observe(&delta.delta_hlc);

        let mut summary = MergeSummary::default();
        let mut nodes_to_persist = Vec::new();
        let mut caps_to_persist = Vec::new();
        let mut consent_to_persist = Vec::new();
        let mut crl_to_persist = Vec::new();

        {
            let mut state_guard = self.state.write().unwrap_or_else(|p| p.into_inner());

            // 1. Nodes
            for remote_node in &delta.nodes {
                let node_id = &remote_node.record.node_id;
                match state_guard.nodes.get(node_id) {
                    None => {
                        state_guard
                            .nodes
                            .insert(node_id.clone(), remote_node.clone());
                        if remote_node.is_active() {
                            summary.nodes_added += 1;
                        } else {
                            summary.nodes_removed += 1;
                        }
                        nodes_to_persist.push(remote_node.clone());
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
                            state_guard.nodes.insert(node_id.clone(), merged.clone());
                            nodes_to_persist.push(merged);
                        }
                    }
                }
            }

            // 2. Capabilities
            for remote_cap in &delta.capabilities {
                let node_id = &remote_cap.node_id;
                match state_guard.capabilities.get(node_id) {
                    None => {
                        state_guard
                            .capabilities
                            .insert(node_id.clone(), remote_cap.clone());
                        summary.capabilities_updated += 1;
                        caps_to_persist.push(remote_cap.clone());
                    }
                    Some(existing) => {
                        let merged = existing.merge(remote_cap);
                        if merged == *existing {
                            summary.ignored_stale += 1;
                        } else {
                            state_guard
                                .capabilities
                                .insert(node_id.clone(), merged.clone());
                            summary.capabilities_updated += 1;
                            caps_to_persist.push(merged);
                        }
                    }
                }
            }

            // 3. Consent
            for remote_consent in &delta.consent {
                let node_id = &remote_consent.node_id;
                match state_guard.consent.get(node_id) {
                    None => {
                        state_guard
                            .consent
                            .insert(node_id.clone(), remote_consent.clone());
                        summary.consent_updated += 1;
                        consent_to_persist.push(remote_consent.clone());
                    }
                    Some(existing) => {
                        let merged = existing.merge(remote_consent);
                        if merged == *existing {
                            summary.ignored_stale += 1;
                        } else {
                            state_guard
                                .consent
                                .insert(node_id.clone(), merged.clone());
                            summary.consent_updated += 1;
                            consent_to_persist.push(merged);
                        }
                    }
                }
            }

            // 4. CRL
            for remote_crl in &delta.crl_entries {
                let serial = &remote_crl.serial_number;
                match state_guard.crl_entries.get(serial) {
                    None => {
                        state_guard
                            .crl_entries
                            .insert(serial.clone(), remote_crl.clone());
                        summary.crl_updated += 1;
                        crl_to_persist.push(remote_crl.clone());
                    }
                    Some(existing) => {
                        let merged = existing.merge(remote_crl);
                        if merged == *existing {
                            summary.ignored_stale += 1;
                        } else {
                            state_guard
                                .crl_entries
                                .insert(serial.clone(), merged.clone());
                            summary.crl_updated += 1;
                            crl_to_persist.push(merged);
                        }
                    }
                }
            }
        }

        // Persist all mutations to disk in a single atomic transaction
        if !nodes_to_persist.is_empty()
            || !caps_to_persist.is_empty()
            || !consent_to_persist.is_empty()
            || !crl_to_persist.is_empty()
        {
            let db_guard = self.db.read().unwrap_or_else(|p| p.into_inner());
            let mut write_txn = db_guard.begin_write()?;
            write_txn.set_durability(self.durability());

            if !nodes_to_persist.is_empty() {
                let mut table = write_txn.open_table(NODES_TABLE)?;
                for entry in nodes_to_persist {
                    let encoded = bincode::serialize(&entry)
                        .map_err(|e| RegistryError::Serialization(e.to_string()))?;
                    table.insert(entry.record.node_id.as_str(), encoded.as_slice())?;
                }
            }

            if !caps_to_persist.is_empty() {
                let mut table = write_txn.open_table(CAPABILITIES_TABLE)?;
                for entry in caps_to_persist {
                    let encoded = bincode::serialize(&entry)
                        .map_err(|e| RegistryError::Serialization(e.to_string()))?;
                    table.insert(entry.node_id.as_str(), encoded.as_slice())?;
                }
            }

            if !consent_to_persist.is_empty() {
                let mut table = write_txn.open_table(CONSENT_TABLE)?;
                for entry in consent_to_persist {
                    let encoded = bincode::serialize(&entry)
                        .map_err(|e| RegistryError::Serialization(e.to_string()))?;
                    table.insert(entry.node_id.as_str(), encoded.as_slice())?;
                }
            }

            if !crl_to_persist.is_empty() {
                let mut table = write_txn.open_table(CERT_CRL_TABLE)?;
                for entry in crl_to_persist {
                    let encoded = bincode::serialize(&entry)
                        .map_err(|e| RegistryError::Serialization(e.to_string()))?;
                    table.insert(entry.serial_number.as_str(), encoded.as_slice())?;
                }
            }

            write_txn.commit()?;
        }

        Ok(summary)
    }

    /// Queries a node record from in-memory CRDT state.
    #[must_use]
    pub fn get_node(&self, node_id: &NodeId) -> Option<NodeRecord> {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.get_node(node_id).cloned()
    }

    /// Queries all active nodes from in-memory CRDT state.
    #[must_use]
    pub fn list_active_nodes(&self) -> Vec<NodeRecord> {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.active_nodes().into_iter().cloned().collect()
    }

    /// Queries hardware capability descriptor for a node.
    #[must_use]
    pub fn get_capabilities(&self, node_id: &NodeId) -> Option<HardwareCapabilityDescriptor> {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.get_capabilities(node_id).cloned()
    }

    /// Queries resource consent policy for a node.
    #[must_use]
    pub fn get_consent(&self, node_id: &NodeId) -> Option<ResourceConsentPolicy> {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.get_consent(node_id).cloned()
    }

    /// Checks if a certificate serial number is revoked in the CRL.
    #[must_use]
    pub fn is_certificate_revoked(&self, serial: &str) -> bool {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.is_revoked(serial)
    }

    /// Generates a causal delta for synchronizing with peers.
    pub fn generate_delta(&self, since: Option<HlcTimestamp>) -> Result<RegistryDelta, RegistryError> {
        let ts = self.hlc.tick()?;
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        Ok(state_guard.generate_delta(since, self.local_node_id.clone(), ts))
    }

    /// Returns a full snapshot of the cluster registry state.
    #[must_use]
    pub fn cluster_state(&self) -> ClusterRegistryState {
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        state_guard.clone()
    }

    /// Returns a clone of the inner thread-safe database handle (primarily for simulation and testing).
    #[must_use]
    pub fn raw_database(&self) -> Arc<RwLock<Database>> {
        Arc::clone(&self.db)
    }

    /// Compacts the underlying `redb` database file, reclaiming unused pages.
    pub fn compact(&self) -> Result<bool, RegistryError> {
        let mut db_guard = self.db.write().unwrap_or_else(|p| p.into_inner());
        Ok(db_guard.compact()?)
    }

    /// Verifies database file integrity.
    pub fn check_integrity(&self) -> Result<bool, RegistryError> {
        let mut db_guard = self.db.write().unwrap_or_else(|p| p.into_inner());
        Ok(db_guard.check_integrity()?)
    }

    /// Computes store statistics including file size, entity counts, and current HLC.
    pub fn stats(&self) -> Result<StoreStats, RegistryError> {
        let file_size_bytes = fs::metadata(&self.path)?.len();
        let state_guard = self.state.read().unwrap_or_else(|p| p.into_inner());
        let active_nodes_count = state_guard.active_nodes().len();
        let total_nodes_count = state_guard.nodes.len();
        let capabilities_count = state_guard.capabilities.len();
        let consent_policies_count = state_guard.consent.len();
        let crl_entries_count = state_guard.crl_entries.len();
        let current_hlc = self.hlc.now();

        Ok(StoreStats {
            db_path: self.path.to_string_lossy().to_string(),
            file_size_bytes,
            active_nodes_count,
            total_nodes_count,
            capabilities_count,
            consent_policies_count,
            crl_entries_count,
            current_hlc,
        })
    }
}
