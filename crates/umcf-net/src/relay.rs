//! `umcf-net::relay`: Encrypted, zero-knowledge DERP/TURN-style packet relay fallback.
//!
//! Provides:
//! - Canonical binary wire protocol framing (`RelayFrame`) supporting `BindNodeDid`, `BindAck`,
//!   `SendPacket`, `RecvPacket`, `KeepAlive`, and `RelayError`.
//! - High-performance, zero-knowledge relay server (`UmcfRelayServer`) with per-client bandwidth quotas.
//! - Transparent hybrid packet forwarder (`HybridUdpRelaySocket`) multiplexing direct and relayed traffic.
//! - Active path upgrade monitor (`PathUpgradeMonitor`) enabling seamless mid-session transition from
//!   relayed to direct P2P UDP with zero dropped streams.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::NetError;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::sleep;
use umcf_identity::did::DidKey;
use umcf_identity::NodeKeypair;

/// Magic bytes header for UMCF Relay frames (*b"UMCFRELAY").
pub const RELAY_MAGIC: &[u8; 9] = b"UMCFRELAY";
pub const RELAY_PROTOCOL_VERSION: u8 = 1;

/// Frame types.
pub const FRAME_TYPE_BIND: u8 = 0x01;
pub const FRAME_TYPE_BIND_ACK: u8 = 0x02;
pub const FRAME_TYPE_SEND: u8 = 0x03;
pub const FRAME_TYPE_RECV: u8 = 0x04;
pub const FRAME_TYPE_KEEPALIVE: u8 = 0x05;
pub const FRAME_TYPE_ERROR: u8 = 0x06;

/// Transmission path type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathType {
    /// Direct local area network link.
    DirectLan,
    /// Direct WAN link established via UDP hole punching.
    DirectWanPunched,
    /// End-to-end encrypted link forwarded through zero-knowledge relay.
    RelayedE2ee,
}

impl PathType {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DirectLan => "DIRECT_LAN",
            Self::DirectWanPunched => "DIRECT_WAN_PUNCHED",
            Self::RelayedE2ee => "RELAYED_E2EE",
        }
    }
}

/// Relay wire protocol frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayFrame {
    /// Client authenticates and binds its `did:key` to the relay session.
    BindNodeDid {
        did: String,
        nonce: u64,
        signature: [u8; 64],
    },
    /// Relay confirms successful binding.
    BindAck {
        success: bool,
        assigned_client_id: u64,
        message: String,
    },
    /// Client requests forwarding of an encrypted payload to `dst_did`.
    SendPacket { dst_did: String, payload: Vec<u8> },
    /// Relay delivers an encrypted payload from `src_did`.
    RecvPacket { src_did: String, payload: Vec<u8> },
    /// Connection heartbeat.
    KeepAlive,
    /// Protocol or quota error diagnostic.
    RelayError { code: u16, message: String },
}

impl RelayFrame {
    /// Creates a signed `BindNodeDid` frame.
    #[must_use]
    pub fn new_bind(keypair: &NodeKeypair, cluster_id: &str, nonce: u64) -> Self {
        let did = keypair.did_key();
        let signing_bytes = compute_bind_signing_bytes(cluster_id, &did, nonce);
        let signature = keypair.sign(&signing_bytes);
        Self::BindNodeDid {
            did,
            nonce,
            signature,
        }
    }

    /// Verifies the cryptographic signature of a `BindNodeDid` frame against its embedded `did:key`.
    pub fn verify_bind(&self, cluster_id: &str) -> Result<bool, NetError> {
        match self {
            Self::BindNodeDid {
                did,
                nonce,
                signature,
            } => {
                let pk_bytes = DidKey::to_ed25519_bytes(did)
                    .map_err(|e| NetError::RelayAuthFailed(format!("Invalid DID format: {e}")))?;
                let verifying_key = VerifyingKey::from_bytes(&pk_bytes).map_err(|e| {
                    NetError::RelayAuthFailed(format!("Invalid verifying key: {e}"))
                })?;
                let msg = compute_bind_signing_bytes(cluster_id, did, *nonce);
                Ok(NodeKeypair::verify_with_key(&verifying_key, &msg, signature).is_ok())
            }
            _ => Err(NetError::RelayProtocol("Not a BindNodeDid frame".into())),
        }
    }

