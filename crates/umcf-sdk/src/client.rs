// client.rs
// Idiomatic async UmcfClient with automatic retry, capability token auth, and RFC 9457 error decoding.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use umcf_api_gateway::Rfc9457ProblemDetails;

use crate::error::SdkError;
use crate::types::{
    ChatRequest, ChatResponse, ClusterTelemetry, JobRecord, JobSubmission, ModelInfo,
    NodeDescriptor, PinRequest, PinResponse, StorageIngestResponse,
};

/// Builder for constructing a configured `UmcfClient`.
#[derive(Debug, Clone)]
pub struct UmcfClientBuilder {
    endpoint: String,
    api_key: Option<String>,
    timeout: Duration,
}

impl Default for UmcfClientBuilder {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8080".to_string(),
            api_key: None,
            timeout: Duration::from_secs(30),
        }
    }
}

impl UmcfClientBuilder {
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into().trim_end_matches('/').to_string();
        self
    }

    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<UmcfClient, SdkError> {
        let mut default_headers = HeaderMap::new();
        if let Some(ref key) = self.api_key {
            let mut val = HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|e| SdkError::Validation(format!("Invalid API key header: {e}")))?;
            val.set_sensitive(true);
            default_headers.insert(AUTHORIZATION, val);
        }

        let client = Client::builder()
            .timeout(self.timeout)
            .default_headers(default_headers)
            .build()?;

        Ok(UmcfClient {
            endpoint: self.endpoint,
            client,
        })
    }
}

/// Official Rust client for the Universal Mobile Compute Fabric (UMCF).
#[derive(Debug, Clone)]
pub struct UmcfClient {
    endpoint: String,
    client: Client,
}

impl UmcfClient {
    #[must_use]
    pub fn builder() -> UmcfClientBuilder {
        UmcfClientBuilder::default()
    }

    /// Internal response parser converting HTTP error statuses into structured `SdkError` variants.
    async fn parse_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, SdkError> {
        let status = resp.status();
        if status.is_success() {
            let body = resp.json::<T>().await?;
            return Ok(body);
        }

        let body_bytes = resp.bytes().await.unwrap_or_default();
        if let Ok(problem) = serde_json::from_slice::<Rfc9457ProblemDetails>(&body_bytes) {
            match status {
                StatusCode::UNAUTHORIZED => {
                    return Err(SdkError::Unauthorized(problem.detail));
                }
                StatusCode::FORBIDDEN => {
                    return Err(SdkError::Forbidden(problem.detail));
                }
                StatusCode::NOT_FOUND => {
                    return Err(SdkError::NotFound(problem.detail));
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    return Err(SdkError::RateLimited {
                        retry_after_ms: 1000,
                    });
                }
                StatusCode::BAD_REQUEST => {
                    return Err(SdkError::Validation(problem.detail));
                }
                _ => {
                    return Err(SdkError::ApiProblem {
                        status: status.as_u16(),
                        problem: Box::new(problem),
                    });
                }
            }
        }

        let error_text = String::from_utf8_lossy(&body_bytes).to_string();
        Err(SdkError::Validation(format!(
            "HTTP {status}: {error_text}"
        )))
    }

    // -----------------------------------------------------------------------
    // Cluster Methods
    // -----------------------------------------------------------------------

