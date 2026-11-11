//! Bounded Slice Reader and Pre-Allocation Guards for untrusted wire payloads.

use crate::error::FuzzError;

pub const MAX_ALLOCATION_BYTES: usize = 16 * 1024 * 1024; // 16 MiB
pub const MAX_RECURSION_DEPTH: usize = 32;

/// Safe, non-allocating binary cursor enforcing bounds and pre-allocation limits.
#[derive(Debug, Clone)]
pub struct BoundedSliceReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> BoundedSliceReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.data.len()
    }

    pub fn read_u8(&mut self) -> Result<u8, FuzzError> {
        if self.offset < self.data.len() {
            let b = self.data[self.offset];
            self.offset += 1;
            Ok(b)
        } else {
            Err(FuzzError::MalformedInput {
                target: "BinaryReader".to_string(),
                reason: "Unexpected EOF reading u8".to_string(),
            })
        }
    }

    pub fn read_u16_le(&mut self) -> Result<u16, FuzzError> {
        if self.remaining() >= 2 {
            let bytes = [self.data[self.offset], self.data[self.offset + 1]];
            self.offset += 2;
            Ok(u16::from_le_bytes(bytes))
        } else {
            Err(FuzzError::MalformedInput {
                target: "BinaryReader".to_string(),
                reason: "Unexpected EOF reading u16_le".to_string(),
            })
        }
    }

    pub fn read_u32_le(&mut self) -> Result<u32, FuzzError> {
        if self.remaining() >= 4 {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.offset..self.offset + 4]);
            self.offset += 4;
            Ok(u32::from_le_bytes(bytes))
        } else {
            Err(FuzzError::MalformedInput {
                target: "BinaryReader".to_string(),
                reason: "Unexpected EOF reading u32_le".to_string(),
            })
        }
    }

    pub fn read_u64_le(&mut self) -> Result<u64, FuzzError> {
        if self.remaining() >= 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.offset..self.offset + 8]);
            self.offset += 8;
            Ok(u64::from_le_bytes(bytes))
        } else {
            Err(FuzzError::MalformedInput {
                target: "BinaryReader".to_string(),
                reason: "Unexpected EOF reading u64_le".to_string(),
            })
        }
    }

    /// Reads a length-prefixed slice, enforcing `requested_len <= max_allowed`
    /// before allocating or slicing, neutralizing length-prefix allocation bombs (e.g. 2^62).
    pub fn read_length_prefixed_slice(
        &mut self,
        target_name: &str,
        max_allowed: usize,
    ) -> Result<&'a [u8], FuzzError> {
        let claimed_len = self.read_u64_le()?;

        // Neutralize integer overflow / allocation bomb upfront
        if claimed_len > max_allowed as u64 {
            return Err(FuzzError::AllocationExceeded {
                target: target_name.to_string(),
                requested_bytes: claimed_len as usize,
                limit_bytes: max_allowed,
            });
        }

        let len = claimed_len as usize;
        if self.remaining() < len {
            return Err(FuzzError::MalformedInput {
                target: target_name.to_string(),
                reason: format!("Truncated payload: claimed {len} bytes, only {} available", self.remaining()),
            });
        }

        let slice = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(slice)
    }
}

/// Recursion depth detector for JSON/bracket-nested schemas.
pub fn check_nesting_depth(data: &[u8], max_depth: usize) -> Result<usize, FuzzError> {
    let mut current_depth = 0usize;
    let mut max_observed = 0usize;

    for &b in data {
        match b {
            b'{' | b'[' | b'(' => {
                current_depth += 1;
                if current_depth > max_observed {
                    max_observed = current_depth;
                }
                if current_depth > max_depth {
                    return Err(FuzzError::RecursionDepthExceeded {
                        target: "NestingParser".to_string(),
                        depth: current_depth,
                        limit: max_depth,
                    });
                }
            }
            b'}' | b']' | b')' => {
                current_depth = current_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    Ok(max_observed)
}