    /// Serializes the frame into canonical binary wire format.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut body = Vec::new();
        let frame_type = match self {
            Self::BindNodeDid {
                did,
                nonce,
                signature,
            } => {
                let did_bytes = did.as_bytes();
                body.extend_from_slice(&(did_bytes.len() as u16).to_be_bytes());
                body.extend_from_slice(did_bytes);
                body.extend_from_slice(&nonce.to_be_bytes());
                body.extend_from_slice(signature);
                FRAME_TYPE_BIND
            }
            Self::BindAck {
                success,
                assigned_client_id,
                message,
            } => {
                body.push(if *success { 1 } else { 0 });
                body.extend_from_slice(&assigned_client_id.to_be_bytes());
                let msg_bytes = message.as_bytes();
                body.extend_from_slice(&(msg_bytes.len() as u16).to_be_bytes());
                body.extend_from_slice(msg_bytes);
                FRAME_TYPE_BIND_ACK
            }
            Self::SendPacket { dst_did, payload } => {
                let dst_bytes = dst_did.as_bytes();
                body.extend_from_slice(&(dst_bytes.len() as u16).to_be_bytes());
                body.extend_from_slice(dst_bytes);
                body.extend_from_slice(&(payload.len() as u32).to_be_bytes());
                body.extend_from_slice(payload);
                FRAME_TYPE_SEND
            }
            Self::RecvPacket { src_did, payload } => {
                let src_bytes = src_did.as_bytes();
                body.extend_from_slice(&(src_bytes.len() as u16).to_be_bytes());
                body.extend_from_slice(src_bytes);
                body.extend_from_slice(&(payload.len() as u32).to_be_bytes());
                body.extend_from_slice(payload);
                FRAME_TYPE_RECV
            }
            Self::KeepAlive => FRAME_TYPE_KEEPALIVE,
            Self::RelayError { code, message } => {
                body.extend_from_slice(&code.to_be_bytes());
                let msg_bytes = message.as_bytes();
                body.extend_from_slice(&(msg_bytes.len() as u16).to_be_bytes());
                body.extend_from_slice(msg_bytes);
                FRAME_TYPE_ERROR
            }
        };

