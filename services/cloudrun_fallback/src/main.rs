//! Google Cloud Run stateless fallback control plane for UMCF.
//! Exposes the exact same `/v1/clusters/:id/*` wire contract as `cf_control_plane`.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use umcf_identity::DOMAIN_SEPARATION_PREFIX;
use umcf_registry::{CloudPeerRecord, HlcTimestamp, RegistryDelta};

const MAX_REPLAY_SKEW_MS: u64 = 60_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterRecord {
    pub cluster_id: String,
    pub peers: HashMap<String, CloudPeerRecord>,
    pub deltas: HashMap<String, RegistryDelta>,
    pub current_hlc: HlcTimestamp,
}

impl ClusterRecord {
    pub fn new(cluster_id: String) -> Self {
        Self {
            cluster_id,
            peers: HashMap::new(),
            deltas: HashMap::new(),
            current_hlc: HlcTimestamp::new(current_time_ms(), 0, 0x4352_4642), // 'CRFB'
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct AppState {
    pub clusters: Arc<RwLock<HashMap<String, ClusterRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub service: String,
    pub status: String,
    pub protocol: String,
    pub timestamp_utc: String,
}

#[derive(Deserialize, Debug)]
pub struct RegisterPayload {
    pub node_id: String,
    pub role: String,
    #[serde(default)]
    pub rendezvous_endpoints: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct ErrorEnvelope {
    pub code: &'static str,
    pub error: String,
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Verifies Ed25519 signature headers and anti-replay freshness.
fn verify_request_auth(
    method: &str,
    pathname: &str,
    headers: &HeaderMap,
    body_bytes: &[u8],
) -> Result<String, (StatusCode, Json<ErrorEnvelope>)> {
    let get_header = |key: &str| -> Option<&str> { headers.get(key).and_then(|v| v.to_str().ok()) };

    let did = get_header("x-umcf-did").or_else(|| get_header("X-UMCF-DID"));
    let ts_str = get_header("x-umcf-timestamp").or_else(|| get_header("X-UMCF-Timestamp"));
    let sig_hex = get_header("x-umcf-signature").or_else(|| get_header("X-UMCF-Signature"));
    let pk_hex = get_header("x-umcf-public-key").or_else(|| get_header("X-UMCF-Public-Key"));

    let (did, ts_str, sig_hex) = match (did, ts_str, sig_hex) {
        (Some(d), Some(t), Some(s)) => (d, t, s),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "Missing mandatory auth headers: X-UMCF-DID, X-UMCF-Timestamp, X-UMCF-Signature".into(),
                }),
            ));
        }
    };

    let ts_ms = ts_str.parse::<u64>().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorEnvelope {
                code: "E2001",
                error: "Invalid X-UMCF-Timestamp format; must be integer milliseconds".into(),
            }),
        )
    })?;

    let now_ms = current_time_ms();
    let drift = now_ms.abs_diff(ts_ms);
    if drift > MAX_REPLAY_SKEW_MS {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorEnvelope {
                code: "E2001",
                error: format!(
                    "Timestamp expired: drift {drift} ms exceeds limit {MAX_REPLAY_SKEW_MS} ms"
                ),
            }),
        ));
    }

    let pk_bytes: [u8; 32] = if let Some(hex_str) = pk_hex {
        let dec = hex::decode(hex_str).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "Invalid hex in X-UMCF-Public-Key".into(),
                }),
            )
        })?;
        dec.try_into().map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "X-UMCF-Public-Key must be 32 bytes".into(),
                }),
            )
        })?
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorEnvelope {
                code: "E2001",
                error: "X-UMCF-Public-Key header required for verification".into(),
            }),
        ));
    };

    let sig_bytes: [u8; 64] = hex::decode(sig_hex)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "Invalid hex in X-UMCF-Signature".into(),
                }),
            )
        })?
        .try_into()
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "X-UMCF-Signature must be 64 bytes".into(),
                }),
            )
        })?;

    let mut hasher = Sha256::new();
    hasher.update(body_bytes);
    let body_hash_hex = hex::encode(hasher.finalize());

    let canonical = format!(
        "{}:{}:{}:{}",
        method.to_uppercase(),
        pathname,
        ts_ms,
        body_hash_hex
    );

    let sig = Signature::from_bytes(&sig_bytes);
    let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorEnvelope {
                code: "E2001",
                error: "Invalid Ed25519 public key bytes".into(),
            }),
        )
    })?;

    // Try raw canonical first, then domain-separated payload
    let verify_raw = vk.verify_strict(canonical.as_bytes(), &sig);
    if verify_raw.is_err() {
        let mut domain_payload =
            Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + canonical.len());
        domain_payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        domain_payload.extend_from_slice(canonical.as_bytes());

        if vk.verify_strict(&domain_payload, &sig).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    code: "E2001",
                    error: "Ed25519 signature verification failed".into(),
                }),
            ));
        }
    }

    Ok(did.to_string())
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "cloudrun_fallback".into(),
        status: "healthy".into(),
        protocol: "1.0.0".into(),
        timestamp_utc: "2026-11-10T11:45:00Z".into(),
    })
}

