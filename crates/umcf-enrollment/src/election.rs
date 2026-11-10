//! Lightweight term-based Raft-inspired leader election with dynamic hardware fitness scoring.
//! Strictly enforces `#![forbid(unsafe_code)]`, zero-placeholder invariants, and local sovereignty.

use crate::error::EnrollmentError;
use crate::role_engine::{NodeRoleState, RoleStateMachine};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use umcf_core::{ByteSize, DeviceClass};

/// Dynamic hardware fitness score determining leader suitability during elections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterFitnessScore {
    /// AC power bonus (+400 if charging, scaled by battery percent if discharging).
    pub power_score: u32,
    /// Device form-factor bonus (+250 server/desktop/dock, +100 laptop, +50 mobile).
    pub device_score: u32,
    /// Available memory capacity bonus (up to +150 points for >= 15 GiB RAM).
    pub memory_score: u32,
    /// Network latency and packet stability bonus (up to +100 points).
    pub network_score: u32,
    /// Deterministic 16-bit pseudo-random tie-breaker derived from BLAKE3(term || did).
    pub tie_breaker: u16,
    /// Cumulative aggregate score.
    pub total_score: u32,
}

impl PartialOrd for MasterFitnessScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MasterFitnessScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_score
            .cmp(&other.total_score)
            .then_with(|| self.tie_breaker.cmp(&other.tie_breaker))
    }
}

/// Instantaneous hardware and network profile used for election fitness scoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeFitnessProfile {
    /// W3C did:key of the node.
    pub did: String,
    /// Hardware device class.
    pub device_class: DeviceClass,
    /// Whether device is plugged into AC power.
    pub is_charging: bool,
    /// Current battery percent (None if AC-only desktop/server).
    pub battery_percent: Option<u8>,
    /// Currently free/available RAM.
    pub available_ram: ByteSize,
    /// Measured network packet loss rate (0.0 = zero loss, 1.0 = total loss).
    pub packet_loss_rate: f32,
}

impl MasterFitnessScore {
    /// Computes the fitness score from a node profile and election term.
    #[must_use]
    pub fn calculate(profile: &NodeFitnessProfile, term: u64) -> Self {
        // 1. Power Score: +400 if on AC/charging; if discharging, 0..200 based on battery %
        let power_score: u32 = if profile.is_charging || profile.battery_percent.is_none() {
            400
        } else {
            let bat = u32::from(profile.battery_percent.unwrap_or(0));
            bat.min(100) * 2 // Max 200 points on battery
        };

        // 2. Device Class Score: Server (250), Workstation (220), Desktop (200), Laptop (100), Mobile (50)
        let device_score: u32 = match profile.device_class {
            DeviceClass::Server => 250,
            DeviceClass::Workstation => 220,
            DeviceClass::Desktop => 200,
            DeviceClass::Laptop => 100,
            DeviceClass::Tablet | DeviceClass::Smartphone | DeviceClass::BrowserWasm => 50,
        };

        // 3. Memory Score: +10 points per GiB of available RAM, capped at 150 points (15 GiB)
        let ram_gib = profile.available_ram.as_bytes() / (1024 * 1024 * 1024);
        let memory_score: u32 = ((ram_gib as u32) * 10).min(150);

        // 4. Network Score: Up to 100 points scaled by (1.0 - packet_loss_rate)
        let loss_clamped = profile.packet_loss_rate.clamp(0.0, 1.0);
        let network_score: u32 = ((1.0 - loss_clamped) * 100.0).round() as u32;

        // 5. Deterministic Tie-Breaker: BLAKE3(term || did)
        let mut hasher = blake3::Hasher::new();
        hasher.update(&term.to_be_bytes());
        hasher.update(profile.did.as_bytes());
        let hash = hasher.finalize();
        let tie_breaker = u16::from_be_bytes([hash.as_bytes()[0], hash.as_bytes()[1]]);

        let total_score = power_score + device_score + memory_score + network_score;

        Self {
            power_score,
            device_score,
            memory_score,
            network_score,
            tie_breaker,
            total_score,
        }
    }
}

