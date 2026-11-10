#ifndef UMCF_GGML_BACKEND_H
#define UMCF_GGML_BACKEND_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

enum ggml_backend_dev_type {
    GGML_BACKEND_DEVICE_TYPE_CPU = 0,
    GGML_BACKEND_DEVICE_TYPE_GPU = 1,
    GGML_BACKEND_DEVICE_TYPE_ACCEL = 2
};

typedef void *ggml_backend_dev_t;
typedef void *ggml_backend_t;

void ggml_backend_load_all(void);
void ggml_backend_load_all_from_path(const char *dir_path);
size_t ggml_backend_dev_count(void);
ggml_backend_dev_t ggml_backend_dev_get(size_t index);
const char *ggml_backend_dev_name(ggml_backend_dev_t dev);
const char *ggml_backend_dev_description(ggml_backend_dev_t dev);
void ggml_backend_dev_memory(ggml_backend_dev_t dev, size_t *free, size_t *total);
enum ggml_backend_dev_type ggml_backend_dev_type(ggml_backend_dev_t dev);

#ifdef __cplusplus
}
#endif

#endif // UMCF_GGML_BACKEND_H