async fn register_handler(
    Path(cluster_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorEnvelope>)> {
    let path = format!("/v1/clusters/{cluster_id}/register");
    let did = verify_request_auth("POST", &path, &headers, &body)?;

    let payload: RegisterPayload = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope {
                code: "E4000",
                error: format!("Invalid register payload: {e}"),
            }),
        )
    })?;

    let now_ms = current_time_ms();
    let peer = CloudPeerRecord {
        node_id: payload.node_id.clone(),
        role: payload.role,
        status: "Active".into(),
        rendezvous_endpoints: payload.rendezvous_endpoints,
        last_seen_ms: now_ms,
    };

    let mut clusters = state.clusters.write().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorEnvelope {
                code: "E5000",
                error: "Cluster lock poisoned".into(),
            }),
        )
    })?;

    let cluster = clusters
        .entry(cluster_id.clone())
        .or_insert_with(|| ClusterRecord::new(cluster_id.clone()));

    cluster.peers.insert(did.clone(), peer.clone());
    let active_peer_count = cluster.peers.len();

    let resp = serde_json::json!({
        "status": "registered",
        "cluster_id": cluster_id,
        "node_id": did,
        "active_peer_count": active_peer_count,
        "peer": peer
    });

    Ok(Json(resp).into_response())
}

async fn sync_delta_handler(
    Path(cluster_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorEnvelope>)> {
    let path = format!("/v1/clusters/{cluster_id}/sync-delta");
    let _did = verify_request_auth("POST", &path, &headers, &body)?;

    let delta: RegistryDelta = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope {
                code: "E4000",
                error: format!("Invalid delta payload: {e}"),
            }),
        )
    })?;

    let now_ms = current_time_ms();
    let mut clusters = state.clusters.write().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorEnvelope {
                code: "E5000",
                error: "Cluster lock poisoned".into(),
            }),
        )
    })?;

    let cluster = clusters
        .entry(cluster_id.clone())
        .or_insert_with(|| ClusterRecord::new(cluster_id.clone()));

    // Monotonic HLC update
    let remote_wall = delta.delta_hlc.wall_ms;
    let remote_logical = delta.delta_hlc.logical;
    let local_wall = cluster.current_hlc.wall_ms;

    let max_wall = local_wall.max(remote_wall).max(now_ms);
    if max_wall == local_wall && max_wall == remote_wall {
        cluster.current_hlc.logical = cluster.current_hlc.logical.max(remote_logical) + 1;
    } else if max_wall == remote_wall {
        cluster.current_hlc.wall_ms = max_wall;
        cluster.current_hlc.logical = remote_logical + 1;
    } else {
        cluster.current_hlc.wall_ms = max_wall;
        cluster.current_hlc.logical += 1;
    }

    let delta_key = format!(
        "{}:{}_{}",
        delta.sender_node_id, delta.delta_hlc.wall_ms, delta.delta_hlc.logical
    );
    cluster.deltas.insert(delta_key, delta);

    let resp = serde_json::json!({
        "accepted": true,
        "current_hlc": cluster.current_hlc,
        "peer_count": cluster.peers.len()
    });

    Ok(Json(resp).into_response())
}

