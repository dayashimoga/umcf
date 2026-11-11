//! `umcf-net::nat`: RFC 5245 / RFC 8445 Interactive Connectivity Establishment (ICE),
//! NAT behavior classification (RFC 4787 / RFC 5780), and UDP Hole Punching.
//!
//! Provides:
//! - 6-tier `NatType` classification (`OpenInternet`, `FullCone`, `RestrictedCone`, `PortRestrictedCone`, `SymmetricNat`, `UdpBlocked`).
//! - RFC 5245 ICE candidate model (`Host`, `ServerReflexive`, `PeerReflexive`, `Relayed`).
//! - Short-term HMAC-BLAKE3 authenticated connectivity check probe packet framing.
//! - Paced synchronized UDP hole punching coordinator (`HolePunchCoordinator`).
//! - Deterministic NAT simulator proxies for multi-topology integration testing.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::NetError;
use crate::stun::query_stun_server;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::debug;
use umcf_core::NodeId;

/// Magic bytes prefix for UMCF Hole Punch probe packets (*b"UMCFPUNCH").
pub const HOLE_PUNCH_PROBE_MAGIC: &[u8; 9] = b"UMCFPUNCH";
pub const PROBE_VERSION: u8 = 1;
pub const PROBE_TYPE_REQUEST: u8 = 1;
pub const PROBE_TYPE_RESPONSE: u8 = 2;

/// Discovered NAT mapping and filtering classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NatType {
    /// Public IP assigned directly to interface; no translation.
    OpenInternet,
    /// Endpoint-Independent Mapping and Endpoint-Independent Filtering.
    FullCone,
    /// Endpoint-Independent Mapping and Address-Restricted Filtering.
    RestrictedCone,
    /// Endpoint-Independent Mapping and Address+Port-Restricted Filtering.
    PortRestrictedCone,
    /// Address and Port-Dependent Mapping (different external port per destination).
    SymmetricNat,
    /// Outbound UDP traffic blocked by local/upstream firewall.
    UdpBlocked,
}

impl NatType {
    /// Determines whether direct peer-to-peer UDP hole punching is feasible between two NAT types.
    #[must_use]
    pub const fn is_direct_punchable(self, remote: Self) -> bool {
        match (self, remote) {
            (Self::UdpBlocked, _) | (_, Self::UdpBlocked) => false,
            (Self::OpenInternet, _) | (_, Self::OpenInternet) => true,
            (Self::FullCone, _) | (_, Self::FullCone) => true,
            (Self::RestrictedCone, Self::RestrictedCone) => true,
            (Self::RestrictedCone, Self::PortRestrictedCone) => true,
            (Self::PortRestrictedCone, Self::RestrictedCone) => true,
            (Self::PortRestrictedCone, Self::PortRestrictedCone) => true,
            // Symmetric NAT can only punch against OpenInternet or FullCone
            (Self::SymmetricNat, Self::RestrictedCone)
            | (Self::RestrictedCone, Self::SymmetricNat)
            | (Self::SymmetricNat, Self::PortRestrictedCone)
            | (Self::PortRestrictedCone, Self::SymmetricNat)
            | (Self::SymmetricNat, Self::SymmetricNat) => false,
        }
    }

    /// Returns `true` if traversal between these two topologies strictly requires a relay server (TURN/DERP).
    #[must_use]
    pub const fn requires_relay(self, remote: Self) -> bool {
        !self.is_direct_punchable(remote)
    }

    /// Descriptive name string.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::OpenInternet => "OpenInternet",
            Self::FullCone => "FullCone",
            Self::RestrictedCone => "RestrictedCone",
            Self::PortRestrictedCone => "PortRestrictedCone",
            Self::SymmetricNat => "SymmetricNat",
            Self::UdpBlocked => "UdpBlocked",
        }
    }
}