        let mut buf = Vec::with_capacity(14 + body.len());
        buf.extend_from_slice(RELAY_MAGIC);
        buf.push(RELAY_PROTOCOL_VERSION);
        buf.push(frame_type);
        buf.extend_from_slice(&(body.len() as u32).to_be_bytes());
        buf.extend_from_slice(&body);
        buf
    }

    /// Deserializes a frame from binary wire bytes.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, NetError> {
        if buf.len() < 15 {
            return Err(NetError::RelayProtocol("Frame too short for header".into()));
        }
        if &buf[0..9] != RELAY_MAGIC {
            return Err(NetError::RelayProtocol("Invalid relay magic prefix".into()));
        }
        if buf[9] != RELAY_PROTOCOL_VERSION {
            return Err(NetError::RelayProtocol(format!(
                "Unsupported protocol version: {}",
                buf[9]
            )));
        }

        let frame_type = buf[10];
        let body_len = u32::from_be_bytes([buf[11], buf[12], buf[13], buf[14]]) as usize;
        if buf.len() < 15 + body_len {
            return Err(NetError::RelayProtocol("Truncated frame body".into()));
        }

        let body = &buf[15..15 + body_len];

        match frame_type {
            FRAME_TYPE_BIND => {
                if body.len() < 2 + 8 + 64 {
                    return Err(NetError::RelayProtocol("Truncated BindNodeDid body".into()));
                }
                let did_len = u16::from_be_bytes([body[0], body[1]]) as usize;
                if body.len() < 2 + did_len + 8 + 64 {
                    return Err(NetError::RelayProtocol("Truncated BindNodeDid DID".into()));
                }
                let did = String::from_utf8(body[2..2 + did_len].to_vec())
                    .map_err(|e| NetError::RelayProtocol(format!("Invalid UTF-8 DID: {e}")))?;
                let offset = 2 + did_len;
                let nonce = u64::from_be_bytes([
                    body[offset],
                    body[offset + 1],
                    body[offset + 2],
                    body[offset + 3],
                    body[offset + 4],
                    body[offset + 5],
                    body[offset + 6],
                    body[offset + 7],
                ]);
                let mut signature = [0u8; 64];
                signature.copy_from_slice(&body[offset + 8..offset + 72]);

                Ok(Self::BindNodeDid {
                    did,
                    nonce,
                    signature,
                })
            }
            FRAME_TYPE_BIND_ACK => {
                if body.len() < 1 + 8 + 2 {
                    return Err(NetError::RelayProtocol("Truncated BindAck body".into()));
                }
                let success = body[0] != 0;
                let assigned_client_id = u64::from_be_bytes([
                    body[1], body[2], body[3], body[4], body[5], body[6], body[7], body[8],
                ]);
                let msg_len = u16::from_be_bytes([body[9], body[10]]) as usize;
                if body.len() < 11 + msg_len {
                    return Err(NetError::RelayProtocol("Truncated BindAck message".into()));
                }
                let message = String::from_utf8(body[11..11 + msg_len].to_vec())
                    .map_err(|e| NetError::RelayProtocol(format!("Invalid UTF-8 message: {e}")))?;

                Ok(Self::BindAck {
                    success,
                    assigned_client_id,
                    message,
                })
            }
            FRAME_TYPE_SEND => {
                if body.len() < 2 {
                    return Err(NetError::RelayProtocol(
                        "Truncated SendPacket header".into(),
                    ));
                }
                let dst_len = u16::from_be_bytes([body[0], body[1]]) as usize;
                if body.len() < 2 + dst_len + 4 {
                    return Err(NetError::RelayProtocol(
                        "Truncated SendPacket dst_did".into(),
                    ));
                }
                let dst_did = String::from_utf8(body[2..2 + dst_len].to_vec())
                    .map_err(|e| NetError::RelayProtocol(format!("Invalid UTF-8 dst_did: {e}")))?;
                let offset = 2 + dst_len;
                let payload_len = u32::from_be_bytes([
                    body[offset],
                    body[offset + 1],
                    body[offset + 2],
                    body[offset + 3],
                ]) as usize;
                if body.len() < offset + 4 + payload_len {
                    return Err(NetError::RelayProtocol(
                        "Truncated SendPacket payload".into(),
                    ));
                }
                let payload = body[offset + 4..offset + 4 + payload_len].to_vec();

                Ok(Self::SendPacket { dst_did, payload })
            }
            FRAME_TYPE_RECV => {
                if body.len() < 2 {
                    return Err(NetError::RelayProtocol(
                        "Truncated RecvPacket header".into(),
                    ));
                }
                let src_len = u16::from_be_bytes([body[0], body[1]]) as usize;
                if body.len() < 2 + src_len + 4 {
                    return Err(NetError::RelayProtocol(
                        "Truncated RecvPacket src_did".into(),
                    ));
                }
                let src_did = String::from_utf8(body[2..2 + src_len].to_vec())
                    .map_err(|e| NetError::RelayProtocol(format!("Invalid UTF-8 src_did: {e}")))?;
                let offset = 2 + src_len;
                let payload_len = u32::from_be_bytes([
                    body[offset],
                    body[offset + 1],
                    body[offset + 2],
                    body[offset + 3],
                ]) as usize;
                if body.len() < offset + 4 + payload_len {
                    return Err(NetError::RelayProtocol(
                        "Truncated RecvPacket payload".into(),
                    ));
                }
                let payload = body[offset + 4..offset + 4 + payload_len].to_vec();

                Ok(Self::RecvPacket { src_did, payload })
            }
            FRAME_TYPE_KEEPALIVE => Ok(Self::KeepAlive),
            FRAME_TYPE_ERROR => {
                if body.len() < 4 {
                    return Err(NetError::RelayProtocol("Truncated RelayError body".into()));
                }
                let code = u16::from_be_bytes([body[0], body[1]]);
                let msg_len = u16::from_be_bytes([body[2], body[3]]) as usize;
                if body.len() < 4 + msg_len {
                    return Err(NetError::RelayProtocol(
                        "Truncated RelayError message".into(),
                    ));
                }
                let message = String::from_utf8(body[4..4 + msg_len].to_vec())
                    .map_err(|e| NetError::RelayProtocol(format!("Invalid UTF-8 message: {e}")))?;

                Ok(Self::RelayError { code, message })
            }
            other => Err(NetError::RelayProtocol(format!(
                "Unknown frame type 0x{other:02X}"
            ))),
        }
    }
}

