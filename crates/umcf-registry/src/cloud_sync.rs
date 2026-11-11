//! Cloudflare Workers & Durable Objects Control Plane Client.
//! Handles signed delta synchronization, rendezvous peer discovery, and WebSocket presence.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use umcf_core::{NodeId, NodeRole};
use umcf_identity::NodeKeypair;

use crate::crdt::RegistryDelta;
use crate::error::RegistryError;
use crate::hlc::HlcTimestamp;

/// Request envelope for registering a node with the Cloudflare control plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNodeRequest {
    /// Node DID identifier (`did:umcf:...`).
    pub node_id: String,
    /// Operating role of the node (Coordinator, Worker, EdgeRelay).
    pub role: String,
    /// List of reachable P2P IP:port or QUIC multiaddrs.
    pub rendezvous_endpoints: Vec<String>,
}

/// Response returned when a node registers with the Cloudflare control plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    /// Status message (e.g., "registered").
    pub status: String,
    /// Cluster ID.
    pub cluster_id: String,
    /// Registered node ID.
    pub node_id: String,
    /// Active peer count in the cluster.
    pub active_peer_count: usize,
    /// Registered peer details.
    pub peer: CloudPeerRecord,
}

/// Peer record as maintained by the Cloudflare Durable Object coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPeerRecord {
    /// Node DID identifier.
    #[serde(rename = "nodeId")]
    pub node_id: String,
    /// Node role.
    pub role: String,
    /// Current node status ("Active", "Draining", "Offline").
    pub status: String,
    /// Active rendezvous endpoints for P2P connection negotiation.
    #[serde(rename = "rendezvousEndpoints", default)]
    pub rendezvous_endpoints: Vec<String>,
    /// Last seen timestamp in Unix milliseconds.
    #[serde(rename = "lastSeenMs", default)]
    pub last_seen_ms: u64,
}

/// Response returned from delta synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDeltaResponse {
    /// Whether the delta was accepted by the coordinator.
    pub accepted: bool,
    /// Current HLC timestamp of the coordinator.
    pub current_hlc: HlcTimestamp,
    /// Active peer count in the cluster.
    pub peer_count: usize,
}

/// Response returned from peer list queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersListResponse {
    /// Cluster ID.
    pub cluster_id: String,
    /// Total peer count.
    pub peer_count: usize,
    /// List of registered peer records.
    pub peers: Vec<CloudPeerRecord>,
}

/// Health check response from `/healthz`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPlaneHealth {
    /// Service identifier.
    pub service: String,
    /// Health status ("healthy").
    pub status: String,
    /// Protocol version.
    pub protocol: String,
    /// Current UTC timestamp.
    pub timestamp_utc: String,
}

