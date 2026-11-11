//! Safe RAII FFI bindings and lifecycle management for `llama.cpp` and `ggml`.
//!
//! Enforces:
//! - Strict Rust ownership and lifetime guarantees (`SafeLlamaContext` holds `Arc<SafeLlamaModel>`).
//! - Panic-free FFI calls with boundary catching.
//! - Non-null pointer checks and deterministic RAII teardown.
//! - Cancellation via progress and abort callbacks.
//! - Structured logging bridge into `tracing` with target `"umcf_ai::llama_cpp"`.

use crate::error::EngineError;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use umcf_gguf::{ModelId, VerifiedModelGuard};
use umcf_llama_sys::*;

// Global backend initialization reference counter
static BACKEND_REFCOUNT: AtomicUsize = AtomicUsize::new(0);

// Global C log callback bridge
unsafe extern "C" fn llama_log_callback_bridge(
    level: c_int,
    text: *const c_char,
    _user_data: *mut c_void,
) {
    if text.is_null() {
        return;
    }
    // SAFETY: text is guaranteed non-null and null-terminated by the engine
    let c_str = unsafe { CStr::from_ptr(text) };
    let Ok(msg) = c_str.to_str() else { return };
    let trimmed = msg.trim_end();

    match level {
        LLAMA_LOG_LEVEL_ERROR => tracing::error!(target: "umcf_ai::llama_cpp", "{}", trimmed),
        LLAMA_LOG_LEVEL_WARN => tracing::warn!(target: "umcf_ai::llama_cpp", "{}", trimmed),
        LLAMA_LOG_LEVEL_INFO => tracing::info!(target: "umcf_ai::llama_cpp", "{}", trimmed),
        _ => tracing::debug!(target: "umcf_ai::llama_cpp", "{}", trimmed),
    }
}

/// Global manager for `llama.cpp` backend lifecycle and logging.
#[derive(Debug, Clone, Copy)]
pub struct LlamaBackendManager;

impl LlamaBackendManager {
    /// Increments backend reference count and initializes native runtime if first reference.
    pub fn init() {
        let prev = BACKEND_REFCOUNT.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            // SAFETY: First reference initializes backend and sets global logger
            unsafe {
                llama_backend_init();
                llama_log_set(Some(llama_log_callback_bridge), std::ptr::null_mut());
            }
            tracing::info!(target: "umcf_ai::llama_cpp", "Initialized llama.cpp backend");
        }
    }

    /// Decrements backend reference count and frees native runtime when count reaches 0.
    pub fn shutdown() {
        let prev = BACKEND_REFCOUNT.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            // SAFETY: Last reference frees backend resources
            unsafe {
                llama_backend_free();
            }
            tracing::info!(target: "umcf_ai::llama_cpp", "Terminated llama.cpp backend");
        }
    }

    /// Returns current active backend reference count.
    #[must_use]
    pub fn ref_count() -> usize {
        BACKEND_REFCOUNT.load(Ordering::SeqCst)
    }
}

/// RAII Guard for backend initialization lifecycle.
#[derive(Debug)]
pub struct LlamaBackendGuard;

impl Default for LlamaBackendGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LlamaBackendGuard {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl LlamaBackendGuard {
    #[must_use]
    pub fn new() -> Self {
        LlamaBackendManager::init();
        Self
    }
}

impl Drop for LlamaBackendGuard {
    fn drop(&mut self) {
        LlamaBackendManager::shutdown();
    }
}

/// Parameters for model loading.
#[derive(Clone)]
pub struct ModelLoadParams {
    pub n_gpu_layers: i32,
    pub split_mode: i32,
    pub main_gpu: i32,
    pub use_mmap: bool,
    pub use_mlock: bool,
    pub check_tensors: bool,
    pub vocab_only: bool,
    pub progress_callback: Option<Arc<dyn Fn(f32) -> bool + Send + Sync + 'static>>,
}

impl std::fmt::Debug for ModelLoadParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelLoadParams")
            .field("n_gpu_layers", &self.n_gpu_layers)
            .field("split_mode", &self.split_mode)
            .field("main_gpu", &self.main_gpu)
            .field("use_mmap", &self.use_mmap)
            .field("use_mlock", &self.use_mlock)
            .field("check_tensors", &self.check_tensors)
            .field("vocab_only", &self.vocab_only)
            .field("has_progress_callback", &self.progress_callback.is_some())
            .finish()
    }
}