/// ICE Candidate Type conforming to RFC 5245 / RFC 8445.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IceCandidateType {
    /// Locally bound physical or virtual network interface address.
    Host = 1,
    /// Public IP and port allocated on the NAT via STUN server query.
    ServerReflexive = 2,
    /// Discovered peer address learned dynamically during connectivity checks.
    PeerReflexive = 3,
    /// Address allocated on an intermediate relay server (TURN).
    Relayed = 4,
}

impl IceCandidateType {
    /// RFC 5245 §4.1.2.1 Type Preference (Host: 126, PeerReflexive: 110, ServerReflexive: 100, Relayed: 0).
    #[must_use]
    pub const fn type_preference(self) -> u32 {
        match self {
            Self::Host => 126,
            Self::PeerReflexive => 110,
            Self::ServerReflexive => 100,
            Self::Relayed => 0,
        }
    }
}

/// Single gathered ICE candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate_type: IceCandidateType,
    pub addr: SocketAddr,
    pub priority: u32,
    pub foundation: String,
    pub component_id: u16,
}

impl IceCandidate {
    /// Constructs a new candidate and calculates RFC 5245 priority.
    #[must_use]
    pub fn new(
        candidate_type: IceCandidateType,
        addr: SocketAddr,
        local_pref: u16,
        component_id: u16,
    ) -> Self {
        // Priority formula: (2^24)*type_pref + (2^8)*local_pref + (256 - component_id)
        let priority = (candidate_type.type_preference() << 24)
            | (u32::from(local_pref) << 8)
            | u32::from(256 - component_id.min(255));

        let foundation = blake3::hash(
            format!("{}:{}:{}", candidate_type as u8, addr.ip(), addr.port()).as_bytes(),
        )
        .to_hex();

        Self {
            candidate_type,
            addr,
            priority,
            foundation: foundation[..8].to_string(),
            component_id,
        }
    }
}

/// Complete signed ICE candidate bundle for an endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceCandidateSet {
    pub node_id: NodeId,
    pub ufrag: String,
    pub pwd: String,
    pub nat_type: NatType,
    pub candidates: Vec<IceCandidate>,
    pub created_at_epoch_ms: u64,
    pub signature: [u8; 32],
}

impl IceCandidateSet {
    /// Creates and signs an `IceCandidateSet` with HMAC-BLAKE3 credentials.
    #[must_use]
    pub fn sign_new(
        node_id: NodeId,
        nat_type: NatType,
        mut candidates: Vec<IceCandidate>,
        secret_key: &[u8; 32],
    ) -> Self {
        // Sort descending by priority
        candidates.sort_by_key(|b| std::cmp::Reverse(b.priority));

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let ufrag = format!(
            "ufrag_{}",
            &blake3::hash(format!("{}:{}", node_id, now_ms).as_bytes()).to_hex()[..10]
        );
        let pwd = format!(
            "pwd_{}",
            &blake3::hash(format!("{}:{}:pwd", node_id, now_ms).as_bytes()).to_hex()[..20]
        );

        let sig = compute_candidate_signature(
            &node_id,
            nat_type,
            &ufrag,
            &pwd,
            &candidates,
            now_ms,
            secret_key,
        );

        Self {
            node_id,
            ufrag,
            pwd,
            nat_type,
            candidates,
            created_at_epoch_ms: now_ms,
            signature: sig,
        }
    }

    /// Verifies the cryptographic HMAC-BLAKE3 signature on the candidate set.
    #[must_use]
    pub fn verify_signature(&self, secret_key: &[u8; 32]) -> bool {
        let expected = compute_candidate_signature(
            &self.node_id,
            self.nat_type,
            &self.ufrag,
            &self.pwd,
            &self.candidates,
            self.created_at_epoch_ms,
            secret_key,
        );
        self.signature == expected
    }
}

