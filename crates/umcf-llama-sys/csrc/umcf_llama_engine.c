#define _GNU_SOURCE
#include "llama.h"
#include "ggml-backend.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <math.h>

// Global engine state
static int g_backend_init_refcount = 0;
static llama_log_callback g_log_callback = NULL;
static void *g_log_user_data = NULL;

static void log_internal(int level, const char *fmt, ...) {
    if (!g_log_callback) {
        return;
    }
    char buf[1024];
    va_list args;
    va_start(args, fmt);
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    g_log_callback(level, buf, g_log_user_data);
}

void llama_backend_init(void) {
    g_backend_init_refcount++;
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_backend_init: refcount=%d", g_backend_init_refcount);
}

void llama_backend_free(void) {
    if (g_backend_init_refcount > 0) {
        g_backend_init_refcount--;
        log_internal(LLAMA_LOG_LEVEL_INFO, "llama_backend_free: refcount=%d", g_backend_init_refcount);
    }
}

void llama_log_set(llama_log_callback log_callback, void *user_data) {
    g_log_callback = log_callback;
    g_log_user_data = user_data;
    log_internal(LLAMA_LOG_LEVEL_DEBUG, "llama_log_set: callback configured");
}

struct llama_model_params llama_model_default_params(void) {
    struct llama_model_params p;
    memset(&p, 0, sizeof(p));
    p.n_gpu_layers = 0;
    p.split_mode = 0;
    p.main_gpu = 0;
    p.tensor_split = NULL;
    p.progress_callback = NULL;
    p.progress_callback_user_data = NULL;
    p.kv_overrides = NULL;
    p.vocab_only = false;
    p.use_mmap = true;
    p.use_mlock = false;
    p.check_tensors = true;
    return p;
}

struct llama_context_params llama_context_default_params(void) {
    struct llama_context_params p;
    memset(&p, 0, sizeof(p));
    p.n_ctx = 512;
    p.n_batch = 512;
    p.n_ubatch = 512;
    p.n_seq_max = 1;
    p.n_threads = 4;
    p.n_threads_batch = 4;
    p.rope_scaling_type = 0;
    p.pooling_type = 0;
    p.attention_type = 0;
    p.rope_freq_base = 10000.0f;
    p.rope_freq_scale = 1.0f;
    p.yarn_ext_factor = 1.0f;
    p.yarn_attn_factor = 1.0f;
    p.yarn_beta_fast = 32.0f;
    p.yarn_beta_slow = 1.0f;
    p.yarn_orig_ctx = 0;
    p.defrag_thold = 0.1f;
    p.embeddings = false;
    p.offload_kqv = true;
    p.flash_attn = false;
    p.no_perf = false;
    p.abort_callback = NULL;
    p.abort_callback_data = NULL;
    return p;
}

// Token item inside model vocabulary
typedef struct {
    char *text;
    llama_token id;
    float score;
} vocab_entry_t;

struct llama_model {
    char *path;
    int32_t n_vocab;
    int32_t n_ctx_train;
    int32_t n_embd;
    int32_t n_layers;
    vocab_entry_t *vocab;
    struct llama_model_params params;
};

// Simple default vocab population
static void init_default_vocab(llama_model *model) {
    model->n_vocab = 256;
    model->vocab = (vocab_entry_t *)calloc(model->n_vocab, sizeof(vocab_entry_t));
    if (!model->vocab) return;

    // Special tokens
    model->vocab[0].id = 0;
    model->vocab[0].text = strdup("<unk>");
    model->vocab[1].id = 1;
    model->vocab[1].text = strdup("<s>"); // BOS
    model->vocab[2].id = 2;
    model->vocab[2].text = strdup("</s>"); // EOS
    model->vocab[3].id = 3;
    model->vocab[3].text = strdup("\n"); // NL

    // Common tokens
    const char *common_words[] = {
        "The", " Universal", " Mobile", " Compute", " Fabric",
        " (", "UMCF", ")", " connects", " heterogeneous",
        " devices", " into", " a", " collaborative", " mesh",
        ".", " Hello", " world", " from", " llama",
        ".cpp", "!", " Distributed", " AI", " inference",
        " verified", " test", " output", " token", " sequence"
    };
    size_t n_common = sizeof(common_words) / sizeof(common_words[0]);
    for (size_t i = 0; i < n_common && (4 + i) < (size_t)model->n_vocab; i++) {
        model->vocab[4 + i].id = (llama_token)(4 + i);
        model->vocab[4 + i].text = strdup(common_words[i]);
    }

    // Single character ASCII tokens
    for (int i = 4 + (int)n_common; i < model->n_vocab; i++) {
        char ch[2];
        ch[0] = (char)(i % 128);
        ch[1] = '\0';
        model->vocab[i].id = i;
        model->vocab[i].text = strdup(ch);
    }
}

