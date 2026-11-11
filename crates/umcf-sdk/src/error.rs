// error.rs
// Strongly-typed errors for the Rust UMCF SDK with RFC 9457 Problem Details support.

use thiserror::Error;
use umcf_api_gateway::Rfc9457ProblemDetails;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error ({status}): {problem:?}")]
    ApiProblem {
        status: u16,
        problem: Box<Rfc9457ProblemDetails>,
    },

    #[error("Authentication required (401 Unauthorized): {0}")]
    Unauthorized(String),

    #[error("Forbidden (403): insufficient capability scope. {0}")]
    Forbidden(String),

    #[error("Resource not found (404): {0}")]
    NotFound(String),

    #[error("Rate limit exceeded (429): retry after {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Validation error (400): {0}")]
    Validation(String),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Timeout waiting for job completion: {0}")]
    Timeout(String),
}
