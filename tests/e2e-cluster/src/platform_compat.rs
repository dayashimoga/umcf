//! Cross-platform wire and path normalization, 16 KiB page alignment, and case-collision detection.

use std::collections::HashMap;
use crate::error::PlatformError;

/// Standard 16 KiB page size mandated by Apple Silicon (macOS/iOS) and Android 15+ kernels.
pub const PAGE_SIZE_16KB: usize = 16 * 1024; // 16,384 bytes

/// Legacy 4 KiB page size for x86_64 systems.
pub const PAGE_SIZE_4KB: usize = 4 * 1024; // 4,096 bytes

/// Canonical virtual path normalizer enforcing POSIX-style forward-slash paths over the wire.
#[derive(Debug, Clone, Copy)]
pub struct VirtualPathNormalizer;

impl VirtualPathNormalizer {
    /// Validates and normalizes a path for cross-platform wire serialization.
    ///
    /// Rejects:
    /// - Windows backslashes (`\`)
    /// - Windows drive letters (`C:`, `D:\`)
    /// - Leading slashes (absolute paths)
    /// - Path traversal segments (`..`)
    /// - Empty segments
    pub fn normalize_wire_path(raw_path: &str) -> Result<String, PlatformError> {
        // 1. Strictly forbid backslashes on the wire
        if raw_path.contains('\\') {
            return Err(PlatformError::InvalidWirePath {
                path: raw_path.to_string(),
                reason: "Windows backslashes ('\\') are strictly prohibited on the wire; use '/'".to_string(),
            });
        }

        // 2. Strictly forbid drive letters (e.g., "C:", "D:")
        if raw_path.len() >= 2 {
            let bytes = raw_path.as_bytes();
            if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
                return Err(PlatformError::InvalidWirePath {
                    path: raw_path.to_string(),
                    reason: "Drive letters (e.g., 'C:') are strictly prohibited on the wire".to_string(),
                });
            }
        }

        // 3. Strictly forbid absolute paths
        if raw_path.starts_with('/') {
            return Err(PlatformError::InvalidWirePath {
                path: raw_path.to_string(),
                reason: "Absolute paths starting with '/' are strictly prohibited on the wire".to_string(),
            });
        }

        // 4. Tokenize and inspect segments
        let segments: Vec<&str> = raw_path.split('/').collect();
        let mut clean_segments = Vec::new();

        for seg in segments {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                return Err(PlatformError::InvalidWirePath {
                    path: raw_path.to_string(),
                    reason: "Path traversal segment ('..') is strictly prohibited".to_string(),
                });
            }
            clean_segments.push(seg);
        }

        if clean_segments.is_empty() {
            return Err(PlatformError::InvalidWirePath {
                path: raw_path.to_string(),
                reason: "Path must contain at least one valid path component".to_string(),
            });
        }

        Ok(clean_segments.join("/"))
    }
}

/// Detects case-collision hazards across heterogeneous case-sensitive and case-insensitive filesystems.
#[derive(Debug, Clone, Default)]
pub struct CaseCollisionDetector;

impl CaseCollisionDetector {
    /// Inspects a list of paths and flags any two paths that collide when lowercased.
    ///
    /// For example, `["docs/README.md", "docs/readme.md"]` will succeed on Linux (case-sensitive)
    /// but silently overwrite and corrupt data on Windows (NTFS) or macOS (APFS case-insensitive).
    pub fn detect_collisions<T: AsRef<str>>(paths: &[T]) -> Result<(), PlatformError> {
        let mut seen: HashMap<String, String> = HashMap::new();

        for p in paths {
            let original = p.as_ref().to_string();
            let lower = original.to_lowercase();

            if let Some(existing) = seen.get(&lower) {
                if existing != &original {
                    return Err(PlatformError::CaseCollisionDetected {
                        path_a: existing.clone(),
                        path_b: original,
                    });
                }
            } else {
                seen.insert(lower, original);
            }
        }

        Ok(())
    }
}

/// Page alignment validator for memory-mapped model tensors and DMA buffers.
#[derive(Debug, Clone, Copy)]
pub struct PageAlignmentValidator;

impl PageAlignmentValidator {
    /// Validates whether an address or buffer offset is aligned to a 16 KiB page boundary.
    ///
    /// Apple Silicon (M1/M2/M3/M4) and Android 15+ kernels enforce 16 KiB page tables.
    /// GGUF models or memory-mapped buffers with only 4 KiB alignment trigger SIGBUS or kernel page faults.
    pub fn validate_16kb_alignment(offset: usize) -> Result<(), PlatformError> {
        if offset % PAGE_SIZE_16KB != 0 {
            return Err(PlatformError::PageAlignmentViolation {
                address: offset,
                offset,
                required_page_size_bytes: PAGE_SIZE_16KB,
            });
        }
        Ok(())
    }

    /// Computes the padded byte size rounded up to the nearest 16 KiB page boundary.
    pub fn pad_to_16kb(size: usize) -> usize {
        let remainder = size % PAGE_SIZE_16KB;
        if remainder == 0 {
            size
        } else {
            size + (PAGE_SIZE_16KB - remainder)
        }
    }
}