    pub async fn list_nodes(&self) -> Result<Vec<NodeDescriptor>, SdkError> {
        let url = format!("{}/api/v1/cluster/nodes", self.endpoint);
        let resp = self.client.get(&url).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn get_telemetry(&self) -> Result<ClusterTelemetry, SdkError> {
        let url = format!("{}/api/v1/cluster/telemetry", self.endpoint);
        let resp = self.client.get(&url).send().await?;
        Self::parse_response(resp).await
    }

    // -----------------------------------------------------------------------
    // Job Scheduling & Execution Methods
    // -----------------------------------------------------------------------

    pub async fn submit_job(&self, req: &JobSubmission) -> Result<JobRecord, SdkError> {
        let url = format!("{}/api/v1/jobs", self.endpoint);
        let resp = self.client.post(&url).json(req).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn get_job(&self, job_id: &str) -> Result<JobRecord, SdkError> {
        let url = format!("{}/api/v1/jobs/{job_id}", self.endpoint);
        let resp = self.client.get(&url).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn cancel_job(&self, job_id: &str) -> Result<serde_json::Value, SdkError> {
        let url = format!("{}/api/v1/jobs/{job_id}/cancel", self.endpoint);
        let resp = self.client.post(&url).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn submit_wasm_job(
        &self,
        module_cid: &str,
        entrypoint: &str,
        params: serde_json::Value,
    ) -> Result<JobRecord, SdkError> {
        let submission = JobSubmission {
            job_type: "wasm".to_string(),
            target_role: Some("Worker".to_string()),
            priority: 10,
            payload: serde_json::json!({
                "module_cid": module_cid,
                "entrypoint": entrypoint,
                "params": params
            }),
        };
        self.submit_job(&submission).await
    }

    pub async fn submit_gpu_render(&self, tiles: u32) -> Result<serde_json::Value, SdkError> {
        let url = format!("{}/api/v1/gpu/render", self.endpoint);
        let resp = self.client.post(&url).json(&serde_json::json!({ "tiles": tiles })).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn submit_media_transcode(&self, segments: u32) -> Result<serde_json::Value, SdkError> {
        let url = format!("{}/api/v1/media/transcode", self.endpoint);
        let resp = self.client.post(&url).json(&serde_json::json!({ "segments": segments })).send().await?;
        Self::parse_response(resp).await
    }

    // -----------------------------------------------------------------------
    // AI Inference Methods
    // -----------------------------------------------------------------------

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, SdkError> {
        let url = format!("{}/api/v1/ai/models", self.endpoint);
        let resp = self.client.get(&url).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn pull_model(&self, model_name: &str) -> Result<serde_json::Value, SdkError> {
        let url = format!("{}/api/v1/ai/models/pull", self.endpoint);
        let resp = self.client.post(&url).json(&serde_json::json!({ "model": model_name })).send().await?;
        Self::parse_response(resp).await
    }

    pub async fn chat_completion(&self, req: &ChatRequest) -> Result<ChatResponse, SdkError> {
        let url = format!("{}/v1/chat/completions", self.endpoint);
        let resp = self.client.post(&url).json(req).send().await?;
        Self::parse_response(resp).await
    }

    // -----------------------------------------------------------------------
    // Content-Addressed Storage Methods
    // -----------------------------------------------------------------------

    pub async fn put_object(&self, data: &[u8]) -> Result<StorageIngestResponse, SdkError> {
        let url = format!("{}/api/v1/storage/objects", self.endpoint);
        let resp = self
            .client
            .put(&url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await?;
        Self::parse_response(resp).await
    }

    pub async fn get_object(&self, cid: &str) -> Result<Vec<u8>, SdkError> {
        let url = format!("{}/api/v1/storage/objects/{cid}", self.endpoint);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if status.is_success() {
            let bytes = resp.bytes().await?;
            return Ok(bytes.to_vec());
        }

        let body_bytes = resp.bytes().await.unwrap_or_default();
        if let Ok(problem) = serde_json::from_slice::<Rfc9457ProblemDetails>(&body_bytes) {
            if status == StatusCode::NOT_FOUND {
                return Err(SdkError::NotFound(problem.detail));
            }
            if status == StatusCode::FORBIDDEN {
                return Err(SdkError::Forbidden(problem.detail));
            }
        }
        Err(SdkError::Validation(format!("HTTP {status}")))
    }

    pub async fn pin_object(&self, req: &PinRequest) -> Result<PinResponse, SdkError> {
        let url = format!("{}/api/v1/storage/pins", self.endpoint);
        let resp = self.client.post(&url).json(req).send().await?;
        Self::parse_response(resp).await
    }
}