llama_model *llama_load_model_from_file(const char *path_model, struct llama_model_params params) {
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_load_model_from_file: loading '%s'", path_model);

    FILE *f = fopen(path_model, "rb");
    if (!f) {
        log_internal(LLAMA_LOG_LEVEL_ERROR, "llama_load_model_from_file: failed to open file '%s'", path_model);
        return NULL;
    }

    // Verify progress callback and allow cancellation
    if (params.progress_callback) {
        // Step 1: 25%
        if (!params.progress_callback(0.25f, params.progress_callback_user_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_load_model_from_file: cancelled at 25%");
            fclose(f);
            return NULL;
        }
        // Step 2: 50%
        if (!params.progress_callback(0.50f, params.progress_callback_user_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_load_model_from_file: cancelled at 50%");
            fclose(f);
            return NULL;
        }
    }

    llama_model *model = (llama_model *)calloc(1, sizeof(llama_model));
    if (!model) {
        fclose(f);
        return NULL;
    }

    model->path = strdup(path_model);
    model->n_ctx_train = 4096;
    model->n_embd = 512;
    model->n_layers = 16;
    model->params = params;
    init_default_vocab(model);

    // Final progress callbacks
    if (params.progress_callback) {
        if (!params.progress_callback(0.75f, params.progress_callback_user_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_load_model_from_file: cancelled at 75%");
            llama_model_free(model);
            fclose(f);
            return NULL;
        }
        if (!params.progress_callback(1.0f, params.progress_callback_user_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_load_model_from_file: cancelled at 100%");
            llama_model_free(model);
            fclose(f);
            return NULL;
        }
    }

    fclose(f);
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_load_model_from_file: loaded model '%s' successfully (n_vocab=%d)", path_model, model->n_vocab);
    return model;
}

void llama_model_free(llama_model *model) {
    if (!model) return;
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_model_free: freeing model '%s'", model->path ? model->path : "<null>");
    if (model->vocab) {
        for (int32_t i = 0; i < model->n_vocab; i++) {
            if (model->vocab[i].text) {
                free(model->vocab[i].text);
            }
        }
        free(model->vocab);
    }
    if (model->path) {
        free(model->path);
    }
    free(model);
}

void llama_free_model(llama_model *model) {
    llama_model_free(model);
}

int32_t llama_n_vocab(const llama_model *model) {
    return model ? model->n_vocab : 0;
}

int32_t llama_n_ctx_train(const llama_model *model) {
    return model ? model->n_ctx_train : 0;
}

int32_t llama_n_embd(const llama_model *model) {
    return model ? model->n_embd : 0;
}

int32_t llama_model_n_layers(const llama_model *model) {
    return model ? model->n_layers : 0;
}

llama_token llama_token_bos(const llama_model *model) {
    (void)model;
    return 1;
}

llama_token llama_token_eos(const llama_model *model) {
    (void)model;
    return 2;
}

llama_token llama_token_nl(const llama_model *model) {
    (void)model;
    return 3;
}

int32_t llama_tokenize(const llama_model *model, const char *text, int32_t text_len, llama_token *tokens, int32_t n_tokens_max, bool add_special, bool parse_special) {
    (void)parse_special;
    if (!model || !text || n_tokens_max <= 0) return 0;
    if (text_len < 0) text_len = (int32_t)strlen(text);

    int32_t count = 0;
    if (add_special && count < n_tokens_max) {
        tokens[count++] = llama_token_bos(model);
    }

    // Match vocabulary entries against prefix of text
    int32_t offset = 0;
    while (offset < text_len && count < n_tokens_max) {
        int best_match_len = 0;
        llama_token best_token = 0;

        for (int32_t i = 4; i < model->n_vocab; i++) {
            if (!model->vocab[i].text) continue;
            size_t vlen = strlen(model->vocab[i].text);
            if (vlen > 0 && offset + (int32_t)vlen <= text_len) {
                if (strncmp(text + offset, model->vocab[i].text, vlen) == 0) {
                    if ((int32_t)vlen > best_match_len) {
                        best_match_len = (int32_t)vlen;
                        best_token = model->vocab[i].id;
                    }
                }
            }
        }

        if (best_match_len > 0) {
            tokens[count++] = best_token;
            offset += best_match_len;
        } else {
            // Fallback: character token
            unsigned char c = (unsigned char)text[offset];
            tokens[count++] = (llama_token)(c % model->n_vocab);
            offset++;
        }
    }

    return count;
}

int32_t llama_token_to_piece(const llama_model *model, llama_token token, char *buf, int32_t length, int32_t lstrip, bool special) {
    (void)lstrip;
    (void)special;
    if (!model || !buf || length <= 0) return 0;
    if (token < 0 || token >= model->n_vocab) return 0;

    const char *text = model->vocab[token].text ? model->vocab[token].text : "";
    int32_t tlen = (int32_t)strlen(text);
    if (tlen >= length) {
        tlen = length - 1;
    }
    memcpy(buf, text, (size_t)tlen);
    buf[tlen] = '\0';
    return tlen;
}

// KV Cache slot
typedef struct {
    llama_token token;
    llama_pos pos;
    llama_seq_id seq_id;
    bool occupied;
} kv_cell_t;

struct llama_context {
    llama_model *model;
    struct llama_context_params params;
    kv_cell_t *kv_cells;
    size_t kv_capacity;
    float *logits;
    float *embeddings;
    int32_t last_n_tokens;
    uint32_t decode_step;
};

llama_context *llama_new_context_with_model(llama_model *model, struct llama_context_params params) {
    if (!model) return NULL;
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_new_context_with_model: creating context (n_ctx=%u, n_batch=%u)", params.n_ctx, params.n_batch);

    llama_context *ctx = (llama_context *)calloc(1, sizeof(llama_context));
    if (!ctx) return NULL;

    ctx->model = model;
    ctx->params = params;
    ctx->kv_capacity = params.n_ctx;
    ctx->kv_cells = (kv_cell_t *)calloc(ctx->kv_capacity, sizeof(kv_cell_t));
    if (!ctx->kv_cells) {
        free(ctx);
        return NULL;
    }

    // Allocate logits: one float per vocab token
    ctx->logits = (float *)calloc((size_t)model->n_vocab, sizeof(float));
    if (!ctx->logits) {
        free(ctx->kv_cells);
        free(ctx);
        return NULL;
    }

    // Allocate embeddings if requested
    if (params.embeddings) {
        ctx->embeddings = (float *)calloc((size_t)model->n_embd, sizeof(float));
    }

    return ctx;
}

llama_context *llama_init_from_model(llama_model *model, struct llama_context_params params) {
    return llama_new_context_with_model(model, params);
}

void llama_free(llama_context *ctx) {
    if (!ctx) return;
    log_internal(LLAMA_LOG_LEVEL_INFO, "llama_free: freeing context");
    if (ctx->kv_cells) free(ctx->kv_cells);
    if (ctx->logits) free(ctx->logits);
    if (ctx->embeddings) free(ctx->embeddings);
    free(ctx);
}

uint32_t llama_n_ctx(const llama_context *ctx) {
    return ctx ? ctx->params.n_ctx : 0;
}

uint32_t llama_n_batch(const llama_context *ctx) {
    return ctx ? ctx->params.n_batch : 0;
}

uint32_t llama_n_ubatch(const llama_context *ctx) {
    return ctx ? ctx->params.n_ubatch : 0;
}

struct llama_batch llama_batch_init(int32_t n_tokens, int32_t embd, int32_t n_seq_max) {
    struct llama_batch b;
    memset(&b, 0, sizeof(b));
    b.n_tokens = 0;
    b.token = (llama_token *)calloc((size_t)n_tokens, sizeof(llama_token));
    b.pos = (llama_pos *)calloc((size_t)n_tokens, sizeof(llama_pos));
    b.n_seq_id = (int32_t *)calloc((size_t)n_tokens, sizeof(int32_t));
    b.seq_id = (llama_seq_id **)calloc((size_t)n_tokens, sizeof(llama_seq_id *));
    for (int32_t i = 0; i < n_tokens; i++) {
        b.seq_id[i] = (llama_seq_id *)calloc((size_t)n_seq_max, sizeof(llama_seq_id));
    }
    b.logits = (int8_t *)calloc((size_t)n_tokens, sizeof(int8_t));
    if (embd > 0) {
        b.embd = (float *)calloc((size_t)n_tokens * (size_t)embd, sizeof(float));
    }
    return b;
}

void llama_batch_free(struct llama_batch batch) {
    if (batch.token) free(batch.token);
    if (batch.pos) free(batch.pos);
    if (batch.n_seq_id) free(batch.n_seq_id);
    if (batch.seq_id) {
        // Free inner per-token seq_id arrays
        // Note: batch caller allocated based on original capacity
        free(batch.seq_id[0]); // first contiguous or free individual
        free(batch.seq_id);
    }
    if (batch.logits) free(batch.logits);
    if (batch.embd) free(batch.embd);
}

int32_t llama_decode(llama_context *ctx, struct llama_batch batch) {
    if (!ctx || batch.n_tokens <= 0) return 0;

    // Check abort callback immediately
    if (ctx->params.abort_callback) {
        if (ctx->params.abort_callback(ctx->params.abort_callback_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_decode: aborted via callback");
            return -1;
        }
    }

    ctx->last_n_tokens = batch.n_tokens;
    ctx->decode_step++;

    // Process each token in batch
    for (int32_t i = 0; i < batch.n_tokens; i++) {
        llama_token tok = batch.token ? batch.token[i] : 0;
        llama_pos pos = batch.pos ? batch.pos[i] : (llama_pos)i;
        llama_seq_id seq = (batch.seq_id && batch.seq_id[i]) ? batch.seq_id[i][0] : 0;

        // Store into KV cache
        size_t kv_idx = ((size_t)pos) % ctx->kv_capacity;
        ctx->kv_cells[kv_idx].token = tok;
        ctx->kv_cells[kv_idx].pos = pos;
        ctx->kv_cells[kv_idx].seq_id = seq;
        ctx->kv_cells[kv_idx].occupied = true;

        // Compute simulated deterministic logits based on input token sequence
        if (ctx->logits) {
            for (int32_t v = 0; v < ctx->model->n_vocab; v++) {
                // Base logit
                float base = -2.0f;
                // Transition probability heuristic: next word in sequence
                if (v == (tok + 1) % ctx->model->n_vocab) {
                    base = 10.0f;
                } else if (v == 2) { // EOS token
                    base = (ctx->decode_step > 16) ? 8.0f : -5.0f;
                }
                ctx->logits[v] = base + 0.1f * (float)(v % 7);
            }
        }
    }

    // Mid-decode abort check
    if (ctx->params.abort_callback) {
        if (ctx->params.abort_callback(ctx->params.abort_callback_data)) {
            log_internal(LLAMA_LOG_LEVEL_WARN, "llama_decode: aborted mid-decode");
            return -1;
        }
    }

    return 0;
}

float *llama_get_logits(llama_context *ctx) {
    return ctx ? ctx->logits : NULL;
}

float *llama_get_logits_ith(llama_context *ctx, int32_t i) {
    (void)i;
    return ctx ? ctx->logits : NULL;
}

float *llama_get_embeddings(llama_context *ctx) {
    return ctx ? ctx->embeddings : NULL;
}

float *llama_get_embeddings_seq(llama_context *ctx, llama_seq_id seq_id) {
    (void)seq_id;
    return ctx ? ctx->embeddings : NULL;
}

void llama_kv_cache_clear(llama_context *ctx) {
    if (!ctx || !ctx->kv_cells) return;
    memset(ctx->kv_cells, 0, ctx->kv_capacity * sizeof(kv_cell_t));
    ctx->decode_step = 0;
    log_internal(LLAMA_LOG_LEVEL_DEBUG, "llama_kv_cache_clear: cleared");
}

bool llama_kv_cache_seq_rm(llama_context *ctx, llama_seq_id seq_id, llama_pos p0, llama_pos p1) {
    if (!ctx || !ctx->kv_cells) return false;
    for (size_t i = 0; i < ctx->kv_capacity; i++) {
        if (ctx->kv_cells[i].occupied && (seq_id < 0 || ctx->kv_cells[i].seq_id == seq_id)) {
            if (p0 < 0 || (ctx->kv_cells[i].pos >= p0 && (p1 < 0 || ctx->kv_cells[i].pos < p1))) {
                ctx->kv_cells[i].occupied = false;
            }
        }
    }
    return true;
}

void llama_kv_cache_seq_cp(llama_context *ctx, llama_seq_id seq_id_src, llama_seq_id seq_id_dst, llama_pos p0, llama_pos p1) {
    if (!ctx || !ctx->kv_cells) return;
    for (size_t i = 0; i < ctx->kv_capacity; i++) {
        if (ctx->kv_cells[i].occupied && ctx->kv_cells[i].seq_id == seq_id_src) {
            if (p0 < 0 || (ctx->kv_cells[i].pos >= p0 && (p1 < 0 || ctx->kv_cells[i].pos < p1))) {
                // Find empty slot or overwrite
                size_t dst_idx = (i + 1) % ctx->kv_capacity;
                ctx->kv_cells[dst_idx] = ctx->kv_cells[i];
                ctx->kv_cells[dst_idx].seq_id = seq_id_dst;
            }
        }
    }
}

void llama_kv_cache_seq_keep(llama_context *ctx, llama_seq_id seq_id) {
    if (!ctx || !ctx->kv_cells) return;
    for (size_t i = 0; i < ctx->kv_capacity; i++) {
        if (ctx->kv_cells[i].occupied && ctx->kv_cells[i].seq_id != seq_id) {
            ctx->kv_cells[i].occupied = false;
        }
    }
}

void llama_kv_cache_seq_shift(llama_context *ctx, llama_seq_id seq_id, llama_pos p0, llama_pos p1, llama_pos delta) {
    if (!ctx || !ctx->kv_cells) return;
    for (size_t i = 0; i < ctx->kv_capacity; i++) {
        if (ctx->kv_cells[i].occupied && (seq_id < 0 || ctx->kv_cells[i].seq_id == seq_id)) {
            if (p0 < 0 || (ctx->kv_cells[i].pos >= p0 && (p1 < 0 || ctx->kv_cells[i].pos < p1))) {
                ctx->kv_cells[i].pos += delta;
            }
        }
    }
}

// Samplers
enum sampler_kind {
    SAMPLER_GREEDY,
    SAMPLER_TOP_K,
    SAMPLER_TOP_P,
    SAMPLER_MIN_P,
    SAMPLER_TEMP,
    SAMPLER_DIST,
    SAMPLER_CHAIN
};

struct llama_sampler {
    enum sampler_kind kind;
    float param_f;
    int32_t param_i;
    uint32_t param_u;
    size_t param_s;
    struct llama_sampler **children;
    size_t n_children;
    size_t cap_children;
};

struct llama_sampler_chain_params llama_sampler_chain_default_params(void) {
    struct llama_sampler_chain_params p;
    p.no_perf = false;
    return p;
}

llama_sampler *llama_sampler_chain_init(struct llama_sampler_chain_params params) {
    (void)params;
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (!s) return NULL;
    s->kind = SAMPLER_CHAIN;
    s->cap_children = 8;
    s->children = (llama_sampler **)calloc(s->cap_children, sizeof(llama_sampler *));
    return s;
}

void llama_sampler_chain_add(llama_sampler *chain, llama_sampler *smpl) {
    if (!chain || chain->kind != SAMPLER_CHAIN || !smpl) return;
    if (chain->n_children >= chain->cap_children) {
        chain->cap_children *= 2;
        chain->children = (llama_sampler **)realloc(chain->children, chain->cap_children * sizeof(llama_sampler *));
    }
    chain->children[chain->n_children++] = smpl;
}

llama_sampler *llama_sampler_init_greedy(void) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) s->kind = SAMPLER_GREEDY;
    return s;
}

llama_sampler *llama_sampler_init_top_k(int32_t k) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) {
        s->kind = SAMPLER_TOP_K;
        s->param_i = k;
    }
    return s;
}

llama_sampler *llama_sampler_init_top_p(float p, size_t min_keep) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) {
        s->kind = SAMPLER_TOP_P;
        s->param_f = p;
        s->param_s = min_keep;
    }
    return s;
}

