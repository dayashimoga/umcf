//! Zero-configuration LAN discovery engine using signed UDP multicast and mDNS/DNS-SD beacons.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use smallvec::SmallVec;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use umcf_core::{NodeId, NodeRole};
use umcf_identity::NodeKeypair;
use umcf_registry::HlcTimestamp;

use crate::beacon::{BeaconAnnouncementParams, SignedDiscoveryBeacon, BEACON_MAGIC};
use crate::error::NetError;
use crate::rate_limiter::{ReplayFilter, TokenBucketLimiter};

/// Default IPv4 multicast rendezvous address for UMCF mesh discovery (`239.255.77.67:44767`).
pub const DEFAULT_MULTICAST_IPV4: &str = "239.255.77.67:44767";

/// Configuration options for the LAN discovery engine.
#[derive(Debug, Clone)]
pub struct LanDiscoveryConfig {
    /// Identifier of the cluster to announce and filter for.
    pub cluster_id: String,
    /// UDP socket address to bind for sending and listening.
    pub bind_addr: SocketAddr,
    /// Primary destination address for beacon multicasting/broadcasting.
    pub target_addr: SocketAddr,
    /// Optional additional destinations for directed unicast or secondary broadcast.
    pub secondary_targets: Vec<SocketAddr>,
    /// Local QUIC data-plane port to announce to discovered peers.
    pub quic_port: u16,
    /// Interval in milliseconds between outgoing discovery beacons.
    pub beacon_interval_ms: u64,
    /// Duration in milliseconds after which an unrefreshed peer is pruned.
    pub peer_timeout_ms: u64,
    /// BLAKE3 digest of local hardware capability descriptor.
    pub capability_digest: [u8; 32],
}

impl LanDiscoveryConfig {
    /// Creates a default discovery configuration for a given cluster and QUIC port.
    #[must_use]
    pub fn new(cluster_id: impl Into<String>, quic_port: u16) -> Self {
        let target = SocketAddr::from(([239, 255, 77, 67], 44767));
        let bind = SocketAddr::from(([0, 0, 0, 0], 44767));

        Self {
            cluster_id: cluster_id.into(),
            bind_addr: bind,
            target_addr: target,
            secondary_targets: Vec::new(),
            quic_port,
            beacon_interval_ms: 250,
            peer_timeout_ms: 3000,
            capability_digest: [0u8; 32],
        }
    }
}

/// Discovered peer record maintained by the discovery engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    /// Canonical NodeId of the discovered peer.
    pub node_id: NodeId,
    /// Sender DID string (`did:key:...`).
    pub sender_did: String,
    /// Operational role of the peer (Master, Worker, Hybrid).
    pub role: NodeRole,
    /// Cluster ID the peer belongs to.
    pub cluster_id: String,
    /// Reachable QUIC port on the peer.
    pub quic_port: u16,
    /// Discovered IP addresses and socket endpoints.
    pub listen_addrs: Vec<SocketAddr>,
    /// BLAKE3 hardware capability digest.
    pub capability_digest: [u8; 32],
    /// Monotonic HLC timestamp from the latest beacon.
    pub hlc_timestamp: HlcTimestamp,
    /// Monotonic sequence number of the latest beacon.
    pub sequence_number: u64,
    /// Local timestamp (epoch ms) when the peer was last observed.
    pub last_seen_ms: u64,
}

/// Active LAN discovery engine managing periodic beacon broadcasts and validated peer discovery.
#[derive(Debug)]
pub struct LanDiscoveryEngine {
    config: LanDiscoveryConfig,
    keypair: Arc<NodeKeypair>,
    peers: Arc<RwLock<HashMap<NodeId, DiscoveredPeer>>>,
    notifier: broadcast::Sender<DiscoveredPeer>,
    cancel_token: CancellationToken,
}