async fn peers_handler(
    Path(cluster_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorEnvelope>)> {
    let path = format!("/v1/clusters/{cluster_id}/peers");
    let _did = verify_request_auth("GET", &path, &headers, &body)?;

    let clusters = state.clusters.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorEnvelope {
                code: "E5000",
                error: "Cluster lock poisoned".into(),
            }),
        )
    })?;

    let (peer_count, peers) = if let Some(c) = clusters.get(&cluster_id) {
        let list: Vec<CloudPeerRecord> = c.peers.values().cloned().collect();
        (list.len(), list)
    } else {
        (0, Vec::new())
    };

    let resp = serde_json::json!({
        "cluster_id": cluster_id,
        "peer_count": peer_count,
        "peers": peers
    });

    Ok(Json(resp).into_response())
}

/// Builds the Axum router for the Cloud Run fallback control plane.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health_handler))
        .route("/v1/clusters/:id/register", post(register_handler))
        .route("/v1/clusters/:id/sync-delta", post(sync_delta_handler))
        .route("/v1/clusters/:id/peers", get(peers_handler))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let state = AppState::default();
    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("UMCF Cloud Run Fallback Control Plane listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");
    axum::serve(listener, app).await.expect("serve axum");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use umcf_core::NodeRole;
    use umcf_identity::NodeKeypair;
    use umcf_registry::CloudflareControlPlaneClient;

    #[tokio::test]
    async fn unit_cloudrun_fallback_healthz() {
        let state = AppState::default();
        let app = create_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind free port");
        let local_addr = listener.local_addr().expect("local addr");

        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let url = format!("http://{local_addr}/healthz");
        let resp = reqwest::get(&url).await.expect("health req");
        assert_eq!(resp.status(), StatusCode::OK);
        let body: HealthResponse = resp.json().await.expect("json body");
        assert_eq!(body.service, "cloudrun_fallback");
        assert_eq!(body.status, "healthy");
    }

    #[tokio::test]
    async fn integration_cloudrun_fallback_wire_contract_parity_with_cf() {
        let state = AppState::default();
        let app = create_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind free port");
        let local_addr = listener.local_addr().expect("local addr");

        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let keypair = Arc::new(NodeKeypair::generate());
        let node_id = keypair.node_id();
        let cluster_id = "test-cluster-cloudrun-parity";

        let client = CloudflareControlPlaneClient::new(
            format!("http://{local_addr}"),
            cluster_id,
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("client creation");

        // 1. Healthcheck
        let health = client.check_health().await.expect("health check");
        assert_eq!(health.status, "healthy");
        assert_eq!(health.service, "cloudrun_fallback");

        // 2. Register
        let reg = client
            .register(NodeRole::Worker, vec!["10.0.0.1:9090".into()])
            .await
            .expect("register peer");
        assert_eq!(reg.status, "registered");
        assert_eq!(reg.node_id, node_id.as_str());
        assert_eq!(reg.active_peer_count, 1);

        // 3. Sync Delta
        let delta = RegistryDelta {
            cluster_id: cluster_id.to_string(),
            sender_node_id: node_id.clone(),
            delta_hlc: HlcTimestamp::new(1_700_000_100_000, 2, 0x9999),
            nodes: vec![],
            capabilities: vec![],
            consent: vec![],
            crl_entries: vec![],
        };
        let sync = client.sync_delta(&delta).await.expect("sync delta");
        assert!(sync.accepted);
        assert_eq!(sync.peer_count, 1);

        // 4. Fetch Peers
        let peers = client.fetch_peers().await.expect("fetch peers");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, node_id.as_str());
        assert_eq!(peers[0].role, "Worker");
    }
}