#[derive(Deserialize)]
struct ErrorPayload {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Client for communicating with the UMCF Cloudflare Workers & Durable Objects Control Plane.
#[derive(Clone, Debug)]
pub struct CloudflareControlPlaneClient {
    base_url: String,
    cluster_id: String,
    keypair: Arc<NodeKeypair>,
    node_id: NodeId,
    client: reqwest::Client,
}

impl CloudflareControlPlaneClient {
    /// Creates a new `CloudflareControlPlaneClient`.
    ///
    /// # Errors
    /// Returns `RegistryError::InvalidUrl` if `base_url` is malformed.
    pub fn new(
        base_url: impl Into<String>,
        cluster_id: impl Into<String>,
        keypair: Arc<NodeKeypair>,
        node_id: NodeId,
    ) -> Result<Self, RegistryError> {
        let mut base = base_url.into();
        while base.ends_with('/') {
            base.pop();
        }

        if !base.starts_with("http://") && !base.starts_with("https://") {
            return Err(RegistryError::InvalidUrl(format!(
                "Base URL must start with http:// or https://: {base}"
            )));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(Self {
            base_url: base,
            cluster_id: cluster_id.into(),
            keypair,
            node_id,
            client,
        })
    }

    /// Returns the cluster ID.
    #[must_use]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Returns the client's node ID.
    #[must_use]
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Returns the base URL of the control plane.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Constructs the WebSocket URL for real-time presence and NAT signaling.
    ///
    /// # Errors
    /// Returns `RegistryError::InvalidUrl` if the base URL cannot be parsed.
    pub fn websocket_url(&self) -> Result<String, RegistryError> {
        let ws_base = if let Some(stripped) = self.base_url.strip_prefix("https://") {
            format!("wss://{stripped}")
        } else if let Some(stripped) = self.base_url.strip_prefix("http://") {
            format!("ws://{stripped}")
        } else {
            return Err(RegistryError::InvalidUrl(self.base_url.clone()));
        };

        Ok(format!(
            "{ws_base}/v1/clusters/{}/ws",
            urlencoding::encode(&self.cluster_id)
        ))
    }

    /// Generates authenticated cryptographic headers (`X-UMCF-DID`, `X-UMCF-Timestamp`, `X-UMCF-Signature`, `X-UMCF-Public-Key`).
    ///
    /// Format: `${method.to_uppercase()}:${pathname}:${timestampMs}:${bodySha256Hex}`
    ///
    /// # Errors
    /// Returns `RegistryError` if header construction or signing fails.
    pub fn sign_request(
        &self,
        method: &str,
        pathname: &str,
        body_bytes: &[u8],
        timestamp_ms: u64,
    ) -> Result<HeaderMap, RegistryError> {
        let mut hasher = Sha256::new();
        hasher.update(body_bytes);
        let body_hash_hex = hex::encode(hasher.finalize());

        let canonical = format!(
            "{}:{}:{}:{}",
            method.to_uppercase(),
            pathname,
            timestamp_ms,
            body_hash_hex
        );

        let signature_bytes = self.keypair.sign_raw(canonical.as_bytes());
        let signature_hex = hex::encode(signature_bytes);
        let pk_hex = hex::encode(self.keypair.ed25519_public_bytes());

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let did_hdr = HeaderName::from_static("x-umcf-did");
        let ts_hdr = HeaderName::from_static("x-umcf-timestamp");
        let sig_hdr = HeaderName::from_static("x-umcf-signature");
        let pk_hdr = HeaderName::from_static("x-umcf-public-key");

        headers.insert(
            did_hdr,
            HeaderValue::from_str(self.node_id.as_str())
                .map_err(|e| RegistryError::Serialization(e.to_string()))?,
        );
        headers.insert(
            ts_hdr,
            HeaderValue::from_str(&timestamp_ms.to_string())
                .map_err(|e| RegistryError::Serialization(e.to_string()))?,
        );
        headers.insert(
            sig_hdr,
            HeaderValue::from_str(&signature_hex)
                .map_err(|e| RegistryError::Serialization(e.to_string()))?,
        );
        headers.insert(
            pk_hdr,
            HeaderValue::from_str(&pk_hex)
                .map_err(|e| RegistryError::Serialization(e.to_string()))?,
        );

        Ok(headers)
    }

    /// Checks the health status of the control plane (`GET /healthz`).
    ///
    /// # Errors
    /// Returns `RegistryError::Network` or `RegistryError::CloudHttp` on failure.
    pub async fn check_health(&self) -> Result<ControlPlaneHealth, RegistryError> {
        let url = format!("{}/healthz", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(RegistryError::CloudHttp {
                status,
                message: text,
            });
        }

        resp.json::<ControlPlaneHealth>()
            .await
            .map_err(|e| RegistryError::Serialization(e.to_string()))
    }

    /// Registers this node with the cluster coordinator (`POST /v1/clusters/:id/register`).
    ///
    /// # Errors
    /// Returns `RegistryError::CloudHttp` or `RegistryError::Network` on failure.
    pub async fn register(
        &self,
        role: NodeRole,
        rendezvous_endpoints: Vec<String>,
    ) -> Result<RegisterResponse, RegistryError> {
        let path = format!(
            "/v1/clusters/{}/register",
            urlencoding::encode(&self.cluster_id)
        );
        let url = format!("{}{}", self.base_url, path);

        let req_body = RegisterNodeRequest {
            node_id: self.node_id.to_string(),
            role: format!("{role:?}"),
            rendezvous_endpoints,
        };

        let body_bytes = serde_json::to_vec(&req_body)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        let now_ms = current_time_ms();
        let headers = self.sign_request("POST", &path, &body_bytes, now_ms)?;

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let msg = if let Ok(p) = serde_json::from_str::<ErrorPayload>(&text) {
                if let Some(code) = p.code {
                    format!("{code}: {}", p.error.unwrap_or_default())
                } else {
                    p.error.unwrap_or(text)
                }
            } else {
                text
            };
            return Err(RegistryError::CloudHttp {
                status,
                message: msg,
            });
        }

        resp.json::<RegisterResponse>()
            .await
            .map_err(|e| RegistryError::Serialization(e.to_string()))
    }

    /// Pushes a CRDT delta update to the cluster coordinator (`POST /v1/clusters/:id/sync-delta`).
    ///
    /// # Errors
    /// Returns `RegistryError::CloudHttp` or `RegistryError::Network` on failure.
    pub async fn sync_delta(
        &self,
        delta: &RegistryDelta,
    ) -> Result<SyncDeltaResponse, RegistryError> {
        let path = format!(
            "/v1/clusters/{}/sync-delta",
            urlencoding::encode(&self.cluster_id)
        );
        let url = format!("{}{}", self.base_url, path);

        let body_bytes =
            serde_json::to_vec(delta).map_err(|e| RegistryError::Serialization(e.to_string()))?;

        let now_ms = current_time_ms();
        let headers = self.sign_request("POST", &path, &body_bytes, now_ms)?;

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let msg = if let Ok(p) = serde_json::from_str::<ErrorPayload>(&text) {
                if let Some(code) = p.code {
                    format!("{code}: {}", p.error.unwrap_or_default())
                } else {
                    p.error.unwrap_or(text)
                }
            } else {
                text
            };
            return Err(RegistryError::CloudHttp {
                status,
                message: msg,
            });
        }

        resp.json::<SyncDeltaResponse>()
            .await
            .map_err(|e| RegistryError::Serialization(e.to_string()))
    }

    /// Queries the list of registered peers in the cluster (`GET /v1/clusters/:id/peers`).
    ///
    /// # Errors
    /// Returns `RegistryError::CloudHttp` or `RegistryError::Network` on failure.
    pub async fn fetch_peers(&self) -> Result<Vec<CloudPeerRecord>, RegistryError> {
        let path = format!(
            "/v1/clusters/{}/peers",
            urlencoding::encode(&self.cluster_id)
        );
        let url = format!("{}{}", self.base_url, path);

        let now_ms = current_time_ms();
        let headers = self.sign_request("GET", &path, &[], now_ms)?;

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let msg = if let Ok(p) = serde_json::from_str::<ErrorPayload>(&text) {
                if let Some(code) = p.code {
                    format!("{code}: {}", p.error.unwrap_or_default())
                } else {
                    p.error.unwrap_or(text)
                }
            } else {
                text
            };
            return Err(RegistryError::CloudHttp {
                status,
                message: msg,
            });
        }

        let list = resp
            .json::<PeersListResponse>()
            .await
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        Ok(list.peers)
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char);
                }
                _ => {
                    use std::fmt::Write;
                    let _ = write!(encoded, "%{:02X}", b);
                }
            }
        }
        encoded
    }
}