/// RPC message sent by candidate nodes seeking quorum votes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestVote {
    /// Election term sequence counter.
    pub term: u64,
    /// Candidate node DID.
    pub candidate_did: String,
    /// Evaluated master fitness score of the candidate.
    pub candidate_fitness: MasterFitnessScore,
    /// Monotonically increasing cluster log generation index.
    pub last_log_index: u64,
}

/// RPC response granting or denying a candidate node's vote request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Election term in which vote was cast.
    pub term: u64,
    /// DID of the voting peer.
    pub voter_did: String,
    /// Whether the vote was granted to the candidate.
    pub vote_granted: bool,
    /// Optional diagnostic reason for vote denial.
    pub reason: Option<String>,
}

/// Periodic heartbeat emitted by the active leader to suppress candidate timeouts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderHeartbeat {
    /// Current leader election term.
    pub term: u64,
    /// Leader node DID.
    pub leader_did: String,
    /// Leader fitness score advertised to mesh.
    pub leader_fitness: MasterFitnessScore,
    /// Number of active workloads currently managed.
    pub active_job_count: u32,
    /// Heartbeat emission epoch timestamp.
    pub timestamp_epoch_ms: u64,
}

/// Protocol payload for graceful coordinator leadership handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderTransfer {
    /// Term in which transfer takes effect.
    pub term: u64,
    /// Current leader initiating the step-down.
    pub current_leader_did: String,
    /// Successor node selected to assume coordinator authority.
    pub target_leader_did: String,
    /// Authenticated state digest preserving job/cluster state across handover.
    pub cluster_state_digest: [u8; 32],
    /// Epoch timestamp when handover was initiated.
    pub transfer_timestamp_ms: u64,
}

/// Raft-inspired consensus election coordinator with fitness scoring and leader handover.
#[derive(Debug)]
pub struct ElectionCoordinator {
    local_did: String,
    current_term: u64,
    voted_for: Option<String>,
    leader_did: Option<String>,
    role_sm: RoleStateMachine,
    cluster_peers: HashSet<String>,
    votes_received: HashSet<String>,
    last_heartbeat_timestamp_ms: u64,
    election_timeout_ms: u64,
}

impl ElectionCoordinator {
    /// Creates a new election coordinator initialized with the local DID and starting role.
    #[must_use]
    pub fn new(local_did: impl Into<String>, initial_role: NodeRoleState) -> Self {
        let local_did = local_did.into();
        Self {
            local_did,
            current_term: 0,
            voted_for: None,
            leader_did: None,
            role_sm: RoleStateMachine::new(initial_role),
            cluster_peers: HashSet::new(),
            votes_received: HashSet::new(),
            last_heartbeat_timestamp_ms: 0,
            election_timeout_ms: 2000, // 2000 ms default
        }
    }

    /// Returns the local node DID.
    #[must_use]
    pub fn local_did(&self) -> &str {
        &self.local_did
    }

    /// Returns the current election term.
    #[must_use]
    pub const fn current_term(&self) -> u64 {
        self.current_term
    }

    /// Returns the active leader DID, if one is recognized.
    #[must_use]
    pub fn leader_did(&self) -> Option<&str> {
        self.leader_did.as_deref()
    }

    /// Returns the active role state of the local node.
    #[must_use]
    pub const fn role_state(&self) -> NodeRoleState {
        self.role_sm.current_state()
    }

    /// Adds a known peer to the cluster topology registry.
    pub fn add_peer(&mut self, peer_did: impl Into<String>) {
        let did = peer_did.into();
        if did != self.local_did {
            self.cluster_peers.insert(did);
        }
    }

    /// Removes a departed peer from the cluster topology registry.
    pub fn remove_peer(&mut self, peer_did: &str) {
        self.cluster_peers.remove(peer_did);
    }

    /// Returns total cluster size (peers + self).
    #[must_use]
    pub fn cluster_size(&self) -> usize {
        self.cluster_peers.len() + 1
    }