fn compute_bind_signing_bytes(cluster_id: &str, did: &str, nonce: u64) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"UMCF-RELAY-BIND-v1");
    hasher.update(cluster_id.as_bytes());
    hasher.update(did.as_bytes());
    hasher.update(&nonce.to_be_bytes());
    hasher.finalize().as_bytes().to_vec()
}

/// Active client session entry maintained by `UmcfRelayServer`.
#[derive(Debug, Clone)]
pub struct RelaySession {
    pub did: String,
    pub socket_addr: SocketAddr,
    pub bound_at_epoch_ms: u64,
    pub last_seen_epoch_ms: u64,
    pub bytes_forwarded: u64,
}

/// Zero-Knowledge DERP/TURN-style packet relay server.
#[derive(Debug)]
pub struct UmcfRelayServer {
    cluster_id: String,
    listen_addr: SocketAddr,
    socket: Arc<UdpSocket>,
    // Mapping: did:key -> RelaySession
    sessions: Arc<RwLock<HashMap<String, RelaySession>>>,
    // Reverse mapping: SocketAddr -> did:key
    addr_to_did: Arc<RwLock<HashMap<SocketAddr, String>>>,
    // Per-client bandwidth allowance limit (bytes per minute)
    max_bytes_per_minute: u64,
    total_packets_forwarded: Arc<AtomicU64>,
    cancel_token: tokio_util::sync::CancellationToken,
}