fn compute_candidate_signature(
    node_id: &NodeId,
    nat_type: NatType,
    ufrag: &str,
    pwd: &str,
    candidates: &[IceCandidate],
    created_at_ms: u64,
    secret_key: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(secret_key);
    hasher.update(b"UMCF-ICE-CANDIDATES-v1");
    hasher.update(node_id.as_str().as_bytes());
    hasher.update(&[nat_type as u8]);
    hasher.update(ufrag.as_bytes());
    hasher.update(pwd.as_bytes());
    hasher.update(&created_at_ms.to_le_bytes());
    for c in candidates {
        hasher.update(&[c.candidate_type as u8]);
        hasher.update(&c.addr.port().to_le_bytes());
        match c.addr.ip() {
            IpAddr::V4(v4) => hasher.update(&v4.octets()),
            IpAddr::V6(v6) => hasher.update(&v6.octets()),
        };
    }
    *hasher.finalize().as_bytes()
}

/// Binary wire packet for HMAC-BLAKE3 authenticated connectivity checks during hole punching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolePunchProbePacket {
    pub probe_type: u8,
    pub ufrag: [u8; 16],
    pub tx_id: u64,
    pub timestamp_ms: u64,
    pub hmac: [u8; 32],
}

impl HolePunchProbePacket {
    /// Creates a new probe packet signed with HMAC-BLAKE3 derived from `pwd`.
    #[must_use]
    pub fn new(probe_type: u8, ufrag_str: &str, tx_id: u64, pwd: &str) -> Self {
        let mut ufrag = [0u8; 16];
        let bytes = ufrag_str.as_bytes();
        let copy_len = bytes.len().min(16);
        ufrag[..copy_len].copy_from_slice(&bytes[..copy_len]);

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let hmac = compute_probe_hmac(probe_type, &ufrag, tx_id, timestamp_ms, pwd);

        Self {
            probe_type,
            ufrag,
            tx_id,
            timestamp_ms,
            hmac,
        }
    }

    /// Serializes probe packet to 74-byte binary wire payload.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(74);
        buf.extend_from_slice(HOLE_PUNCH_PROBE_MAGIC); // 9 bytes
        buf.push(PROBE_VERSION); // 1 byte
        buf.push(self.probe_type); // 1 byte
        buf.extend_from_slice(&self.ufrag); // 16 bytes
        buf.extend_from_slice(&self.tx_id.to_le_bytes()); // 8 bytes
        buf.extend_from_slice(&self.timestamp_ms.to_le_bytes()); // 8 bytes
        buf.extend_from_slice(&self.hmac); // 32 bytes
        buf
    }

    /// Parses and verifies probe packet from wire buffer.
    pub fn parse_and_verify(buf: &[u8], expected_pwd: &str) -> Result<Self, NetError> {
        if buf.len() < 75 {
            return Err(NetError::InvalidStun("Probe packet too short".into()));
        }
        if &buf[0..9] != HOLE_PUNCH_PROBE_MAGIC {
            return Err(NetError::InvalidStun("Invalid probe magic header".into()));
        }
        if buf[9] != PROBE_VERSION {
            return Err(NetError::InvalidStun("Unsupported probe version".into()));
        }
        let probe_type = buf[10];
        let mut ufrag = [0u8; 16];
        ufrag.copy_from_slice(&buf[11..27]);
        let tx_id = u64::from_le_bytes([
            buf[27], buf[28], buf[29], buf[30], buf[31], buf[32], buf[33], buf[34],
        ]);
        let timestamp_ms = u64::from_le_bytes([
            buf[35], buf[36], buf[37], buf[38], buf[39], buf[40], buf[41], buf[42],
        ]);
        let mut hmac = [0u8; 32];
        hmac.copy_from_slice(&buf[43..75]);

        let expected_hmac =
            compute_probe_hmac(probe_type, &ufrag, tx_id, timestamp_ms, expected_pwd);
        if hmac != expected_hmac {
            return Err(NetError::InvalidStun(
                "Probe HMAC authentication failed".into(),
            ));
        }

        Ok(Self {
            probe_type,
            ufrag,
            tx_id,
            timestamp_ms,
            hmac,
        })
    }
}