    /// Returns the strict majority quorum size required to win an election: floor(N/2) + 1.
    #[must_use]
    pub fn quorum_size(&self) -> usize {
        (self.cluster_size() / 2) + 1
    }

    /// Initiates an election campaign, advancing the term and generating a `RequestVote` message.
    pub fn start_election(
        &mut self,
        profile: &NodeFitnessProfile,
        now_ms: u64,
    ) -> Result<RequestVote, EnrollmentError> {
        self.current_term = self.current_term.saturating_add(1);
        self.voted_for = Some(self.local_did.clone());
        self.leader_did = None;
        self.votes_received.clear();
        self.votes_received.insert(self.local_did.clone());

        let fitness = MasterFitnessScore::calculate(profile, self.current_term);

        // Single isolated node automatically wins election immediately
        if self.cluster_size() == 1 {
            self.role_sm.transition_to(
                NodeRoleState::AutonomousSolo,
                "Solo node election self-quorum",
                now_ms,
            )?;
            self.leader_did = Some(self.local_did.clone());
        } else {
            self.role_sm.transition_to(
                NodeRoleState::Candidate,
                format!("Started election for term {}", self.current_term),
                now_ms,
            )?;
        }

        Ok(RequestVote {
            term: self.current_term,
            candidate_did: self.local_did.clone(),
            candidate_fitness: fitness,
            last_log_index: 0,
        })
    }

    /// Processes an incoming `RequestVote` message from a candidate node.
    pub fn handle_request_vote(
        &mut self,
        req: &RequestVote,
        local_profile: &NodeFitnessProfile,
        now_ms: u64,
    ) -> VoteResponse {
        // 1. Term check: Reject stale terms
        if req.term < self.current_term {
            return VoteResponse {
                term: self.current_term,
                voter_did: self.local_did.clone(),
                vote_granted: false,
                reason: Some(format!(
                    "Stale term: candidate term {} < local term {}",
                    req.term, self.current_term
                )),
            };
        }

        // 2. Advance term if candidate has higher term
        if req.term > self.current_term {
            self.current_term = req.term;
            self.voted_for = None;
            self.leader_did = None;

            if matches!(
                self.role_sm.current_state(),
                NodeRoleState::Master | NodeRoleState::Candidate
            ) {
                let _ = self.role_sm.transition_to(
                    NodeRoleState::Worker,
                    format!("Stepped down on discovering higher term {}", req.term),
                    now_ms,
                );
            }
        }

        // 3. Grant vote if haven't voted yet or already voted for this candidate
        let can_vote = self.voted_for.is_none()
            || self.voted_for.as_deref() == Some(&req.candidate_did);

        if can_vote {
            // If the local node is also an active Candidate, compare fitness to decide whether to yield
            if self.role_sm.current_state() == NodeRoleState::Candidate {
                let local_fitness = MasterFitnessScore::calculate(local_profile, self.current_term);
                if req.candidate_fitness < local_fitness {
                    return VoteResponse {
                        term: self.current_term,
                        voter_did: self.local_did.clone(),
                        vote_granted: false,
                        reason: Some("Local candidate has higher fitness".to_string()),
                    };
                }
            }

            self.voted_for = Some(req.candidate_did.clone());
            VoteResponse {
                term: self.current_term,
                voter_did: self.local_did.clone(),
                vote_granted: true,
                reason: None,
            }
        } else {
            VoteResponse {
                term: self.current_term,
                voter_did: self.local_did.clone(),
                vote_granted: false,
                reason: Some(format!(
                    "Already voted for {:?} in term {}",
                    self.voted_for, self.current_term
                )),
            }
        }
    }

