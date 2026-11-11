// auth.rs
// Fine-grained Scoped Capability Tokens & Token-Bucket Rate Limiter.
// Enforces principle of least privilege across all UMCF API Gateway routes.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::error::GatewayError;

static KEY_NONCE: AtomicU64 = AtomicU64::new(1);

/// Fine-grained capability scopes for API access authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityScope {
    /// Read-only access to cluster topology, nodes, and telemetry.
    #[serde(rename = "cluster:read")]
    ClusterRead,
    /// Administrative cluster control (join, drain, pair, configure).
    #[serde(rename = "cluster:admin")]
    ClusterAdmin,
    /// Submit new compute/WASM/media/GPU/AI jobs.
    #[serde(rename = "jobs:submit")]
    JobsSubmit,
    /// Query job status, logs, and progress events.
    #[serde(rename = "jobs:read")]
    JobsRead,
    /// Execute prompt inference and token generation against AI models.
    #[serde(rename = "ai:inference")]
    AiInference,
    /// Dispatch distributed GPU shaders and tile rendering jobs.
    #[serde(rename = "gpu:render")]
    GpuRender,
    /// Submit segment-parallel media transcoding tasks.
    #[serde(rename = "media:transcode")]
    MediaTranscode,
    /// Read content-addressed chunks, objects, and manifests.
    #[serde(rename = "storage:read")]
    StorageRead,
    /// Ingest, write, or pin objects in the content-addressed store.
    #[serde(rename = "storage:write")]
    StorageWrite,
    /// Connect to interactive remote desktop and game streaming sessions.
    #[serde(rename = "stream:connect")]
    StreamConnect,
    /// Unrestricted administrative access across all endpoints.
    #[serde(rename = "admin:full")]
    AdminFull,
}

impl CapabilityScope {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClusterRead => "cluster:read",
            Self::ClusterAdmin => "cluster:admin",
            Self::JobsSubmit => "jobs:submit",
            Self::JobsRead => "jobs:read",
            Self::AiInference => "ai:inference",
            Self::GpuRender => "gpu:render",
            Self::MediaTranscode => "media:transcode",
            Self::StorageRead => "storage:read",
            Self::StorageWrite => "storage:write",
            Self::StreamConnect => "stream:connect",
            Self::AdminFull => "admin:full",
        }
    }

    #[must_use]
    pub fn from_scope_str(s: &str) -> Option<Self> {
        match s {
            "cluster:read" => Some(Self::ClusterRead),
            "cluster:admin" => Some(Self::ClusterAdmin),
            "jobs:submit" => Some(Self::JobsSubmit),
            "jobs:read" => Some(Self::JobsRead),
            "ai:inference" => Some(Self::AiInference),
            "gpu:render" => Some(Self::GpuRender),
            "media:transcode" => Some(Self::MediaTranscode),
            "storage:read" => Some(Self::StorageRead),
            "storage:write" => Some(Self::StorageWrite),
            "stream:connect" => Some(Self::StreamConnect),
            "admin:full" => Some(Self::AdminFull),
            _ => None,
        }
    }
}

impl fmt::Display for CapabilityScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata record for an active or revoked API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    /// Unique public key identifier (e.g. `key_live_4a8b...`).
    pub key_id: String,
    /// BLAKE3 hash of the secret token material.
    pub key_hash: [u8; 32],
    /// Human-friendly descriptive label.
    pub label: String,
    /// Granted capability scopes.
    pub scopes: HashSet<CapabilityScope>,
    /// Sustained requests per second limit.
    pub rate_limit_rps: u32,
    /// Maximum burst bucket capacity.
    pub burst_allowance: u32,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at_ms: u64,
    /// Expiration timestamp in milliseconds (or None if perpetual).
    pub expires_at_ms: Option<u64>,
    /// Revocation flag.
    pub revoked: bool,
}

impl ApiKeyRecord {
    #[must_use]
    pub fn satisfies_scope(&self, required: CapabilityScope) -> bool {
        if self.scopes.contains(&CapabilityScope::AdminFull) {
            return true;
        }
        self.scopes.contains(&required)
    }
}

/// In-memory token bucket rate limiter with sub-millisecond refill precision.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub capacity: f64,
    pub tokens: f64,
    pub refill_rate_per_sec: f64,
    pub last_refill_ms: u64,
}

impl TokenBucket {
    #[must_use]
    pub fn new(capacity: u32, rate_per_sec: u32, now_ms: u64) -> Self {
        Self {
            capacity: f64::from(capacity),
            tokens: f64::from(capacity),
            refill_rate_per_sec: f64::from(rate_per_sec),
            last_refill_ms: now_ms,
        }
    }

    pub fn try_consume(&mut self, cost: f64, now_ms: u64) -> Result<(), u64> {
        if now_ms > self.last_refill_ms {
            let elapsed_sec = (now_ms - self.last_refill_ms) as f64 / 1000.0;
            self.tokens = (self.tokens + elapsed_sec * self.refill_rate_per_sec).min(self.capacity);
            self.last_refill_ms = now_ms;
        }

        if self.tokens >= cost {
            self.tokens -= cost;
            Ok(())
        } else {
            let deficit = cost - self.tokens;
            let wait_sec = deficit / self.refill_rate_per_sec;
            let wait_ms = (wait_sec * 1000.0).ceil() as u64;
            Err(wait_ms.max(1))
        }
    }
}