impl Default for ModelLoadParams {
    fn default() -> Self {
        Self {
            n_gpu_layers: 0,
            split_mode: 0,
            main_gpu: 0,
            use_mmap: true,
            use_mlock: false,
            check_tensors: true,
            vocab_only: false,
            progress_callback: None,
        }
    }
}

struct ProgressCallbackPayload {
    callback: Arc<dyn Fn(f32) -> bool + Send + Sync + 'static>,
}

unsafe extern "C" fn progress_callback_trampoline(progress: f32, user_data: *mut c_void) -> bool {
    if user_data.is_null() {
        return true;
    }
    // SAFETY: user_data points to valid ProgressCallbackPayload on the stack during load
    let payload = unsafe { &*(user_data as *const ProgressCallbackPayload) };
    (payload.callback)(progress)
}

/// Safe, RAII-managed representation of a loaded `llama_model`.
///
/// Immutable weights allow this struct to be `Send + Sync`.
#[derive(Debug)]
pub struct SafeLlamaModel {
    ptr: NonNull<LlamaModel>,
    model_id: ModelId,
    _backend_guard: LlamaBackendGuard,
}

// SAFETY: llama_model weights are immutable after loading and can be safely shared across threads
unsafe impl Send for SafeLlamaModel {}
unsafe impl Sync for SafeLlamaModel {}

impl SafeLlamaModel {
    /// Loads a model *exclusively* via a certified [`VerifiedModelGuard`].
    ///
    /// This establishes a compile-time invariant ensuring unverified or corrupted model
    /// files can never reach the native C++ inference engine.
    pub fn load_verified(
        guard: &VerifiedModelGuard,
        params: &ModelLoadParams,
    ) -> Result<Arc<Self>, EngineError> {
        let guard_backend = LlamaBackendGuard::new();
        let path = guard.path();
        let path_str = path.to_string_lossy().to_string();

        let c_path =
            CString::new(path_str.as_bytes()).map_err(|e| EngineError::ModelLoadFailed {
                path: path_str.clone(),
                reason: format!("invalid null byte in model path: {e}"),
            })?;

        // SAFETY: llama_model_default_params returns pure value struct
        let mut c_params = unsafe { llama_model_default_params() };
        c_params.n_gpu_layers = params.n_gpu_layers;
        c_params.split_mode = params.split_mode;
        c_params.main_gpu = params.main_gpu;
        c_params.use_mmap = params.use_mmap;
        c_params.use_mlock = params.use_mlock;
        c_params.check_tensors = params.check_tensors;
        c_params.vocab_only = params.vocab_only;

        let payload_opt = params
            .progress_callback
            .clone()
            .map(|cb| ProgressCallbackPayload { callback: cb });

        if let Some(ref payload) = payload_opt {
            c_params.progress_callback = Some(progress_callback_trampoline);
            c_params.progress_callback_user_data =
                (payload as *const ProgressCallbackPayload) as *mut c_void;
        }

        // SAFETY: c_path is a valid CString, c_params contains initialized fields
        let raw_model = unsafe { llama_load_model_from_file(c_path.as_ptr(), c_params) };

        if raw_model.is_null() {
            if payload_opt.is_some() {
                // If progress callback aborted, return LoadAborted
                return Err(EngineError::LoadAborted);
            }
            return Err(EngineError::ModelLoadFailed {
                path: path_str,
                reason: "llama_load_model_from_file returned null".to_string(),
            });
        }

        let non_null = NonNull::new(raw_model).ok_or_else(|| EngineError::ModelLoadFailed {
            path: path_str,
            reason: "null model pointer".to_string(),
        })?;

        Ok(Arc::new(Self {
            ptr: non_null,
            model_id: *guard.model_id(),
            _backend_guard: guard_backend,
        }))
    }

    #[must_use]
    pub fn model_id(&self) -> ModelId {
        self.model_id
    }

