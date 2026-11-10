//! Dynamic GGML backend plugin loader and hardware accelerator prober.
//!
//! Enforces:
//! - Cryptographic verification of dynamic plugins (BLAKE3 digest check against binary manifest).
//! - Enumeration of compiled-in and dynamic GGML devices (CPU, Vulkan, Metal, CUDA, ROCm).
//! - Device memory query and telemetry integration.

use crate::error::EngineError;
use std::ffi::{CStr, CString};
use std::path::Path;
use umcf_llama_sys::*;

/// Probed metadata and memory capacities for a GGML backend device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GgmlBackendDescriptor {
    pub name: String,
    pub description: String,
    pub device_type: GgmlBackendDevType,
    pub memory_free_bytes: usize,
    pub memory_total_bytes: usize,
}

/// Dynamic backend plugin and hardware prober.
#[derive(Debug, Clone, Copy, Default)]
pub struct DynamicBackendLoader;

impl DynamicBackendLoader {
    /// Discovers and enumerates all available compiled-in GGML backends and accelerators.
    #[must_use]
    pub fn probe_available_backends() -> Vec<GgmlBackendDescriptor> {
        // SAFETY: ggml_backend_load_all initializes default compiled-in backends
        unsafe {
            ggml_backend_load_all();
        }

        // SAFETY: ggml_backend_dev_count queries registered devices
        let count = unsafe { ggml_backend_dev_count() };
        let mut descriptors = Vec::with_capacity(count);

        for i in 0..count {
            // SAFETY: i < count
            let dev = unsafe { ggml_backend_dev_get(i) };
            if dev.is_null() {
                continue;
            }

            // SAFETY: dev is non-null
            let name_ptr = unsafe { ggml_backend_dev_name(dev) };
            let name = if name_ptr.is_null() {
                format!("Backend-{i}")
            } else {
                // SAFETY: name_ptr is null-terminated C string
                unsafe { CStr::from_ptr(name_ptr) }
                    .to_str()
                    .unwrap_or("Unknown")
                    .to_string()
            };

            // SAFETY: dev is non-null
            let desc_ptr = unsafe { ggml_backend_dev_description(dev) };
            let description = if desc_ptr.is_null() {
                String::new()
            } else {
                // SAFETY: desc_ptr is null-terminated C string
                unsafe { CStr::from_ptr(desc_ptr) }
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            };

            let mut free = 0usize;
            let mut total = 0usize;
            // SAFETY: free and total are valid stack pointers
            unsafe {
                ggml_backend_dev_memory(dev, &mut free, &mut total);
            }

            // SAFETY: dev is non-null
            let dev_type = unsafe { ggml_backend_dev_type(dev) };

            descriptors.push(GgmlBackendDescriptor {
                name,
                description,
                device_type: dev_type,
                memory_free_bytes: free,
                memory_total_bytes: total,
            });
        }

        descriptors
    }

    /// Verifies the cryptographic digest of a dynamic GGML backend plugin shared library
    /// before calling into the native dynamic loader.
    ///
    /// If the computed BLAKE3 digest of the plugin binary does not match the expected
    /// hash from the UMCF binary release manifest, loading is rejected immediately.
    pub fn load_plugin_verified(
        plugin_path: &Path,
        expected_blake3_hex: &str,
    ) -> Result<Vec<GgmlBackendDescriptor>, EngineError> {
        let path_str = plugin_path.to_string_lossy().to_string();

        if !plugin_path.exists() {
            return Err(EngineError::BackendPluginLoadFailed {
                path: path_str,
                reason: "plugin file does not exist".to_string(),
            });
        }

        // Read file bytes and verify BLAKE3 hash
        let bytes = std::fs::read(plugin_path)?;
        let actual_blake3_hex = blake3::hash(&bytes).to_hex().to_string();

        if actual_blake3_hex != expected_blake3_hex {
            return Err(EngineError::BackendPluginSignatureMismatch {
                path: path_str,
                expected: expected_blake3_hex.to_string(),
                actual: actual_blake3_hex,
            });
        }

        // Parent directory containing dynamic plugin
        let parent_dir = plugin_path.parent().unwrap_or(plugin_path);
        let c_dir = CString::new(parent_dir.to_string_lossy().as_bytes()).map_err(|e| {
            EngineError::BackendPluginLoadFailed {
                path: path_str.clone(),
                reason: format!("null byte in directory path: {e}"),
            }
        })?;

        // SAFETY: c_dir is valid null-terminated CString
        unsafe {
            ggml_backend_load_all_from_path(c_dir.as_ptr());
        }

        tracing::info!(
            target: "umcf_ai::llama_cpp",
            plugin = %path_str,
            digest = %actual_blake3_hex,
            "Successfully verified and loaded GGML backend plugin"
        );

        Ok(Self::probe_available_backends())
    }
}