/// Thread-safe API Key and Token-Bucket Validator.
#[derive(Debug, Clone)]
pub struct ApiKeyValidator {
    keys_by_hash: Arc<RwLock<HashMap<[u8; 32], ApiKeyRecord>>>,
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl Default for ApiKeyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyValidator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys_by_hash: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Computes the standard BLAKE3 hash of a raw token string.
    #[must_use]
    pub fn hash_token(raw_token: &str) -> [u8; 32] {
        *blake3::hash(raw_token.as_bytes()).as_bytes()
    }

    /// Generates and registers a new scoped API key.
    ///
    /// Returns the raw secret token (to be displayed to the user once) and the stored record.
    pub async fn create_api_key(
        &self,
        label: impl Into<String>,
        scopes: HashSet<CapabilityScope>,
        rate_limit_rps: u32,
        burst_allowance: u32,
        expires_at_ms: Option<u64>,
    ) -> (String, ApiKeyRecord) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let nonce = KEY_NONCE.fetch_add(1, Ordering::Relaxed);
        let random_bytes: [u8; 24] = {
            let mut buf = [0u8; 24];
            let mut seed = [0u8; 16];
            seed[0..8].copy_from_slice(&now_ms.to_le_bytes());
            seed[8..16].copy_from_slice(&nonce.to_le_bytes());
            let entropy = blake3::hash(&seed);
            buf.copy_from_slice(&entropy.as_bytes()[0..24]);
            buf
        };

        let raw_token = format!("umcf_live_{}", hex::encode(random_bytes));
        let key_hash = Self::hash_token(&raw_token);
        let key_id = format!("key_{}", &raw_token[10..18]);

        let record = ApiKeyRecord {
            key_id: key_id.clone(),
            key_hash,
            label: label.into(),
            scopes,
            rate_limit_rps: rate_limit_rps.max(1),
            burst_allowance: burst_allowance.max(1),
            created_at_ms: now_ms,
            expires_at_ms,
            revoked: false,
        };

        let mut keys = self.keys_by_hash.write().await;
        keys.insert(key_hash, record.clone());

        let mut buckets = self.buckets.lock().await;
        buckets.insert(
            key_id,
            TokenBucket::new(record.burst_allowance, record.rate_limit_rps, now_ms),
        );

        (raw_token, record)
    }

    /// Registers a known token directly (useful for deterministic tests & seed configs).
    pub async fn register_raw_token(&self, raw_token: &str, record: ApiKeyRecord) {
        let key_hash = Self::hash_token(raw_token);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut keys = self.keys_by_hash.write().await;
        keys.insert(key_hash, record.clone());

        let mut buckets = self.buckets.lock().await;
        buckets.insert(
            record.key_id,
            TokenBucket::new(record.burst_allowance, record.rate_limit_rps, now_ms),
        );
    }

    /// Revokes an API key by its key_id.
    pub async fn revoke_key(&self, key_id: &str) -> bool {
        let mut keys = self.keys_by_hash.write().await;
        for record in keys.values_mut() {
            if record.key_id == key_id {
                record.revoked = true;
                return true;
            }
        }
        false
    }

    /// Validates an incoming Bearer token, checks scope authorization, and consumes rate limit.
    pub async fn validate_token(
        &self,
        raw_token: &str,
        required_scope: CapabilityScope,
    ) -> Result<ApiKeyRecord, GatewayError> {
        let cleaned_token = raw_token.trim().strip_prefix("Bearer ").unwrap_or(raw_token.trim());
        if cleaned_token.is_empty() {
            return Err(GatewayError::MissingToken);
        }

        let hash = Self::hash_token(cleaned_token);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let record = {
            let keys = self.keys_by_hash.read().await;
            keys.get(&hash).cloned().ok_or(GatewayError::InvalidToken)?
        };

        if record.revoked {
            return Err(GatewayError::TokenRevoked);
        }

        if let Some(exp) = record.expires_at_ms {
            if now_ms >= exp {
                return Err(GatewayError::TokenExpired);
            }
        }

        if !record.satisfies_scope(required_scope) {
            let granted_strings: Vec<String> = record.scopes.iter().map(|s| s.to_string()).collect();
            return Err(GatewayError::InsufficientScope {
                required: required_scope.to_string(),
                granted: granted_strings,
            });
        }

        // Rate limit check
        {
            let mut buckets = self.buckets.lock().await;
            let bucket = buckets.entry(record.key_id.clone()).or_insert_with(|| {
                TokenBucket::new(record.burst_allowance, record.rate_limit_rps, now_ms)
            });

            if let Err(retry_after_ms) = bucket.try_consume(1.0, now_ms) {
                return Err(GatewayError::RateLimited { retry_after_ms });
            }
        }

        Ok(record)
    }
}