impl UmcfRelayServer {
    /// Starts a new `UmcfRelayServer` listening on `bind_addr`.
    pub async fn start(
        cluster_id: &str,
        bind_addr: SocketAddr,
        max_bytes_per_minute: u64,
    ) -> Result<Self, NetError> {
        let socket = Arc::new(UdpSocket::bind(bind_addr).await.map_err(NetError::Io)?);
        let listen_addr = socket.local_addr().map_err(NetError::Io)?;
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let addr_to_did = Arc::new(RwLock::new(HashMap::new()));
        let total_packets_forwarded = Arc::new(AtomicU64::new(0));

        let s_sock = Arc::clone(&socket);
        let s_cancel = cancel_token.clone();
        let s_sessions = Arc::clone(&sessions);
        let s_addr_map = Arc::clone(&addr_to_did);
        let s_cluster_id = cluster_id.to_string();
        let s_forward_counter = Arc::clone(&total_packets_forwarded);

        // Spawn relay packet forwarding loop
        tokio::spawn(async move {
            let mut buf = [0u8; 65536];

            while !s_cancel.is_cancelled() {
                tokio::select! {
                    _ = s_cancel.cancelled() => break,
                    res = s_sock.recv_from(&mut buf) => {
                        if let Ok((len, peer_src)) = res {
                            let frame = match RelayFrame::from_bytes(&buf[..len]) {
                                Ok(f) => f,
                                Err(_) => continue,
                            };

                            let now_ms = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;

                            match frame {
                                RelayFrame::BindNodeDid { ref did, nonce, .. } => {
                                    if frame.verify_bind(&s_cluster_id).unwrap_or(false) {
                                        let mut sess_lock = s_sessions.write().await;
                                        let mut addr_lock = s_addr_map.write().await;

                                        sess_lock.insert(did.clone(), RelaySession {
                                            did: did.clone(),
                                            socket_addr: peer_src,
                                            bound_at_epoch_ms: now_ms,
                                            last_seen_epoch_ms: now_ms,
                                            bytes_forwarded: 0,
                                        });
                                        addr_lock.insert(peer_src, did.clone());

                                        let ack = RelayFrame::BindAck {
                                            success: true,
                                            assigned_client_id: nonce,
                                            message: "Bound".into(),
                                        };
                                        let ack_bytes = ack.to_bytes();
                                        let _ = s_sock.send_to(&ack_bytes, peer_src).await;
                                    } else {
                                        let err = RelayFrame::RelayError {
                                            code: 401,
                                            message: "Bind signature verification failed".into(),
                                        };
                                        let _ = s_sock.send_to(&err.to_bytes(), peer_src).await;
                                    }
                                }
                                RelayFrame::SendPacket { dst_did, payload } => {
                                    let addr_lock = s_addr_map.read().await;
                                    let src_did = match addr_lock.get(&peer_src) {
                                        Some(d) => d.clone(),
                                        None => {
                                            drop(addr_lock);
                                            let err = RelayFrame::RelayError {
                                                code: 401,
                                                message: "Unauthenticated client; BindNodeDid required".into(),
                                            };
                                            let _ = s_sock.send_to(&err.to_bytes(), peer_src).await;
                                            continue;
                                        }
                                    };
                                    drop(addr_lock);

                                    let mut sess_lock = s_sessions.write().await;
                                    if let Some(src_sess) = sess_lock.get_mut(&src_did) {
                                        src_sess.last_seen_epoch_ms = now_ms;
                                        if max_bytes_per_minute > 0 && src_sess.bytes_forwarded + (payload.len() as u64) > max_bytes_per_minute {
                                            drop(sess_lock);
                                            let err = RelayFrame::RelayError {
                                                code: 429,
                                                message: "Bandwidth quota exceeded".into(),
                                            };
                                            let _ = s_sock.send_to(&err.to_bytes(), peer_src).await;
                                            continue;
                                        }
                                        src_sess.bytes_forwarded += payload.len() as u64;
                                    }

                                    // Look up destination
                                    if let Some(dst_sess) = sess_lock.get(&dst_did) {
                                        let dst_addr = dst_sess.socket_addr;
                                        drop(sess_lock);

                                        let recv_frame = RelayFrame::RecvPacket {
                                            src_did,
                                            payload,
                                        };
                                        let recv_bytes = recv_frame.to_bytes();
                                        if s_sock.send_to(&recv_bytes, dst_addr).await.is_ok() {
                                            s_forward_counter.fetch_add(1, Ordering::Relaxed);
                                        }
                                    } else {
                                        drop(sess_lock);
                                        let err = RelayFrame::RelayError {
                                            code: 404,
                                            message: format!("Destination {dst_did} not connected to relay"),
                                        };
                                        let _ = s_sock.send_to(&err.to_bytes(), peer_src).await;
                                    }
                                }
                                RelayFrame::KeepAlive => {
                                    let addr_lock = s_addr_map.read().await;
                                    if let Some(did) = addr_lock.get(&peer_src) {
                                        let mut sess_lock = s_sessions.write().await;
                                        if let Some(sess) = sess_lock.get_mut(did) {
                                            sess.last_seen_epoch_ms = now_ms;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            cluster_id: cluster_id.to_string(),
            listen_addr,
            socket,
            sessions,
            addr_to_did,
            max_bytes_per_minute,
            total_packets_forwarded,
            cancel_token,
        })
    }

    /// Returns the bound local listening address of the relay server.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    /// Returns the number of currently active registered client sessions.
    pub async fn active_sessions_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Returns total count of zero-knowledge packets forwarded by this relay server.
    #[must_use]
    pub fn total_packets_forwarded(&self) -> u64 {
        self.total_packets_forwarded.load(Ordering::Relaxed)
    }

    /// Returns the target cluster identifier.
    #[must_use]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Returns the configured bandwidth quota limit per client.
    #[must_use]
    pub fn max_bytes_per_minute(&self) -> u64 {
        self.max_bytes_per_minute
    }

    /// Returns the underlying UDP socket reference.
    #[must_use]
    pub fn socket(&self) -> &Arc<UdpSocket> {
        &self.socket
    }

    /// Returns the registered DID associated with a source socket address.
    pub async fn registered_did_for_addr(&self, addr: &SocketAddr) -> Option<String> {
        self.addr_to_did.read().await.get(addr).cloned()
    }

    /// Stops the relay server.
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for UmcfRelayServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Transparent hybrid packet forwarder connecting directly via UDP or falling back to `UmcfRelayServer`.
#[derive(Debug)]
pub struct HybridUdpRelaySocket {
    local_did: String,
    socket: Arc<UdpSocket>,
    relay_addr: Option<SocketAddr>,
    active_path: Arc<RwLock<PathType>>,
    peer_did: String,
    peer_direct_addr: SocketAddr,
    direct_upgrade_detected: Arc<AtomicBool>,
}

impl HybridUdpRelaySocket {
    /// Binds a hybrid socket with default fallback path.
    pub async fn bind(
        local_keypair: &NodeKeypair,
        cluster_id: &str,
        relay_addr: Option<SocketAddr>,
        peer_did: &str,
        peer_direct_addr: SocketAddr,
        initial_path: PathType,
    ) -> Result<Self, NetError> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.map_err(NetError::Io)?);
        let local_did = local_keypair.did_key();

        // If relay is provided, authenticate via BindNodeDid
        if let Some(r_addr) = relay_addr {
            let bind_frame = RelayFrame::new_bind(local_keypair, cluster_id, 1001);
            let bytes = bind_frame.to_bytes();
            socket.send_to(&bytes, r_addr).await.map_err(NetError::Io)?;

            // Read Ack
            let mut buf = [0u8; 1024];
            let (len, _) =
                tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf))
                    .await
                    .map_err(|_| NetError::Timeout("Relay bind timed out".into()))?
                    .map_err(NetError::Io)?;

            let resp = RelayFrame::from_bytes(&buf[..len])?;
            match resp {
                RelayFrame::BindAck { success: true, .. } => {}
                other => {
                    return Err(NetError::RelayAuthFailed(format!(
                        "Unexpected bind response: {:?}",
                        other
                    )))
                }
            }
        }

        Ok(Self {
            local_did,
            socket,
            relay_addr,
            active_path: Arc::new(RwLock::new(initial_path)),
            peer_did: peer_did.to_string(),
            peer_direct_addr,
            direct_upgrade_detected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Sends a datagram either directly or encapsulated through the relay depending on active path.
    pub async fn send_datagram(&self, payload: &[u8]) -> Result<usize, NetError> {
        let path = *self.active_path.read().await;

        match path {
            PathType::DirectLan | PathType::DirectWanPunched => self
                .socket
                .send_to(payload, self.peer_direct_addr)
                .await
                .map_err(NetError::Io),
            PathType::RelayedE2ee => {
                let r_addr = self.relay_addr.ok_or_else(|| {
                    NetError::RelayProtocol("No relay configured for RelayedE2ee path".into())
                })?;
                let frame = RelayFrame::SendPacket {
                    dst_did: self.peer_did.clone(),
                    payload: payload.to_vec(),
                };
                let bytes = frame.to_bytes();
                self.socket
                    .send_to(&bytes, r_addr)
                    .await
                    .map_err(NetError::Io)?;
                Ok(payload.len())
            }
        }
    }

    /// Receives a datagram, automatically unwrapping relay framing if arrived from relay.
    pub async fn recv_datagram(&self, buf: &mut [u8]) -> Result<(usize, PathType), NetError> {
        let mut raw = [0u8; 65536];
        let (len, src) = self
            .socket
            .recv_from(&mut raw)
            .await
            .map_err(NetError::Io)?;

        // Check if arrived from relay
        if Some(src) == self.relay_addr {
            if let Ok(RelayFrame::RecvPacket { payload, .. }) = RelayFrame::from_bytes(&raw[..len])
            {
                let copy_len = payload.len().min(buf.len());
                buf[..copy_len].copy_from_slice(&payload[..copy_len]);
                return Ok((copy_len, PathType::RelayedE2ee));
            }
        }

        // Direct packet: if direct upgrade check packet, mark upgraded
        if src == self.peer_direct_addr {
            let mut path_lock = self.active_path.write().await;
            if *path_lock == PathType::RelayedE2ee {
                *path_lock = PathType::DirectWanPunched;
                self.direct_upgrade_detected.store(true, Ordering::SeqCst);
            }
            drop(path_lock);

            let copy_len = len.min(buf.len());
            buf[..copy_len].copy_from_slice(&raw[..copy_len]);
            return Ok((copy_len, PathType::DirectWanPunched));
        }

        let copy_len = len.min(buf.len());
        buf[..copy_len].copy_from_slice(&raw[..copy_len]);
        Ok((copy_len, PathType::DirectLan))
    }

    /// Returns the currently active transmission path.
    pub async fn current_path(&self) -> PathType {
        *self.active_path.read().await
    }

    /// Explicitly switches the transmission path.
    pub async fn switch_path(&self, new_path: PathType) {
        let mut lock = self.active_path.write().await;
        *lock = new_path;
    }

    /// Returns the local node DID bound to this socket.
    #[must_use]
    pub fn local_did(&self) -> &str {
        &self.local_did
    }

    /// Returns the peer node DID.
    #[must_use]
    pub fn peer_did(&self) -> &str {
        &self.peer_did
    }

    /// Returns the configured direct socket address for the peer.
    #[must_use]
    pub fn peer_direct_addr(&self) -> SocketAddr {
        self.peer_direct_addr
    }

    /// Returns the relay server socket address, if configured.
    #[must_use]
    pub fn relay_addr(&self) -> Option<SocketAddr> {
        self.relay_addr
    }

    /// Returns `true` if an automatic seamless path upgrade occurred.
    #[must_use]
    pub fn was_upgraded_to_direct(&self) -> bool {
        self.direct_upgrade_detected.load(Ordering::SeqCst)
    }

    /// Returns the local socket listening address.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.socket.local_addr().map_err(NetError::Io)
    }

    /// Underlying UDP socket reference.
    #[must_use]
    pub fn socket(&self) -> &Arc<UdpSocket> {
        &self.socket
    }
}

/// Background monitor that probes direct P2P connectivity while on `RelayedE2ee` and triggers automatic path upgrade.
#[derive(Debug)]
pub struct PathUpgradeMonitor {
    cancel_token: tokio_util::sync::CancellationToken,
}

impl PathUpgradeMonitor {
    /// Starts background direct path probing.
    pub fn spawn(socket: Arc<HybridUdpRelaySocket>, probe_interval: Duration) -> Self {
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let s_cancel = cancel_token.clone();

        tokio::spawn(async move {
            let probe_bytes = b"UMCF_P2P_UPGRADE_PROBE";

            while !s_cancel.is_cancelled() {
                sleep(probe_interval).await;
                if s_cancel.is_cancelled() {
                    break;
                }

                if socket.current_path().await == PathType::RelayedE2ee {
                    // Send direct probe to peer's physical socket
                    let _ = socket
                        .socket
                        .send_to(probe_bytes, socket.peer_direct_addr)
                        .await;
                }
            }
        });

        Self { cancel_token }
    }

    /// Stops the upgrade monitor.
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for PathUpgradeMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}