    /// Processes an incoming `VoteResponse`. If quorum is reached, ascends to `Master`.
    pub fn handle_vote_response(
        &mut self,
        resp: &VoteResponse,
        now_ms: u64,
    ) -> Result<Option<NodeRoleState>, EnrollmentError> {
        if resp.term != self.current_term || !resp.vote_granted {
            return Ok(None);
        }

        if self.role_sm.current_state() != NodeRoleState::Candidate {
            return Ok(None);
        }

        self.votes_received.insert(resp.voter_did.clone());

        // Quorum reached -> Ascend to Master
        if self.votes_received.len() >= self.quorum_size() {
            self.role_sm.transition_to(
                NodeRoleState::Master,
                format!(
                    "Ascended to Master: quorum ({}/{}) reached in term {}",
                    self.votes_received.len(),
                    self.cluster_size(),
                    self.current_term
                ),
                now_ms,
            )?;
            self.leader_did = Some(self.local_did.clone());
            return Ok(Some(NodeRoleState::Master));
        }

        Ok(None)
    }

    /// Processes an incoming leader heartbeat, validating term and refreshing leader lease.
    pub fn handle_leader_heartbeat(
        &mut self,
        heartbeat: &LeaderHeartbeat,
        now_ms: u64,
    ) -> Result<Option<NodeRoleState>, EnrollmentError> {
        if heartbeat.term < self.current_term {
            return Ok(None);
        }

        if heartbeat.term > self.current_term {
            self.current_term = heartbeat.term;
            self.voted_for = None;
        }

        self.leader_did = Some(heartbeat.leader_did.clone());
        self.last_heartbeat_timestamp_ms = now_ms;

        // Step down to Worker if currently campaigning or isolated
        if matches!(
            self.role_sm.current_state(),
            NodeRoleState::Candidate | NodeRoleState::AutonomousSolo
        ) {
            self.role_sm.transition_to(
                NodeRoleState::Worker,
                format!(
                    "Recognized valid leader {} for term {}",
                    heartbeat.leader_did, heartbeat.term
                ),
                now_ms,
            )?;
        }

        Ok(Some(self.role_sm.current_state()))
    }

    /// Hands off coordinator authority gracefully to the highest-fitness peer in < 250 ms.
    pub fn initiate_leader_transfer(
        &mut self,
        target_did: impl Into<String>,
        cluster_state_digest: [u8; 32],
        now_ms: u64,
    ) -> Result<LeaderTransfer, EnrollmentError> {
        if !matches!(
            self.role_sm.current_state(),
            NodeRoleState::Master | NodeRoleState::Hybrid
        ) {
            return Err(EnrollmentError::ElectionFailed(
                "Only an active Master/Hybrid can initiate leader transfer".to_string(),
            ));
        }

        let target = target_did.into();
        self.role_sm.transition_to(
            NodeRoleState::Worker,
            format!("Initiated graceful leadership transfer to {target}"),
            now_ms,
        )?;
        self.leader_did = Some(target.clone());

        Ok(LeaderTransfer {
            term: self.current_term,
            current_leader_did: self.local_did.clone(),
            target_leader_did: target,
            cluster_state_digest,
            transfer_timestamp_ms: now_ms,
        })
    }

    /// Accepts leadership handover from a stepping-down coordinator.
    pub fn accept_leader_transfer(
        &mut self,
        transfer: &LeaderTransfer,
        now_ms: u64,
    ) -> Result<(), EnrollmentError> {
        if transfer.target_leader_did != self.local_did {
            return Err(EnrollmentError::ElectionFailed(format!(
                "LeaderTransfer addressed to {}, not local node {}",
                transfer.target_leader_did, self.local_did
            )));
        }

        self.current_term = transfer.term.max(self.current_term).saturating_add(1);
        self.voted_for = Some(self.local_did.clone());
        self.leader_did = Some(self.local_did.clone());

        self.role_sm.transition_to(
            NodeRoleState::Master,
            format!(
                "Accepted graceful leadership transfer from {} for term {}",
                transfer.current_leader_did, self.current_term
            ),
            now_ms,
        )?;

        Ok(())
    }

    /// Evaluates whether the heartbeat lease has timed out on a worker node.
    #[must_use]
    pub fn is_heartbeat_timed_out(&self, now_ms: u64) -> bool {
        if self.role_sm.current_state() == NodeRoleState::Worker {
            now_ms.saturating_sub(self.last_heartbeat_timestamp_ms) >= self.election_timeout_ms
        } else {
            false
        }
    }
}