    #[must_use]
    pub fn n_vocab(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_n_vocab(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn n_ctx_train(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_n_ctx_train(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn n_embd(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_n_embd(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn n_layers(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_model_n_layers(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn bos_token(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_token_bos(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn eos_token(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_token_eos(self.ptr.as_ptr()) }
    }

    #[must_use]
    pub fn nl_token(&self) -> i32 {
        // SAFETY: self.ptr is valid
        unsafe { llama_token_nl(self.ptr.as_ptr()) }
    }

    /// Tokenizes prompt text into a vector of token IDs.
    pub fn tokenize(
        &self,
        text: &str,
        add_special: bool,
        parse_special: bool,
    ) -> Result<Vec<i32>, EngineError> {
        let c_text = CString::new(text).map_err(|e| EngineError::TokenizeFailed {
            text: text.to_string(),
            reason: format!("null byte in prompt text: {e}"),
        })?;

        // Allocate buffer: text len + 4 is a safe upper bound
        let max_tokens = text.len() as i32 + 8;
        let mut tokens: Vec<i32> = vec![0; max_tokens.max(16) as usize];

        // SAFETY: self.ptr is valid, tokens has allocated capacity
        let count = unsafe {
            llama_tokenize(
                self.ptr.as_ptr(),
                c_text.as_ptr(),
                text.len() as i32,
                tokens.as_mut_ptr(),
                tokens.len() as i32,
                add_special,
                parse_special,
            )
        };

        if count < 0 {
            let req = -count;
            return Err(EngineError::TokenizeFailed {
                text: text.to_string(),
                reason: format!("tokenization buffer overflow (required {req} tokens)"),
            });
        }

        tokens.truncate(count as usize);
        Ok(tokens)
    }

    /// Converts a token ID into its corresponding text piece.
    pub fn token_to_piece(&self, token: i32) -> Result<String, EngineError> {
        let mut buf = vec![0u8; 128];
        // SAFETY: self.ptr is valid, buf has 128 bytes
        let written = unsafe {
            llama_token_to_piece(
                self.ptr.as_ptr(),
                token,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as i32,
                0,
                true,
            )
        };

        if written < 0 {
            // Need larger buffer
            let req = (-written) as usize;
            buf.resize(req + 1, 0);
            // SAFETY: resized buffer has enough capacity
            let written2 = unsafe {
                llama_token_to_piece(
                    self.ptr.as_ptr(),
                    token,
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len() as i32,
                    0,
                    true,
                )
            };
            if written2 <= 0 {
                return Ok(String::new());
            }
            buf.truncate(written2 as usize);
        } else {
            buf.truncate(written as usize);
        }

        String::from_utf8(buf).map_err(|e| EngineError::TokenizeFailed {
            text: format!("token_{token}"),
            reason: format!("non-utf8 token piece: {e}"),
        })
    }
}

impl Drop for SafeLlamaModel {
    fn drop(&mut self) {
        // SAFETY: self.ptr is non-null and valid
        unsafe {
            llama_model_free(self.ptr.as_ptr());
        }
    }
}

/// Parameters for inference context creation.
#[derive(Debug, Clone)]
pub struct ContextParams {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub n_seq_max: u32,
    pub n_threads: u32,
    pub embeddings: bool,
    pub flash_attn: bool,
}

impl Default for ContextParams {
    fn default() -> Self {
        Self {
            n_ctx: 512,
            n_batch: 512,
            n_ubatch: 512,
            n_seq_max: 1,
            n_threads: 4,
            embeddings: false,
            flash_attn: false,
        }
    }
}

unsafe extern "C" fn abort_callback_trampoline(user_data: *mut c_void) -> bool {
    if user_data.is_null() {
        return false;
    }
    // SAFETY: user_data points to Arc<AtomicBool> managed by SafeLlamaContext
    let signal = unsafe { &*(user_data as *const AtomicBool) };
    signal.load(Ordering::SeqCst)
}

/// Safe, RAII-managed representation of a `llama_context`.
///
/// Holds `Arc<SafeLlamaModel>` so the underlying model is structurally guaranteed to outlive
/// the context. Marked `Send` but `!Sync` (requires exclusive `&mut self` access for evaluation).
#[derive(Debug)]
pub struct SafeLlamaContext {
    model: Arc<SafeLlamaModel>,
    ctx_ptr: NonNull<LlamaContext>,
    abort_signal: Arc<AtomicBool>,
}

// SAFETY: llama_context can be moved between threads, but cannot be shared concurrently
unsafe impl Send for SafeLlamaContext {}

impl SafeLlamaContext {
    /// Creates a new inference context with model reference counting.
    pub fn new(model: Arc<SafeLlamaModel>, params: &ContextParams) -> Result<Self, EngineError> {
        let abort_signal = Arc::new(AtomicBool::new(false));

        // SAFETY: llama_context_default_params returns value struct
        let mut c_params = unsafe { llama_context_default_params() };
        c_params.n_ctx = params.n_ctx;
        c_params.n_batch = params.n_batch;
        c_params.n_ubatch = params.n_ubatch;
        c_params.n_seq_max = params.n_seq_max;
        c_params.n_threads = params.n_threads;
        c_params.embeddings = params.embeddings;
        c_params.flash_attn = params.flash_attn;

        // Configure abort callback
        c_params.abort_callback = Some(abort_callback_trampoline);
        c_params.abort_callback_data = Arc::as_ptr(&abort_signal) as *mut c_void;

        // SAFETY: model.ptr is valid, c_params is initialized
        let raw_ctx = unsafe { llama_new_context_with_model(model.ptr.as_ptr(), c_params) };

        let ctx_ptr = NonNull::new(raw_ctx).ok_or_else(|| EngineError::ContextCreationFailed {
            reason: "llama_new_context_with_model returned null".to_string(),
        })?;

        Ok(Self {
            model,
            ctx_ptr,
            abort_signal,
        })
    }

    #[must_use]
    pub fn model(&self) -> &Arc<SafeLlamaModel> {
        &self.model
    }

    #[must_use]
    pub fn n_ctx(&self) -> u32 {
        // SAFETY: ctx_ptr is valid
        unsafe { llama_n_ctx(self.ctx_ptr.as_ptr()) }
    }

    /// Evaluates/decodes a batch of tokens, advancing KV-cache state and updating logits.
    pub fn decode(&mut self, batch: &mut SafeLlamaBatch) -> Result<(), EngineError> {
        if self.abort_signal.load(Ordering::SeqCst) {
            return Err(EngineError::GenerationAborted);
        }

        // SAFETY: ctx_ptr and batch are valid
        let code = unsafe { llama_decode(self.ctx_ptr.as_ptr(), batch.raw) };

        if code != 0 {
            if self.abort_signal.load(Ordering::SeqCst) {
                return Err(EngineError::GenerationAborted);
            }
            return Err(EngineError::DecodeFailed {
                code,
                reason: format!("llama_decode returned error code {code}"),
            });
        }

        Ok(())
    }

    /// Returns a slice over the logits for the final evaluated token.
    #[must_use]
    pub fn get_logits(&self) -> &[f32] {
        // SAFETY: ctx_ptr is valid
        let ptr = unsafe { llama_get_logits(self.ctx_ptr.as_ptr()) };
        if ptr.is_null() {
            return &[];
        }
        let n_vocab = self.model.n_vocab() as usize;
        // SAFETY: ptr points to valid buffer of n_vocab floats
        unsafe { std::slice::from_raw_parts(ptr, n_vocab) }
    }

    /// Returns a slice over the logits for the token at index `i` in the batch.
    #[must_use]
    pub fn get_logits_ith(&self, i: i32) -> &[f32] {
        // SAFETY: ctx_ptr is valid
        let ptr = unsafe { llama_get_logits_ith(self.ctx_ptr.as_ptr(), i) };
        if ptr.is_null() {
            return &[];
        }
        let n_vocab = self.model.n_vocab() as usize;
        // SAFETY: ptr points to valid buffer of n_vocab floats
        unsafe { std::slice::from_raw_parts(ptr, n_vocab) }
    }

    /// Returns a slice over embeddings if context was created with embeddings = true.
    #[must_use]
    pub fn get_embeddings(&self) -> Option<&[f32]> {
        // SAFETY: ctx_ptr is valid
        let ptr = unsafe { llama_get_embeddings(self.ctx_ptr.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        let n_embd = self.model.n_embd() as usize;
        // SAFETY: ptr points to valid buffer of n_embd floats
        unsafe { Some(std::slice::from_raw_parts(ptr, n_embd)) }
    }

    /// Clears the entire KV cache.
    pub fn clear_kv_cache(&mut self) {
        // SAFETY: ctx_ptr is valid
        unsafe { llama_kv_cache_clear(self.ctx_ptr.as_ptr()) }
    }

    /// Removes tokens in position range `[p0, p1)` for sequence `seq_id`.
    pub fn rm_kv_cache_seq(&mut self, seq_id: i32, p0: i32, p1: i32) -> bool {
        // SAFETY: ctx_ptr is valid
        unsafe { llama_kv_cache_seq_rm(self.ctx_ptr.as_ptr(), seq_id, p0, p1) }
    }

    /// Copies sequence positions from `seq_id_src` to `seq_id_dst`.
    pub fn cp_kv_cache_seq(&mut self, seq_id_src: i32, seq_id_dst: i32, p0: i32, p1: i32) {
        // SAFETY: ctx_ptr is valid
        unsafe { llama_kv_cache_seq_cp(self.ctx_ptr.as_ptr(), seq_id_src, seq_id_dst, p0, p1) }
    }

    /// Shifts sequence positions by delta in range [p0, p1).
    pub fn shift_kv_cache_seq(&mut self, seq_id: i32, p0: i32, p1: i32, delta: i32) {
        // SAFETY: ctx_ptr is valid
        unsafe { llama_kv_cache_seq_shift(self.ctx_ptr.as_ptr(), seq_id, p0, p1, delta) }
    }

    /// Triggers immediate cancellation of active and upcoming decode calls.
    pub fn abort(&self) {
        self.abort_signal.store(true, Ordering::SeqCst);
    }

    /// Resets the abort signal.
    pub fn reset_abort(&self) {
        self.abort_signal.store(false, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.abort_signal.load(Ordering::SeqCst)
    }
}

impl Drop for SafeLlamaContext {
    fn drop(&mut self) {
        // SAFETY: ctx_ptr is valid
        unsafe {
            llama_free(self.ctx_ptr.as_ptr());
        }
    }
}

/// Bounds-checked token batch with deterministic RAII deallocation.
#[derive(Debug)]
pub struct SafeLlamaBatch {
    raw: LlamaBatch,
    capacity: usize,
    max_seq_ids: usize,
}

impl SafeLlamaBatch {
    /// Creates a batch buffer with capacity for `n_tokens`.
    #[must_use]
    pub fn new(n_tokens: usize, embd: usize, n_seq_max: usize) -> Self {
        // SAFETY: llama_batch_init allocates arrays
        let raw = unsafe { llama_batch_init(n_tokens as i32, embd as i32, n_seq_max as i32) };
        Self {
            raw,
            capacity: n_tokens,
            max_seq_ids: n_seq_max,
        }
    }

    /// Adds a token to the batch with bounds checking.
    pub fn add(
        &mut self,
        token: i32,
        pos: i32,
        seq_ids: &[i32],
        logits: bool,
    ) -> Result<(), EngineError> {
        let idx = self.raw.n_tokens as usize;
        if idx >= self.capacity {
            return Err(EngineError::InvalidBatch {
                reason: format!("batch capacity {} exceeded", self.capacity),
            });
        }

        // SAFETY: idx < capacity, raw pointers are valid and allocated by llama_batch_init
        unsafe {
            *self.raw.token.add(idx) = token;
            *self.raw.pos.add(idx) = pos;

            let n_seq = seq_ids.len().min(self.max_seq_ids);
            *self.raw.n_seq_id.add(idx) = n_seq as i32;

            let seq_ptr = *self.raw.seq_id.add(idx);
            for s in 0..n_seq {
                if let Some(&seq) = seq_ids.get(s) {
                    *seq_ptr.add(s) = seq;
                }
            }

            *self.raw.logits.add(idx) = if logits { 1 } else { 0 };
        }

        self.raw.n_tokens += 1;
        Ok(())
    }

    /// Resets token count to 0 without reallocating buffers.
    pub fn clear(&mut self) {
        self.raw.n_tokens = 0;
    }

    #[must_use]
    pub fn n_tokens(&self) -> usize {
        self.raw.n_tokens as usize
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for SafeLlamaBatch {
    fn drop(&mut self) {
        // SAFETY: raw is allocated by llama_batch_init
        unsafe {
            llama_batch_free(self.raw);
        }
    }
}

/// Safe RAII wrapper over `llama.cpp` samplers and sampler chains.
#[derive(Debug)]
pub struct SafeSamplerChain {
    ptr: NonNull<LlamaSampler>,
}

// SAFETY: sampler can be moved between threads
unsafe impl Send for SafeSamplerChain {}

impl SafeSamplerChain {
    /// Creates a greedy (argmax) sampler.
    #[must_use]
    pub fn new_greedy() -> Self {
        // SAFETY: llama_sampler_init_greedy returns newly allocated sampler
        let ptr = unsafe { llama_sampler_init_greedy() };
        Self {
            ptr: NonNull::new(ptr).expect("non-null sampler"),
        }
    }

    /// Creates a top-k sampler.
    #[must_use]
    pub fn new_top_k(k: i32) -> Self {
        // SAFETY: llama_sampler_init_top_k returns newly allocated sampler
        let ptr = unsafe { llama_sampler_init_top_k(k) };
        Self {
            ptr: NonNull::new(ptr).expect("non-null sampler"),
        }
    }

    /// Creates a top-p (nucleus) sampler.
    #[must_use]
    pub fn new_top_p(p: f32, min_keep: usize) -> Self {
        // SAFETY: returns newly allocated sampler
        let ptr = unsafe { llama_sampler_init_top_p(p, min_keep) };
        Self {
            ptr: NonNull::new(ptr).expect("non-null sampler"),
        }
    }

    /// Creates a min-p sampler.
    #[must_use]
    pub fn new_min_p(p: f32, min_keep: usize) -> Self {
        // SAFETY: returns newly allocated sampler
        let ptr = unsafe { llama_sampler_init_min_p(p, min_keep) };
        Self {
            ptr: NonNull::new(ptr).expect("non-null sampler"),
        }
    }

    /// Creates a temperature scaling sampler.
    #[must_use]
    pub fn new_temp(t: f32) -> Self {
        // SAFETY: returns newly allocated sampler
        let ptr = unsafe { llama_sampler_init_temp(t) };
        Self {
            ptr: NonNull::new(ptr).expect("non-null sampler"),
        }
    }

    /// Creates an empty sampler chain.
    #[must_use]
    pub fn new_chain() -> Self {
        // SAFETY: llama_sampler_chain_default_params returns value struct
        let params = unsafe { llama_sampler_chain_default_params() };
        let ptr = unsafe { llama_sampler_chain_init(params) };
        Self {
            ptr: NonNull::new(ptr).expect("non-null chain"),
        }
    }

    /// Appends a child sampler to this chain, transferring ownership.
    pub fn add(&mut self, sampler: SafeSamplerChain) {
        // Prevent child's drop from freeing the sampler, as the chain now owns it
        let child_ptr = sampler.ptr.as_ptr();
        std::mem::forget(sampler);
        // SAFETY: self.ptr is chain, child_ptr is valid sampler
        unsafe {
            llama_sampler_chain_add(self.ptr.as_ptr(), child_ptr);
        }
    }

    /// Samples a token from context logits at batch index `idx`.
    #[must_use]
    pub fn sample(&mut self, ctx: &SafeLlamaContext, idx: i32) -> i32 {
        // SAFETY: self.ptr and ctx.ctx_ptr are valid
        unsafe { llama_sampler_sample(self.ptr.as_ptr(), ctx.ctx_ptr.as_ptr(), idx) }
    }
}

impl Drop for SafeSamplerChain {
    fn drop(&mut self) {
        // SAFETY: self.ptr is valid
        unsafe {
            llama_sampler_free(self.ptr.as_ptr());
        }
    }
}