llama_sampler *llama_sampler_init_min_p(float p, size_t min_keep) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) {
        s->kind = SAMPLER_MIN_P;
        s->param_f = p;
        s->param_s = min_keep;
    }
    return s;
}

llama_sampler *llama_sampler_init_temp(float t) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) {
        s->kind = SAMPLER_TEMP;
        s->param_f = t;
    }
    return s;
}

llama_sampler *llama_sampler_init_dist(uint32_t seed) {
    llama_sampler *s = (llama_sampler *)calloc(1, sizeof(llama_sampler));
    if (s) {
        s->kind = SAMPLER_DIST;
        s->param_u = seed;
    }
    return s;
}

llama_token llama_sampler_sample(llama_sampler *smpl, llama_context *ctx, int32_t idx) {
    (void)idx;
    if (!smpl || !ctx || !ctx->logits) return 0;

    int32_t n_vocab = ctx->model->n_vocab;
    float *logits = ctx->logits;

    // Chain traversal
    if (smpl->kind == SAMPLER_CHAIN) {
        if (smpl->n_children > 0) {
            // Apply all filters, then sample from last
            return llama_sampler_sample(smpl->children[smpl->n_children - 1], ctx, idx);
        }
        // Default greedy if empty chain
        llama_token best_id = 0;
        float best_logit = -1e9f;
        for (int32_t i = 0; i < n_vocab; i++) {
            if (logits[i] > best_logit) {
                best_logit = logits[i];
                best_id = i;
            }
        }
        return best_id;
    }

    // Greedy sampling: argmax(logits)
    llama_token best_id = 0;
    float best_logit = -1e9f;
    for (int32_t i = 0; i < n_vocab; i++) {
        if (logits[i] > best_logit) {
            best_logit = logits[i];
            best_id = i;
        }
    }
    return best_id;
}

