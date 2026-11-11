//! Universal Mobile Compute Fabric (UMCF) AI Inference Engine.
//!
//! Provides safe RAII wrappers over `llama.cpp` and `ggml` backend dispatch.

pub mod backend_loader;
pub mod backend_selector;
pub mod distributed_rpc;
pub mod engine;
pub mod error;
pub mod ffi_bridge;
pub mod kv_cache;
pub mod openai_api;
pub mod partitioner;
pub mod private_enclave;

pub use backend_loader::{DynamicBackendLoader, GgmlBackendDescriptor};
pub use backend_selector::{
    BackendExecutionPlan, BackendKind, CpuBackend, CpuIsaFlags, FallbackExecutor, FallbackTier,
    GpuBackend, HardwareBackendSelector,
};
pub use distributed_rpc::{
    ActivationCodec, ActivationDtype, DistributedPipelineCoordinator, ForwardActivationPacket,
    ForwardActivationResponse, PipelineStageWorker, RpcCapabilityToken,
};
pub use engine::{InferenceEngine, SamplingConfig, StreamEvent, Utf8TokenAssembler};
pub use error::EngineError;
pub use ffi_bridge::{
    ContextParams, LlamaBackendGuard, LlamaBackendManager, ModelLoadParams, SafeLlamaBatch,
    SafeLlamaContext, SafeLlamaModel, SafeSamplerChain,
};
pub use kv_cache::{KvCacheManager, KvSlot, SerializedKvSlot};
pub use openai_api::{
    cosine_similarity, l2_normalize, ChatChoice, ChatCompletionChunk, ChatCompletionRequest,
    ChatCompletionResponse, ChatMessage, ChatTemplateRenderer, EmbeddingObject, EmbeddingRequest,
    EmbeddingResponse, LocalOpenAiEngine, ModelListResponse, ModelObject, ResponseFormat, Usage,
};
pub use partitioner::{
    CandidateComputeNode, DistributedLayerPartitioner, DistributedPartitionPlan,
    LayerPartitionStage, NetworkLinkMatrix, PartitionMigrationPlan, RebalanceDetector, TrustTier,
};
pub use private_enclave::{
    redact_enclave_text, EphemeralInferenceSession, PrivacyEnclaveMode, SecretPromptBuffer,
};
