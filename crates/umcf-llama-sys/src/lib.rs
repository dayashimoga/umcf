//! Low-level C/C++ FFI bindings to `llama.cpp` and `ggml`.
//!
//! Provides raw types and C symbols with strictly audited safety invariants.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

pub const LLAMA_LOG_LEVEL_ERROR: c_int = 1;
pub const LLAMA_LOG_LEVEL_WARN: c_int = 2;
pub const LLAMA_LOG_LEVEL_INFO: c_int = 3;
pub const LLAMA_LOG_LEVEL_DEBUG: c_int = 4;

pub type LlamaToken = i32;
pub type LlamaPos = i32;
pub type LlamaSeqId = i32;

/// Opaque model pointer.
#[repr(C)]
#[derive(Debug)]
pub struct LlamaModel {
    _unused: [u8; 0],
}

/// Opaque context pointer.
#[repr(C)]
#[derive(Debug)]
pub struct LlamaContext {
    _unused: [u8; 0],
}

/// Opaque sampler pointer.
#[repr(C)]
#[derive(Debug)]
pub struct LlamaSampler {
    _unused: [u8; 0],
}

pub type LlamaProgressCallback =
    Option<unsafe extern "C" fn(progress: f32, user_data: *mut c_void) -> bool>;
pub type GgmlAbortCallback = Option<unsafe extern "C" fn(user_data: *mut c_void) -> bool>;
pub type LlamaLogCallback =
    Option<unsafe extern "C" fn(level: c_int, text: *const c_char, user_data: *mut c_void)>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaModelParams {
    pub n_gpu_layers: i32,
    pub split_mode: i32,
    pub main_gpu: i32,
    pub tensor_split: *const f32,
    pub progress_callback: LlamaProgressCallback,
    pub progress_callback_user_data: *mut c_void,
    pub kv_overrides: *const c_void,
    pub vocab_only: bool,
    pub use_mmap: bool,
    pub use_mlock: bool,
    pub check_tensors: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaContextParams {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub n_seq_max: u32,
    pub n_threads: u32,
    pub n_threads_batch: u32,
    pub rope_scaling_type: i32,
    pub pooling_type: i32,
    pub attention_type: i32,
    pub rope_freq_base: f32,
    pub rope_freq_scale: f32,
    pub yarn_ext_factor: f32,
    pub yarn_attn_factor: f32,
    pub yarn_beta_fast: f32,
    pub yarn_beta_slow: f32,
    pub yarn_orig_ctx: u32,
    pub defrag_thold: f32,
    pub embeddings: bool,
    pub offload_kqv: bool,
    pub flash_attn: bool,
    pub no_perf: bool,
    pub abort_callback: GgmlAbortCallback,
    pub abort_callback_data: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaBatch {
    pub n_tokens: i32,
    pub token: *mut LlamaToken,
    pub embd: *mut f32,
    pub pos: *mut LlamaPos,
    pub n_seq_id: *mut i32,
    pub seq_id: *mut *mut LlamaSeqId,
    pub logits: *mut i8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaSamplerChainParams {
    pub no_perf: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaTokenData {
    pub id: LlamaToken,
    pub logit: f32,
    pub p: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LlamaTokenDataArray {
    pub data: *mut LlamaTokenData,
    pub size: usize,
    pub selected: i64,
    pub sorted: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgmlBackendDevType {
    Cpu = 0,
    Gpu = 1,
    Accel = 2,
}

pub type GgmlBackendDevT = *mut c_void;
pub type GgmlBackendT = *mut c_void;

extern "C" {
    // Lifecycle
    pub fn llama_backend_init();
    pub fn llama_backend_free();
    pub fn llama_log_set(log_callback: LlamaLogCallback, user_data: *mut c_void);

    pub fn llama_model_default_params() -> LlamaModelParams;
    pub fn llama_context_default_params() -> LlamaContextParams;

    // Model operations
    pub fn llama_load_model_from_file(
        path_model: *const c_char,
        params: LlamaModelParams,
    ) -> *mut LlamaModel;
    pub fn llama_model_free(model: *mut LlamaModel);
    pub fn llama_free_model(model: *mut LlamaModel);

    // Context operations
    pub fn llama_new_context_with_model(
        model: *mut LlamaModel,
        params: LlamaContextParams,
    ) -> *mut LlamaContext;
    pub fn llama_init_from_model(
        model: *mut LlamaModel,
        params: LlamaContextParams,
    ) -> *mut LlamaContext;
    pub fn llama_free(ctx: *mut LlamaContext);

    // Batch operations
    pub fn llama_batch_init(n_tokens: i32, embd: i32, n_seq_max: i32) -> LlamaBatch;
    pub fn llama_batch_free(batch: LlamaBatch);

    // Tokenization
    pub fn llama_tokenize(
        model: *const LlamaModel,
        text: *const c_char,
        text_len: i32,
        tokens: *mut LlamaToken,
        n_tokens_max: i32,
        add_special: bool,
        parse_special: bool,
    ) -> i32;
    pub fn llama_token_to_piece(
        model: *const LlamaModel,
        token: LlamaToken,
        buf: *mut c_char,
        length: i32,
        lstrip: i32,
        special: bool,
    ) -> i32;
    pub fn llama_token_bos(model: *const LlamaModel) -> LlamaToken;
    pub fn llama_token_eos(model: *const LlamaModel) -> LlamaToken;
    pub fn llama_token_nl(model: *const LlamaModel) -> LlamaToken;

    // Model properties
    pub fn llama_n_vocab(model: *const LlamaModel) -> i32;
    pub fn llama_n_ctx_train(model: *const LlamaModel) -> i32;
    pub fn llama_n_embd(model: *const LlamaModel) -> i32;
    pub fn llama_model_n_layers(model: *const LlamaModel) -> i32;

    // Context properties
    pub fn llama_n_ctx(ctx: *const LlamaContext) -> u32;
    pub fn llama_n_batch(ctx: *const LlamaContext) -> u32;
    pub fn llama_n_ubatch(ctx: *const LlamaContext) -> u32;

    // Execution
    pub fn llama_decode(ctx: *mut LlamaContext, batch: LlamaBatch) -> i32;
    pub fn llama_get_logits(ctx: *mut LlamaContext) -> *mut f32;
    pub fn llama_get_logits_ith(ctx: *mut LlamaContext, i: i32) -> *mut f32;
    pub fn llama_get_embeddings(ctx: *mut LlamaContext) -> *mut f32;
    pub fn llama_get_embeddings_seq(ctx: *mut LlamaContext, seq_id: LlamaSeqId) -> *mut f32;

    // KV Cache
    pub fn llama_kv_cache_clear(ctx: *mut LlamaContext);
    pub fn llama_kv_cache_seq_rm(
        ctx: *mut LlamaContext,
        seq_id: LlamaSeqId,
        p0: LlamaPos,
        p1: LlamaPos,
    ) -> bool;
    pub fn llama_kv_cache_seq_cp(
        ctx: *mut LlamaContext,
        seq_id_src: LlamaSeqId,
        seq_id_dst: LlamaSeqId,
        p0: LlamaPos,
        p1: LlamaPos,
    );
    pub fn llama_kv_cache_seq_keep(ctx: *mut LlamaContext, seq_id: LlamaSeqId);
    pub fn llama_kv_cache_seq_shift(
        ctx: *mut LlamaContext,
        seq_id: LlamaSeqId,
        p0: LlamaPos,
        p1: LlamaPos,
        delta: LlamaPos,
    );

    // Samplers
    pub fn llama_sampler_chain_default_params() -> LlamaSamplerChainParams;
    pub fn llama_sampler_chain_init(params: LlamaSamplerChainParams) -> *mut LlamaSampler;
    pub fn llama_sampler_chain_add(chain: *mut LlamaSampler, smpl: *mut LlamaSampler);
    pub fn llama_sampler_init_greedy() -> *mut LlamaSampler;
    pub fn llama_sampler_init_top_k(k: i32) -> *mut LlamaSampler;
    pub fn llama_sampler_init_top_p(p: f32, min_keep: usize) -> *mut LlamaSampler;
    pub fn llama_sampler_init_min_p(p: f32, min_keep: usize) -> *mut LlamaSampler;
    pub fn llama_sampler_init_temp(t: f32) -> *mut LlamaSampler;
    pub fn llama_sampler_init_dist(seed: u32) -> *mut LlamaSampler;
    pub fn llama_sampler_sample(
        smpl: *mut LlamaSampler,
        ctx: *mut LlamaContext,
        idx: i32,
    ) -> LlamaToken;
    pub fn llama_sampler_free(smpl: *mut LlamaSampler);

    // GGML Backend Devices
    pub fn ggml_backend_load_all();
    pub fn ggml_backend_load_all_from_path(dir_path: *const c_char);
    pub fn ggml_backend_dev_count() -> usize;
    pub fn ggml_backend_dev_get(index: usize) -> GgmlBackendDevT;
    pub fn ggml_backend_dev_name(dev: GgmlBackendDevT) -> *const c_char;
    pub fn ggml_backend_dev_description(dev: GgmlBackendDevT) -> *const c_char;
    pub fn ggml_backend_dev_memory(dev: GgmlBackendDevT, free: *mut usize, total: *mut usize);
    pub fn ggml_backend_dev_type(dev: GgmlBackendDevT) -> GgmlBackendDevType;
}