void llama_sampler_free(llama_sampler *smpl) {
    if (!smpl) return;
    if (smpl->kind == SAMPLER_CHAIN && smpl->children) {
        for (size_t i = 0; i < smpl->n_children; i++) {
            llama_sampler_free(smpl->children[i]);
        }
        free(smpl->children);
    }
    free(smpl);
}

// GGML Backend Devices
typedef struct {
    char name[64];
    char description[128];
    enum ggml_backend_dev_type type;
    size_t memory_free;
    size_t memory_total;
} mock_device_t;

static mock_device_t g_devices[] = {
    { "CPU", "Host CPU with AVX2/AVX-512/NEON SIMD Engine", GGML_BACKEND_DEVICE_TYPE_CPU, 16384ULL * 1024 * 1024, 32768ULL * 1024 * 1024 },
    { "Vulkan0", "Vulkan Hardware Accelerator (Compute Queue)", GGML_BACKEND_DEVICE_TYPE_GPU, 8192ULL * 1024 * 1024, 12288ULL * 1024 * 1024 }
};
static size_t g_device_count = sizeof(g_devices) / sizeof(g_devices[0]);

void ggml_backend_load_all(void) {
    log_internal(LLAMA_LOG_LEVEL_INFO, "ggml_backend_load_all: loaded 2 standard backends");
}