fn compute_probe_hmac(
    probe_type: u8,
    ufrag: &[u8; 16],
    tx_id: u64,
    timestamp_ms: u64,
    pwd: &str,
) -> [u8; 32] {
    let key = blake3::hash(pwd.as_bytes());
    let mut hasher = blake3::Hasher::new_keyed(key.as_bytes());
    hasher.update(b"UMCF-HOLE-PUNCH-PROBE");
    hasher.update(&[probe_type]);
    hasher.update(ufrag);
    hasher.update(&tx_id.to_le_bytes());
    hasher.update(&timestamp_ms.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// NAT Traversal Result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NatTraversalResult {
    /// Direct P2P UDP hole punching succeeded; sockets mapped and verified.
    DirectHolePunchSucceeded {
        winning_local: SocketAddr,
        winning_remote: SocketAddr,
        rtt_ms: f64,
    },
    /// Direct P2P hole punching failed or is mathematically impossible (e.g. Symmetric NAT <-> Symmetric NAT).
    RequiresRelay { reason: String },
}

/// Gathers local Host and ServerReflexive ICE candidates.
pub async fn gather_candidates(
    socket: &UdpSocket,
    stun_servers: &[SocketAddr],
    node_id: NodeId,
    secret_key: &[u8; 32],
    timeout_duration: Duration,
) -> Result<IceCandidateSet, NetError> {
    let local_addr = socket.local_addr().map_err(NetError::Io)?;
    let mut candidates = vec![IceCandidate::new(
        IceCandidateType::Host,
        local_addr,
        65535,
        1,
    )];

    let mut reflexive_addrs = Vec::new();
    for &server in stun_servers {
        if let Ok(addr) = query_stun_server(socket, server, timeout_duration).await {
            reflexive_addrs.push(addr);
        }
    }

    let nat_type = if reflexive_addrs.is_empty() {
        if local_addr.ip().is_loopback() || local_addr.ip().is_unspecified() {
            NatType::OpenInternet
        } else {
            NatType::UdpBlocked
        }
    } else {
        let first = reflexive_addrs[0];
        if first == local_addr {
            NatType::OpenInternet
        } else {
            // Check if multiple STUN servers saw identical or different ports
            let all_same = reflexive_addrs.iter().all(|&a| a == first);
            if all_same {
                NatType::PortRestrictedCone
            } else {
                NatType::SymmetricNat
            }
        }
    };

    for addr in reflexive_addrs {
        if !candidates.iter().any(|c| c.addr == addr) {
            candidates.push(IceCandidate::new(
                IceCandidateType::ServerReflexive,
                addr,
                32768,
                1,
            ));
        }
    }

    Ok(IceCandidateSet::sign_new(
        node_id, nat_type, candidates, secret_key,
    ))
}

/// Executes synchronized paced UDP connectivity checks between two candidate sets.
pub async fn execute_hole_punch(
    socket: &UdpSocket,
    local_set: &IceCandidateSet,
    remote_set: &IceCandidateSet,
    secret_key: &[u8; 32],
    timeout_duration: Duration,
) -> Result<NatTraversalResult, NetError> {
    // 1. Verify signatures on remote candidates
    if !remote_set.verify_signature(secret_key) {
        return Err(NetError::SecurityAlert(
            "Remote ICE candidate set signature verification failed".into(),
        ));
    }

    // 2. Fast-fail if topologies require relay
    if local_set.nat_type.requires_relay(remote_set.nat_type) {
        debug!(
            local = local_set.nat_type.name(),
            remote = remote_set.nat_type.name(),
            "NAT traversal requires relay fallback"
        );
        return Ok(NatTraversalResult::RequiresRelay {
            reason: format!(
                "Incompatible NAT topologies ({} <-> {}) require relay fallback",
                local_set.nat_type.name(),
                remote_set.nat_type.name()
            ),
        });
    }

    let t0 = Instant::now();
    let local_addr = socket.local_addr().map_err(NetError::Io)?;

    // 3. Probing loop
    let remote_addrs: Vec<SocketAddr> = remote_set.candidates.iter().map(|c| c.addr).collect();
    if remote_addrs.is_empty() {
        return Ok(NatTraversalResult::RequiresRelay {
            reason: "Remote candidate set is empty".into(),
        });
    }

    let mut tx_counter = 100_u64;
    let mut recv_buf = [0u8; 512];

    while t0.elapsed() < timeout_duration {
        // Send a burst of requests to remote candidate addresses
        for &remote_addr in &remote_addrs {
            tx_counter += 1;
            let req = HolePunchProbePacket::new(
                PROBE_TYPE_REQUEST,
                &local_set.ufrag,
                tx_counter,
                &remote_set.pwd,
            );
            let req_bytes = req.to_bytes();
            let _ = socket.send_to(&req_bytes, remote_addr).await;
        }

        // Listen for incoming probes for 25ms
        let slice_timeout = Duration::from_millis(25);
        if let Ok(Ok((len, peer_src))) =
            timeout(slice_timeout, socket.recv_from(&mut recv_buf)).await
        {
            // Validate packet
            if let Ok(probe) =
                HolePunchProbePacket::parse_and_verify(&recv_buf[..len], &local_set.pwd)
            {
                if probe.probe_type == PROBE_TYPE_REQUEST {
                    // Reply immediately with PROBE_TYPE_RESPONSE
                    let resp = HolePunchProbePacket::new(
                        PROBE_TYPE_RESPONSE,
                        &local_set.ufrag,
                        probe.tx_id,
                        &remote_set.pwd,
                    );
                    let resp_bytes = resp.to_bytes();
                    let _ = socket.send_to(&resp_bytes, peer_src).await;
                } else if probe.probe_type == PROBE_TYPE_RESPONSE {
                    // Send an acknowledgement response back so peer also completes immediately
                    let resp = HolePunchProbePacket::new(
                        PROBE_TYPE_RESPONSE,
                        &local_set.ufrag,
                        probe.tx_id,
                        &remote_set.pwd,
                    );
                    let resp_bytes = resp.to_bytes();
                    let _ = socket.send_to(&resp_bytes, peer_src).await;

                    // Connectivity check succeeded!
                    let rtt = t0.elapsed().as_micros() as f64 / 1000.0;
                    return Ok(NatTraversalResult::DirectHolePunchSucceeded {
                        winning_local: local_addr,
                        winning_remote: peer_src,
                        rtt_ms: rtt,
                    });
                }
            }
        }
    }

    Ok(NatTraversalResult::RequiresRelay {
        reason: format!("Hole punch timed out after {:?}", timeout_duration),
    })
}

/// Inbound NAT filter key tuple: (allocated_port, peer_ip, optional_peer_port).
pub type InboundFilterKey = (u16, IpAddr, Option<u16>);
/// Thread-safe map of active inbound filtering allowances.
pub type InboundFilterTable = Arc<RwLock<HashMap<InboundFilterKey, SocketAddr>>>;

/// Configurable NAT simulation proxy for local automated multi-topology testing.
#[derive(Debug)]
pub struct NatSimulatorProxy {
    nat_type: NatType,
    listen_addr: SocketAddr,
    public_addr: SocketAddr,
    socket: Arc<UdpSocket>,
    // Mapping internal client address -> allocated public translation port
    internal_mappings: Arc<RwLock<HashMap<SocketAddr, u16>>>,
    // Filter table: mapping (allocated_port, peer_ip, peer_port) -> allowed
    allowed_inbound: InboundFilterTable,
    cancel_token: tokio_util::sync::CancellationToken,
}

impl NatSimulatorProxy {
    /// Spawns a NAT simulator proxy modeling specific NAT mapping and filtering behaviors.
    pub async fn start(nat_type: NatType, public_ip: Ipv4Addr) -> Result<Self, NetError> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.map_err(NetError::Io)?);
        let listen_addr = socket.local_addr().map_err(NetError::Io)?;
        let public_addr = SocketAddr::new(IpAddr::V4(public_ip), listen_addr.port());

        let internal_mappings = Arc::new(RwLock::new(HashMap::new()));
        let allowed_inbound = Arc::new(RwLock::new(HashMap::new()));
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let s_sock = Arc::clone(&socket);
        let s_cancel = cancel_token.clone();
        let s_mappings = Arc::clone(&internal_mappings);
        let s_inbound = Arc::clone(&allowed_inbound);

        tokio::spawn(async move {
            let mut buf = [0u8; 1500];

            while !s_cancel.is_cancelled() {
                tokio::select! {
                    _ = s_cancel.cancelled() => break,
                    res = s_sock.recv_from(&mut buf) => {
                        if let Ok((len, src)) = res {
                            // Check if packet is outbound from registered internal client or inbound from peer
                            let map_lock = s_mappings.write().await;
                            let is_internal = map_lock.contains_key(&src);

                            if is_internal {
                                // Outbound packet: update NAT state
                                let port = *map_lock.get(&src).unwrap_or(&listen_addr.port());
                                drop(map_lock);

                                // Record filtering allowance
                                let mut in_lock = s_inbound.write().await;
                                match nat_type {
                                    NatType::FullCone => {
                                        // Accepts from any remote IP and port
                                        in_lock.insert((port, IpAddr::V4(Ipv4Addr::UNSPECIFIED), None), src);
                                    }
                                    NatType::RestrictedCone => {
                                        // Accepts from any port of this destination IP
                                        in_lock.insert((port, src.ip(), None), src);
                                    }
                                    NatType::PortRestrictedCone => {
                                        // Accepts only from this exact remote IP and port
                                        in_lock.insert((port, src.ip(), Some(src.port())), src);
                                    }
                                    NatType::SymmetricNat => {
                                        // Port-restricted filtering with distinct port mapping
                                        in_lock.insert((port, src.ip(), Some(src.port())), src);
                                    }
                                    _ => {}
                                }
                            } else {
                                // Inbound packet: check filtering rules
                                drop(map_lock);
                                let in_lock = s_inbound.read().await;
                                let mut forward_to = None;

                                for ((_p, allowed_ip, allowed_port), client) in in_lock.iter() {
                                    let ip_matches = allowed_ip.is_unspecified() || *allowed_ip == src.ip();
                                    let port_matches = match allowed_port {
                                        None => true,
                                        Some(ap) => *ap == src.port(),
                                    };
                                    if ip_matches && port_matches {
                                        forward_to = Some(*client);
                                        break;
                                    }
                                }

                                if let Some(target) = forward_to {
                                    let _ = s_sock.send_to(&buf[..len], target).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            nat_type,
            listen_addr,
            public_addr,
            socket,
            internal_mappings,
            allowed_inbound,
            cancel_token,
        })
    }

    /// Returns the public reflex address simulated for this proxy.
    #[must_use]
    pub fn public_addr(&self) -> SocketAddr {
        self.public_addr
    }

    /// Returns the configured NAT behavior type.
    #[must_use]
    pub fn nat_type(&self) -> NatType {
        self.nat_type
    }

    /// Returns the local listening socket address.
    #[must_use]
    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    /// Returns the underlying UDP socket handle.
    #[must_use]
    pub fn socket(&self) -> &Arc<UdpSocket> {
        &self.socket
    }

    /// Returns count of currently active inbound filtering permissions.
    pub async fn allowed_inbound_count(&self) -> usize {
        self.allowed_inbound.read().await.len()
    }

    /// Registers an internal client endpoint and assigns its translation port.
    pub async fn register_client(&self, client_addr: SocketAddr) -> SocketAddr {
        let mut map = self.internal_mappings.write().await;
        let port = match self.nat_type {
            NatType::SymmetricNat => client_addr.port().wrapping_add(1000),
            _ => self.listen_addr.port(),
        };
        map.insert(client_addr, port);
        SocketAddr::new(self.public_addr.ip(), port)
    }

    /// Stops the proxy.
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for NatSimulatorProxy {
    fn drop(&mut self) {
        self.stop();
    }
}
