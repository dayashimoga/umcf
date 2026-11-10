#ifndef UMCF_LLAMA_H
#define UMCF_LLAMA_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LLAMA_LOG_LEVEL_ERROR 1
#define LLAMA_LOG_LEVEL_WARN  2
#define LLAMA_LOG_LEVEL_INFO  3
#define LLAMA_LOG_LEVEL_DEBUG 4

typedef int32_t llama_token;
typedef int32_t llama_pos;
typedef int32_t llama_seq_id;

typedef struct llama_model llama_model;
typedef struct llama_context llama_context;
typedef struct llama_sampler llama_sampler;

typedef bool (*llama_progress_callback)(float progress, void *user_data);
typedef bool (*ggml_abort_callback)(void *user_data);
typedef void (*llama_log_callback)(int level, const char *text, void *user_data);

struct llama_model_params {
    int32_t n_gpu_layers;
    int32_t split_mode;
    int32_t main_gpu;
    const float *tensor_split;
    llama_progress_callback progress_callback;
    void *progress_callback_user_data;
    const void *kv_overrides;
    bool vocab_only;
    bool use_mmap;
    bool use_mlock;
    bool check_tensors;
};

struct llama_context_params {
    uint32_t n_ctx;
    uint32_t n_batch;
    uint32_t n_ubatch;
    uint32_t n_seq_max;
    uint32_t n_threads;
    uint32_t n_threads_batch;
    int32_t rope_scaling_type;
    int32_t pooling_type;
    int32_t attention_type;
    float rope_freq_base;
    float rope_freq_scale;
    float yarn_ext_factor;
    float yarn_attn_factor;
    float yarn_beta_fast;
    float yarn_beta_slow;
    uint32_t yarn_orig_ctx;
    float defrag_thold;
    bool embeddings;
    bool offload_kqv;
    bool flash_attn;
    bool no_perf;
    ggml_abort_callback abort_callback;
    void *abort_callback_data;
};

struct llama_batch {
    int32_t n_tokens;
    llama_token *token;
    float *embd;
    llama_pos *pos;
    int32_t *n_seq_id;
    llama_seq_id **seq_id;
    int8_t *logits;
};

struct llama_sampler_chain_params {
    bool no_perf;
};

struct llama_token_data {
    llama_token id;
    float logit;
    float p;
};

struct llama_token_data_array {
    struct llama_token_data *data;
    size_t size;
    int64_t selected;
    bool sorted;
};

// Lifecycle
void llama_backend_init(void);
void llama_backend_free(void);
void llama_log_set(llama_log_callback log_callback, void *user_data);

struct llama_model_params llama_model_default_params(void);
struct llama_context_params llama_context_default_params(void);

// Model operations
llama_model *llama_load_model_from_file(const char *path_model, struct llama_model_params params);
void llama_model_free(llama_model *model);
void llama_free_model(llama_model *model); // alias

// Context operations
llama_context *llama_new_context_with_model(llama_model *model, struct llama_context_params params);
llama_context *llama_init_from_model(llama_model *model, struct llama_context_params params); // alias
void llama_free(llama_context *ctx);

// Batch operations
struct llama_batch llama_batch_init(int32_t n_tokens, int32_t embd, int32_t n_seq_max);
void llama_batch_free(struct llama_batch batch);

// Tokenization
int32_t llama_tokenize(const llama_model *model, const char *text, int32_t text_len, llama_token *tokens, int32_t n_tokens_max, bool add_special, bool parse_special);
int32_t llama_token_to_piece(const llama_model *model, llama_token token, char *buf, int32_t length, int32_t lstrip, bool special);
llama_token llama_token_bos(const llama_model *model);
llama_token llama_token_eos(const llama_model *model);
llama_token llama_token_nl(const llama_model *model);

// Model properties
int32_t llama_n_vocab(const llama_model *model);
int32_t llama_n_ctx_train(const llama_model *model);
int32_t llama_n_embd(const llama_model *model);
int32_t llama_model_n_layers(const llama_model *model);

// Context properties
uint32_t llama_n_ctx(const llama_context *ctx);
uint32_t llama_n_batch(const llama_context *ctx);
uint32_t llama_n_ubatch(const llama_context *ctx);

// Execution
int32_t llama_decode(llama_context *ctx, struct llama_batch batch);
float *llama_get_logits(llama_context *ctx);
float *llama_get_logits_ith(llama_context *ctx, int32_t i);
float *llama_get_embeddings(llama_context *ctx);
float *llama_get_embeddings_seq(llama_context *ctx, llama_seq_id seq_id);

// KV Cache
void llama_kv_cache_clear(llama_context *ctx);
bool llama_kv_cache_seq_rm(llama_context *ctx, llama_seq_id seq_id, llama_pos p0, llama_pos p1);
void llama_kv_cache_seq_cp(llama_context *ctx, llama_seq_id seq_id_src, llama_seq_id seq_id_dst, llama_pos p0, llama_pos p1);
void llama_kv_cache_seq_keep(llama_context *ctx, llama_seq_id seq_id);
void llama_kv_cache_seq_shift(llama_context *ctx, llama_seq_id seq_id, llama_pos p0, llama_pos p1, llama_pos delta);

// Samplers
struct llama_sampler_chain_params llama_sampler_chain_default_params(void);
llama_sampler *llama_sampler_chain_init(struct llama_sampler_chain_params params);
void llama_sampler_chain_add(llama_sampler *chain, llama_sampler *smpl);
llama_sampler *llama_sampler_init_greedy(void);
llama_sampler *llama_sampler_init_top_k(int32_t k);
llama_sampler *llama_sampler_init_top_p(float p, size_t min_keep);
llama_sampler *llama_sampler_init_min_p(float p, size_t min_keep);
llama_sampler *llama_sampler_init_temp(float t);
llama_sampler *llama_sampler_init_dist(uint32_t seed);
llama_token llama_sampler_sample(llama_sampler *smpl, llama_context *ctx, int32_t idx);
void llama_sampler_free(llama_sampler *smpl);

#ifdef __cplusplus
}
#endif

#endif // UMCF_LLAMA_H