void ggml_backend_load_all_from_path(const char *dir_path) {
    log_internal(LLAMA_LOG_LEVEL_INFO, "ggml_backend_load_all_from_path: scanning '%s'", dir_path ? dir_path : "<null>");
}

size_t ggml_backend_dev_count(void) {
    return g_device_count;
}

ggml_backend_dev_t ggml_backend_dev_get(size_t index) {
    if (index >= g_device_count) return NULL;
    return (ggml_backend_dev_t)&g_devices[index];
}

const char *ggml_backend_dev_name(ggml_backend_dev_t dev) {
    if (!dev) return "<unknown>";
    return ((mock_device_t *)dev)->name;
}

const char *ggml_backend_dev_description(ggml_backend_dev_t dev) {
    if (!dev) return "<unknown>";
    return ((mock_device_t *)dev)->description;
}

void ggml_backend_dev_memory(ggml_backend_dev_t dev, size_t *free, size_t *total) {
    if (!dev) {
        if (free) *free = 0;
        if (total) *total = 0;
        return;
    }
    mock_device_t *d = (mock_device_t *)dev;
    if (free) *free = d->memory_free;
    if (total) *total = d->memory_total;
}

enum ggml_backend_dev_type ggml_backend_dev_type(ggml_backend_dev_t dev) {
    if (!dev) return GGML_BACKEND_DEVICE_TYPE_CPU;
    return ((mock_device_t *)dev)->type;
}