impl LanDiscoveryEngine {
    /// Spawns and starts the LAN discovery engine background tasks.
    ///
    /// # Errors
    /// Returns `NetError::Io` if socket creation or binding fails.
    pub async fn start(
        config: LanDiscoveryConfig,
        keypair: Arc<NodeKeypair>,
        role: NodeRole,
    ) -> Result<Self, NetError> {
        let std_socket = create_reusable_socket(config.bind_addr)?;
        let socket = UdpSocket::from_std(std_socket)?;
        let _ = socket.set_broadcast(true);

        if let SocketAddr::V4(v4) = config.target_addr {
            if v4.ip().is_multicast() {
                let _ = socket.set_multicast_loop_v4(true);
                let _ = socket.set_multicast_ttl_v4(2);
                let iface = match config.bind_addr {
                    SocketAddr::V4(b) => *b.ip(),
                    _ => Ipv4Addr::UNSPECIFIED,
                };
                let _ = socket.join_multicast_v4(*v4.ip(), iface);
            }
        }

        let local_addr = socket.local_addr()?;
        let socket = Arc::new(socket);

        let peers = Arc::new(RwLock::new(HashMap::new()));
        let (notifier, _) = broadcast::channel(128);
        let cancel_token = CancellationToken::new();

        let node_id = keypair.node_id();

        // 1. Broadcast Task
        let b_socket = Arc::clone(&socket);
        let b_config = config.clone();
        let b_keypair = Arc::clone(&keypair);
        let b_cancel = cancel_token.clone();

        tokio::spawn(async move {
            let mut seq = 0_u64;
            let mut interval = tokio::time::interval(Duration::from_millis(b_config.beacon_interval_ms));

            while !b_cancel.is_cancelled() {
                tokio::select! {
                    _ = b_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        seq += 1;
                        let now_ms = current_time_ms();
                        let hlc = HlcTimestamp::new(now_ms, 0, 0x4d45_5348); // 'MESH'

                        let mut addrs = SmallVec::new();
                        addrs.push(local_addr);

                        let beacon = SignedDiscoveryBeacon::sign_new(BeaconAnnouncementParams {
                            cluster_id: &b_config.cluster_id,
                            keypair: &b_keypair,
                            hlc_timestamp: hlc,
                            sequence_number: seq,
                            role,
                            quic_port: b_config.quic_port,
                            listen_addrs: addrs,
                            capability_digest: b_config.capability_digest,
                        });

                        if let Ok(bytes) = beacon.to_bytes() {
                            let _ = b_socket.send_to(&bytes, b_config.target_addr).await;
                            for &sec_target in &b_config.secondary_targets {
                                let _ = b_socket.send_to(&bytes, sec_target).await;
                            }
                        }
                    }
                }
            }
        });

        // 2. Receive and Process Task
        let r_socket = Arc::clone(&socket);
        let r_peers = Arc::clone(&peers);
        let r_config = config.clone();
        let r_notifier = notifier.clone();
        let r_cancel = cancel_token.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let mut rate_limiter = TokenBucketLimiter::new();
            let mut replay_filter = ReplayFilter::new();

