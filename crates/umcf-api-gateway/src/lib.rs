// lib.rs
// UMCF Versioned API Gateway (`umcf-api-gateway`)
// Provides unified HTTP/REST, WebSocket, and OpenAI-compatible endpoints with scoped capability authorization.

pub mod auth;
pub mod error;
pub mod openapi;
pub mod problem;
pub mod router;
pub mod websocket;

pub use auth::{ApiKeyRecord, ApiKeyValidator, CapabilityScope, TokenBucket};
pub use error::GatewayError;
pub use openapi::{generate_openapi_spec, serve_docs_html};
pub use problem::{InvalidParam, Rfc9457ProblemDetails};
pub use router::{create_router, GatewayState};
pub use websocket::{handle_ws_stream, EventRingBuffer, JobEventRecord};

