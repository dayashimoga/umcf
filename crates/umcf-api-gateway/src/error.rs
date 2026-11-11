// error.rs
// Gateway error types with automatic RFC 9457 Problem Details conversion.

use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::problem::{InvalidParam, Rfc9457ProblemDetails};

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("Missing Authorization header or Bearer token")]
    MissingToken,

    #[error("Invalid or unrecognized Bearer token")]
    InvalidToken,

    #[error("Token has expired")]
    TokenExpired,

    #[error("Token has been revoked")]
    TokenRevoked,

    #[error("Insufficient capability scope: required '{required}', granted [{granted:?}]")]
    InsufficientScope {
        required: String,
        granted: Vec<String>,
    },

    #[error("Rate limit exceeded; retry after {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Resource not found: {resource} ({id})")]
    NotFound { resource: String, id: String },

    #[error("Validation failed: {detail}")]
    Validation {
        detail: String,
        invalid_params: Vec<InvalidParam>,
    },

    #[error("Internal gateway error: {0}")]
    Internal(String),

    #[error("Storage engine error: {0}")]
    Storage(String),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let problem = match self {
            Self::MissingToken => {
                Rfc9457ProblemDetails::unauthorized("Bearer token required in Authorization header.")
            }
            Self::InvalidToken => {
                Rfc9457ProblemDetails::unauthorized("The provided Bearer token is invalid or unrecognized.")
            }
            Self::TokenExpired => {
                let mut p = Rfc9457ProblemDetails::unauthorized("The provided Bearer token has expired.");
                p.error_code = "UMCF_AUTH_TOKEN_EXPIRED".to_string();
                p
            }
            Self::TokenRevoked => {
                let mut p = Rfc9457ProblemDetails::unauthorized("The provided Bearer token has been revoked.");
                p.error_code = "UMCF_AUTH_TOKEN_REVOKED".to_string();
                p
            }
            Self::InsufficientScope { required, granted } => {
                Rfc9457ProblemDetails::forbidden(&required, &granted)
            }
            Self::RateLimited { retry_after_ms } => {
                Rfc9457ProblemDetails::rate_limited(retry_after_ms)
            }
            Self::NotFound { resource, id } => {
                Rfc9457ProblemDetails::not_found(&resource, &id)
            }
            Self::Validation { detail, invalid_params } => {
                Rfc9457ProblemDetails::validation_error(detail, invalid_params)
            }
            Self::Internal(msg) => Rfc9457ProblemDetails {
                error_type: "https://umcf.dev/errors/internal".to_string(),
                title: "Internal Server Error".to_string(),
                status: 500,
                detail: msg,
                instance: None,
                error_code: "UMCF_INTERNAL_ERROR".to_string(),
                invalid_params: Vec::new(),
            },
            Self::Storage(msg) => Rfc9457ProblemDetails {
                error_type: "https://umcf.dev/errors/storage".to_string(),
                status: 500,
                title: "Storage Engine Error".to_string(),
                detail: msg,
                instance: None,
                error_code: "UMCF_STORAGE_ERROR".to_string(),
                invalid_params: Vec::new(),
            },
        };

        problem.into_response()
    }
}