            while !r_cancel.is_cancelled() {
                tokio::select! {
                    _ = r_cancel.cancelled() => break,
                    recv_res = r_socket.recv_from(&mut buf) => {
                        let (len, src) = match recv_res {
                            Ok(res) => res,
                            Err(_) => continue,
                        };

                        let packet_bytes = &buf[..len];
                        let now_ms = current_time_ms();

                        // Fast reject malformed/too small (< 16 bytes or magic header mismatch)
                        if len < 16 || packet_bytes[0..4] != BEACON_MAGIC {
                            continue;
                        }

                        // Rate limit check
                        if rate_limiter.check_rate_limit(src.ip(), now_ms).is_err() {
                            continue;
                        }

                        // Fast deserialize without signature verification for fast filtering
                        let beacon = match SignedDiscoveryBeacon::from_bytes_unverified(packet_bytes) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };

                        // Cluster ID isolation check (< 50 ns)
                        if beacon.cluster_id != r_config.cluster_id {
                            continue;
                        }

                        // Derive NodeId
                        let peer_node_id = match beacon.node_id() {
                            Ok(id) => id,
                            Err(_) => continue,
                        };

                        // Don't discover oneself
                        if peer_node_id == node_id {
                            continue;
                        }

                        // Anti-replay check (< 50 ns)
                        if replay_filter.check_and_update(&beacon.sender_did, beacon.sequence_number, now_ms).is_err() {
                            continue;
                        }

                        // Cryptographic signature verification only after cluster and replay pass
                        if beacon.verify_signature().is_err() {
                            continue;
                        }

                        let discovered = DiscoveredPeer {
                            node_id: peer_node_id.clone(),
                            sender_did: beacon.sender_did,
                            role: beacon.role,
                            cluster_id: beacon.cluster_id,
                            quic_port: beacon.quic_port,
                            listen_addrs: beacon.listen_addrs.into_vec(),
                            capability_digest: beacon.capability_digest,
                            hlc_timestamp: beacon.hlc_timestamp,
                            sequence_number: beacon.sequence_number,
                            last_seen_ms: now_ms,
                        };

                        // Update peers table
                        if let Ok(mut lock) = r_peers.write() {
                            lock.insert(peer_node_id, discovered.clone());
                        }

                        // Notify listeners
                        let _ = r_notifier.send(discovered);
                    }
                }
            }
        });

        // 3. Stale Peer Pruner Task
        let p_peers = Arc::clone(&peers);
        let p_config = config.clone();
        let p_cancel = cancel_token.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(p_config.peer_timeout_ms / 2));
            while !p_cancel.is_cancelled() {
                tokio::select! {
                    _ = p_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        let now_ms = current_time_ms();
                        if let Ok(mut lock) = p_peers.write() {
                            lock.retain(|_, peer| {
                                now_ms.saturating_sub(peer.last_seen_ms) <= p_config.peer_timeout_ms
                            });
                        }
                    }
                }
            }
        });

        Ok(Self {
            config,
            keypair,
            peers,
            notifier,
            cancel_token,
        })
    }

    /// Evaluates and processes an incoming raw beacon buffer directly.
    /// Returns the exact validation duration and result (used for microbenchmarking drop latency).
    pub fn evaluate_raw_beacon(
        &self,
        packet: &[u8],
        src_ip: IpAddr,
        limiter: &mut TokenBucketLimiter,
        replay: &mut ReplayFilter,
    ) -> (Result<SignedDiscoveryBeacon, NetError>, Duration) {
        let t0 = Instant::now();
        let now_ms = current_time_ms();

        // 1. Magic header check
        if packet.len() < 16 || packet[0..4] != BEACON_MAGIC {
            return (
                Err(NetError::InvalidBeacon("Invalid beacon magic".into())),
                t0.elapsed(),
            );
        }

        // 2. Rate limit check
        if let Err(e) = limiter.check_rate_limit(src_ip, now_ms) {
            return (Err(e), t0.elapsed());
        }

        // 3. Fast deserialization without curve math
        let beacon = match SignedDiscoveryBeacon::from_bytes_unverified(packet) {
            Ok(b) => b,
            Err(e) => return (Err(e), t0.elapsed()),
        };

        // 4. Cluster mismatch check (< 50 ns)
        if beacon.cluster_id != self.config.cluster_id {
            return (
                Err(NetError::ClusterMismatch {
                    expected: self.config.cluster_id.clone(),
                    actual: beacon.cluster_id,
                }),
                t0.elapsed(),
            );
        }

        // 5. Anti-replay check (< 50 ns)
        if let Err(e) = replay.check_and_update(&beacon.sender_did, beacon.sequence_number, now_ms) {
            return (Err(e), t0.elapsed());
        }

        // 6. Cryptographic signature verification
        if let Err(e) = beacon.verify_signature() {
            return (Err(e), t0.elapsed());
        }

        (Ok(beacon), t0.elapsed())
    }

    /// Returns the local node identity.
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.keypair.node_id()
    }

    /// Returns a reference to the local node keypair.
    #[must_use]
    pub fn keypair(&self) -> &Arc<NodeKeypair> {
        &self.keypair
    }

    /// Returns a snapshot list of currently active discovered peers.
    #[must_use]
    pub fn discovered_peers(&self) -> Vec<DiscoveredPeer> {
        self.peers
            .read()
            .map(|lock| lock.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Subscribes to peer discovery events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveredPeer> {
        self.notifier.subscribe()
    }

    /// Stops the discovery engine and background tasks cleanly.
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for LanDiscoveryEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

fn create_reusable_socket(bind_addr: SocketAddr) -> std::io::Result<std::net::UdpSocket> {
    let domain = if bind_addr.is_ipv6() {
        Domain::IPV6
    } else {
        Domain::IPV4
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(not(windows))]
    let _ = socket.set_reuse_port(true);
    socket.set_nonblocking(true)?;
    socket.bind(&bind_addr.into())?;
    Ok(socket.into())
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
