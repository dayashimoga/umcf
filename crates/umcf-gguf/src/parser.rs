//! Safe zero-copy binary parser and metadata inspector for GGUF v2/v3 models.
//!
//! Parses GGUF headers, all 13 metadata value types, tensor directories,
//! quantization types, architecture hyperparameters, and per-layer weight profiles.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::StoreError;

/// 4-byte GGUF magic identifier ("GGUF" in little-endian = 0x46554747).
pub const GGUF_MAGIC: u32 = 0x4655_4747;

/// Supported GGUF specification versions.
pub const GGUF_VERSION_V2: u32 = 2;
pub const GGUF_VERSION_V3: u32 = 3;

/// Security boundary caps to prevent memory denial-of-service on corrupted files.
pub const MAX_METADATA_COUNT: u64 = 10_000;
pub const MAX_TENSOR_COUNT: u64 = 10_000;
pub const MAX_STRING_LEN: usize = 16 * 1024 * 1024; // 16 MiB max string
pub const MAX_ARRAY_LEN: usize = 1_000_000;

/// GGML numeric quantization and tensor element types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
    Q8_K = 15,
    IQ2_XXS = 16,
    IQ2_XS = 17,
    IQ3_XXS = 18,
    IQ1_S = 19,
    IQ4_NL = 20,
    IQ3_S = 21,
    IQ2_S = 22,
    IQ4_XS = 23,
    I8 = 24,
    I16 = 25,
    I32 = 26,
    I64 = 27,
    F64 = 28,
    IQ1_M = 29,
    BF16 = 30,
}

impl GgmlType {
    /// Attempts to parse a GGML type from its 32-bit integer identifier.
    pub fn from_u32(val: u32) -> Result<Self, StoreError> {
        match val {
            0 => Ok(Self::F32),
            1 => Ok(Self::F16),
            2 => Ok(Self::Q4_0),
            3 => Ok(Self::Q4_1),
            6 => Ok(Self::Q5_0),
            7 => Ok(Self::Q5_1),
            8 => Ok(Self::Q8_0),
            9 => Ok(Self::Q8_1),
            10 => Ok(Self::Q2_K),
            11 => Ok(Self::Q3_K),
            12 => Ok(Self::Q4_K),
            13 => Ok(Self::Q5_K),
            14 => Ok(Self::Q6_K),
            15 => Ok(Self::Q8_K),
            16 => Ok(Self::IQ2_XXS),
            17 => Ok(Self::IQ2_XS),
            18 => Ok(Self::IQ3_XXS),
            19 => Ok(Self::IQ1_S),
            20 => Ok(Self::IQ4_NL),
            21 => Ok(Self::IQ3_S),
            22 => Ok(Self::IQ2_S),
            23 => Ok(Self::IQ4_XS),
            24 => Ok(Self::I8),
            25 => Ok(Self::I16),
            26 => Ok(Self::I32),
            27 => Ok(Self::I64),
            28 => Ok(Self::F64),
            29 => Ok(Self::IQ1_M),
            30 => Ok(Self::BF16),
            other => Err(StoreError::CorruptedFile(format!(
                "unsupported or invalid GGML tensor type ID: {other}"
            ))),
        }
    }

    /// Returns the block size (number of elements per quantization block).
    #[must_use]
    pub const fn block_size(&self) -> u64 {
        match self {
            Self::F32 | Self::I32 | Self::I16 | Self::I8 | Self::I64 | Self::F64 | Self::F16 | Self::BF16 => 1,
            Self::Q4_0 | Self::Q4_1 | Self::Q5_0 | Self::Q5_1 | Self::Q8_0 | Self::Q8_1 | Self::IQ4_NL => 32,
            Self::Q2_K | Self::Q3_K | Self::Q4_K | Self::Q5_K | Self::Q6_K | Self::Q8_K
            | Self::IQ2_XXS | Self::IQ2_XS | Self::IQ3_XXS | Self::IQ1_S | Self::IQ3_S | Self::IQ2_S
            | Self::IQ4_XS | Self::IQ1_M => 256,
        }
    }

    /// Returns the byte size of an individual quantization block.
    #[must_use]
    pub const fn type_size(&self) -> u64 {
        match self {
            Self::F32 => 4,
            Self::F16 | Self::BF16 => 2,
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I32 => 4,
            Self::I64 | Self::F64 => 8,
            Self::Q4_0 => 18,
            Self::Q4_1 => 20,
            Self::Q5_0 => 22,
            Self::Q5_1 => 24,
            Self::Q8_0 => 34,
            Self::Q8_1 => 36,
            Self::Q2_K => 84,
            Self::Q3_K => 110,
            Self::Q4_K => 144,
            Self::Q5_K => 176,
            Self::Q6_K => 210,
            Self::Q8_K => 292,
            Self::IQ2_XXS => 66,
            Self::IQ2_XS => 74,
            Self::IQ3_XXS => 98,
            Self::IQ1_S => 48,
            Self::IQ4_NL => 18,
            Self::IQ3_S => 110,
            Self::IQ2_S => 82,
            Self::IQ4_XS => 136,
            Self::IQ1_M => 56,
        }
    }

    /// Calculates total byte storage required for `n_elements`.
    pub fn tensor_size_bytes(&self, n_elements: u64) -> Result<u64, StoreError> {
        let blk = self.block_size();
        let t_sz = self.type_size();
        if n_elements == 0 {
            return Ok(0);
        }
        let num_blocks = n_elements.div_ceil(blk);
        num_blocks
            .checked_mul(t_sz)
            .ok_or_else(|| StoreError::CorruptedFile("tensor size byte calculation overflowed".into()))
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::F16 => "F16",
            Self::BF16 => "BF16",
            Self::Q4_0 => "Q4_0",
            Self::Q4_1 => "Q4_1",
            Self::Q5_0 => "Q5_0",
            Self::Q5_1 => "Q5_1",
            Self::Q8_0 => "Q8_0",
            Self::Q8_1 => "Q8_1",
            Self::Q2_K => "Q2_K",
            Self::Q3_K => "Q3_K",
            Self::Q4_K => "Q4_K",
            Self::Q5_K => "Q5_K",
            Self::Q6_K => "Q6_K",
            Self::Q8_K => "Q8_K",
            Self::IQ2_XXS => "IQ2_XXS",
            Self::IQ2_XS => "IQ2_XS",
            Self::IQ3_XXS => "IQ3_XXS",
            Self::IQ1_S => "IQ1_S",
            Self::IQ4_NL => "IQ4_NL",
            Self::IQ3_S => "IQ3_S",
            Self::IQ2_S => "IQ2_S",
            Self::IQ4_XS => "IQ4_XS",
            Self::I8 => "I8",
            Self::I16 => "I16",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::F64 => "F64",
            Self::IQ1_M => "IQ1_M",
        }
    }
}

impl fmt::Display for GgmlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// GGUF metadata value representing all 13 supported specification types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GgufMetadataValue {
    UInt8(u8),
    Int8(i8),
    UInt16(u16),
    Int16(i16),
    UInt32(u32),
    Int32(i32),
    Float32(f32),
    Bool(bool),
    String(String),
    Array(Vec<GgufMetadataValue>),
    UInt64(u64),
    Int64(i64),
    Float64(f64),
}

impl GgufMetadataValue {
    #[must_use]
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::UInt32(v) => Some(*v),
            Self::UInt64(v) => u32::try_from(*v).ok(),
            Self::Int32(v) => u32::try_from(*v).ok(),
            Self::UInt16(v) => Some(u32::from(*v)),
            Self::UInt8(v) => Some(u32::from(*v)),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::UInt64(v) => Some(*v),
            Self::UInt32(v) => Some(u64::from(*v)),
            Self::Int64(v) => u64::try_from(*v).ok(),
            Self::Int32(v) => u64::try_from(*v).ok(),
            Self::UInt16(v) => Some(u64::from(*v)),
            Self::UInt8(v) => Some(u64::from(*v)),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float32(v) => Some(*v),
            Self::Float64(v) => Some(*v as f32),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Metadata describing an individual tensor inside the GGUF file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GgufTensorInfo {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub ggml_type: GgmlType,
    pub offset: u64,
    pub size_bytes: u64,
    pub layer_index: Option<usize>,
}

impl GgufTensorInfo {
    /// Total number of scalar/quantized elements in this tensor.
    #[must_use]
    pub fn total_elements(&self) -> u64 {
        if self.dimensions.is_empty() {
            0
        } else {
            self.dimensions.iter().copied().product()
        }
    }
}

/// Normalized transformer architecture hyperparameters extracted from GGUF metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelArchitectureMetadata {
    pub architecture: String,
    pub block_count: u32,
    pub context_length: u32,
    pub embedding_length: u32,
    pub feed_forward_length: Option<u32>,
    pub head_count: u32,
    pub head_count_kv: u32,
    pub rope_freq_base: Option<f32>,
    pub rope_dim: Option<u32>,
    pub vocab_size: Option<u32>,
    pub alignment: u32,
}

impl Default for ModelArchitectureMetadata {
    fn default() -> Self {
        Self {
            architecture: "llama".to_string(),
            block_count: 32,
            context_length: 4096,
            embedding_length: 4096,
            feed_forward_length: Some(11008),
            head_count: 32,
            head_count_kv: 32,
            rope_freq_base: Some(10000.0),
            rope_dim: Some(128),
            vocab_size: Some(32000),
            alignment: 32,
        }
    }
}

/// Profile of weight memory and tensors associated with a specific transformer layer or block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerWeightProfile {
    pub layer_index: Option<usize>,
    pub name_prefix: String,
    pub tensor_count: usize,
    pub total_weight_bytes: u64,
    pub quantization_breakdown: BTreeMap<String, u64>,
}

/// Complete inspection report of a GGUF file's structure and hyperparameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInspection {
    pub version: u32,
    pub metadata: ModelArchitectureMetadata,
    pub raw_metadata: BTreeMap<String, GgufMetadataValue>,
    pub tensors: Vec<GgufTensorInfo>,
    pub layer_profiles: Vec<LayerWeightProfile>,
    pub total_weight_bytes: u64,
    pub data_offset: u64,
}

/// Cursor-based parser for reading primitives safely without unsafe pointer arithmetic.
struct ByteCursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], StoreError> {
        if self.remaining() < len {
            return Err(StoreError::CorruptedFile(format!(
                "unexpected EOF in GGUF stream at byte {}: requested {} bytes, {} available",
                self.pos,
                len,
                self.remaining()
            )));
        }
        let slice = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, StoreError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_i8(&mut self) -> Result<i8, StoreError> {
        Ok(self.read_exact(1)?[0] as i8)
    }

    fn read_u16(&mut self) -> Result<u16, StoreError> {
        let b = self.read_exact(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_i16(&mut self) -> Result<i16, StoreError> {
        let b = self.read_exact(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, StoreError> {
        let b = self.read_exact(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, StoreError> {
        let b = self.read_exact(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, StoreError> {
        let b = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, StoreError> {
        let b = self.read_exact(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_f32(&mut self) -> Result<f32, StoreError> {
        let b = self.read_exact(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_f64(&mut self) -> Result<f64, StoreError> {
        let b = self.read_exact(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_string(&mut self) -> Result<String, StoreError> {
        let len = self.read_u64()? as usize;
        if len > MAX_STRING_LEN {
            return Err(StoreError::CorruptedFile(format!(
                "string length {len} exceeds MAX_STRING_LEN {MAX_STRING_LEN}"
            )));
        }
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(String::from)
            .map_err(|e| StoreError::CorruptedFile(format!("invalid UTF-8 string: {e}")))
    }

    fn read_metadata_value(&mut self, val_type: u32) -> Result<GgufMetadataValue, StoreError> {
        match val_type {
            0 => Ok(GgufMetadataValue::UInt8(self.read_u8()?)),
            1 => Ok(GgufMetadataValue::Int8(self.read_i8()?)),
            2 => Ok(GgufMetadataValue::UInt16(self.read_u16()?)),
            3 => Ok(GgufMetadataValue::Int16(self.read_i16()?)),
            4 => Ok(GgufMetadataValue::UInt32(self.read_u32()?)),
            5 => Ok(GgufMetadataValue::Int32(self.read_i32()?)),
            6 => Ok(GgufMetadataValue::Float32(self.read_f32()?)),
            7 => Ok(GgufMetadataValue::Bool(self.read_u8()? != 0)),
            8 => Ok(GgufMetadataValue::String(self.read_string()?)),
            9 => {
                let elem_type = self.read_u32()?;
                let elem_len = self.read_u64()? as usize;
                if elem_len > MAX_ARRAY_LEN {
                    return Err(StoreError::CorruptedFile(format!(
                        "array length {elem_len} exceeds MAX_ARRAY_LEN {MAX_ARRAY_LEN}"
                    )));
                }
                let mut arr = Vec::with_capacity(elem_len);
                for _ in 0..elem_len {
                    arr.push(self.read_metadata_value(elem_type)?);
                }
                Ok(GgufMetadataValue::Array(arr))
            }
            10 => Ok(GgufMetadataValue::UInt64(self.read_u64()?)),
            11 => Ok(GgufMetadataValue::Int64(self.read_i64()?)),
            12 => Ok(GgufMetadataValue::Float64(self.read_f64()?)),
            other => Err(StoreError::CorruptedFile(format!(
                "unknown GGUF metadata type tag: {other}"
            ))),
        }
    }
}

/// Zero-copy GGUF v2/v3 parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct GgufParser;

impl GgufParser {
    /// Parses a memory buffer containing a GGUF file.
    pub fn parse(data: &[u8]) -> Result<ModelInspection, StoreError> {
        let mut cursor = ByteCursor::new(data);

        // 1. Magic
        let magic = cursor.read_u32()?;
        if magic != GGUF_MAGIC {
            return Err(StoreError::CorruptedFile(format!(
                "invalid GGUF magic bytes: expected 0x{:08X}, got 0x{:08X}",
                GGUF_MAGIC, magic
            )));
        }

        // 2. Version
        let version = cursor.read_u32()?;
        if version != GGUF_VERSION_V2 && version != GGUF_VERSION_V3 {
            return Err(StoreError::CorruptedFile(format!(
                "unsupported GGUF version: {version}. Expected version 2 or 3"
            )));
        }

        // 3. Tensor count and Metadata KV count
        let tensor_count = cursor.read_u64()?;
        if tensor_count > MAX_TENSOR_COUNT {
            return Err(StoreError::CorruptedFile(format!(
                "tensor count {tensor_count} exceeds MAX_TENSOR_COUNT {MAX_TENSOR_COUNT}"
            )));
        }

        let metadata_kv_count = cursor.read_u64()?;
        if metadata_kv_count > MAX_METADATA_COUNT {
            return Err(StoreError::CorruptedFile(format!(
                "metadata count {metadata_kv_count} exceeds MAX_METADATA_COUNT {MAX_METADATA_COUNT}"
            )));
        }

        // 4. Parse Metadata Key-Value pairs
        let mut raw_metadata = BTreeMap::new();
        for _ in 0..metadata_kv_count {
            let key = cursor.read_string()?;
            let val_type = cursor.read_u32()?;
            let value = cursor.read_metadata_value(val_type)?;
            raw_metadata.insert(key, value);
        }

        // 5. Normalize architecture hyperparameters
        let architecture = raw_metadata
            .get("general.architecture")
            .and_then(|v| v.as_str())
            .unwrap_or("llama")
            .to_string();

        let alignment = raw_metadata
            .get("general.alignment")
            .and_then(|v| v.as_u32())
            .unwrap_or(32);

        let block_count = raw_metadata
            .get(&format!("{architecture}.block_count"))
            .or_else(|| raw_metadata.get("general.block_count"))
            .and_then(|v| v.as_u32())
            .unwrap_or(32);

        let context_length = raw_metadata
            .get(&format!("{architecture}.context_length"))
            .and_then(|v| v.as_u32())
            .unwrap_or(4096);

        let embedding_length = raw_metadata
            .get(&format!("{architecture}.embedding_length"))
            .and_then(|v| v.as_u32())
            .unwrap_or(4096);

        let feed_forward_length = raw_metadata
            .get(&format!("{architecture}.feed_forward_length"))
            .and_then(|v| v.as_u32());

        let head_count = raw_metadata
            .get(&format!("{architecture}.attention.head_count"))
            .and_then(|v| v.as_u32())
            .unwrap_or(32);

        let head_count_kv = raw_metadata
            .get(&format!("{architecture}.attention.head_count_kv"))
            .and_then(|v| v.as_u32())
            .unwrap_or(head_count);

        let rope_freq_base = raw_metadata
            .get(&format!("{architecture}.rope.freq_base"))
            .and_then(|v| v.as_f32());

        let rope_dim = raw_metadata
            .get(&format!("{architecture}.rope.dimension_count"))
            .and_then(|v| v.as_u32());

        let vocab_size = raw_metadata
            .get(&format!("{architecture}.vocab_size"))
            .and_then(|v| v.as_u32());

        let meta = ModelArchitectureMetadata {
            architecture,
            block_count,
            context_length,
            embedding_length,
            feed_forward_length,
            head_count,
            head_count_kv,
            rope_freq_base,
            rope_dim,
            vocab_size,
            alignment,
        };

        // 6. Parse Tensor Information Entries
        let mut tensors = Vec::with_capacity(tensor_count as usize);
        let mut total_weight_bytes = 0u64;

        for _ in 0..tensor_count {
            let name = cursor.read_string()?;
            let n_dims = cursor.read_u32()?;
            if n_dims > 4 {
                return Err(StoreError::CorruptedFile(format!(
                    "tensor '{name}' has {n_dims} dimensions; GGUF max supported is 4"
                )));
            }

            let mut dimensions = Vec::with_capacity(n_dims as usize);
            for _ in 0..n_dims {
                dimensions.push(cursor.read_u64()?);
            }

            let type_id = cursor.read_u32()?;
            let ggml_type = GgmlType::from_u32(type_id)?;
            let offset = cursor.read_u64()?;

            // Compute total elements checking for arithmetic overflow
            let mut total_elements: u64 = 1;
            for &dim in &dimensions {
                total_elements = total_elements.checked_mul(dim).ok_or_else(|| {
                    StoreError::CorruptedFile(format!(
                        "tensor '{name}' dimension product overflowed u64"
                    ))
                })?;
            }

            let size_bytes = ggml_type.tensor_size_bytes(total_elements)?;
            total_weight_bytes = total_weight_bytes
                .checked_add(size_bytes)
                .ok_or_else(|| StoreError::CorruptedFile("total model weight overflowed u64".into()))?;

            let layer_index = parse_tensor_layer_index(&name);

            tensors.push(GgufTensorInfo {
                name,
                dimensions,
                ggml_type,
                offset,
                size_bytes,
                layer_index,
            });
        }

        // Align data offset to general.alignment
        let pos_u64 = cursor.pos as u64;
        let alignment_u64 = u64::from(alignment.max(1));
        let data_offset = if pos_u64.is_multiple_of(alignment_u64) {
            pos_u64
        } else {
            pos_u64 + (alignment_u64 - (pos_u64 % alignment_u64))
        };

        // 7. Group into LayerWeightProfiles
        let layer_profiles = build_layer_profiles(&tensors);

        Ok(ModelInspection {
            version,
            metadata: meta,
            raw_metadata,
            tensors,
            layer_profiles,
            total_weight_bytes,
            data_offset,
        })
    }
}

/// Extracts layer index from tensor names like `blk.0.attn_q.weight` -> Some(0).
fn parse_tensor_layer_index(name: &str) -> Option<usize> {
    if let Some(rest) = name.strip_prefix("blk.") {
        let dot = rest.find('.')?;
        rest[..dot].parse::<usize>().ok()
    } else if let Some(rest) = name.strip_prefix("layers.") {
        let dot = rest.find('.')?;
        rest[..dot].parse::<usize>().ok()
    } else {
        None
    }
}

/// Groups tensors by transformer layer into `LayerWeightProfile`s.
fn build_layer_profiles(tensors: &[GgufTensorInfo]) -> Vec<LayerWeightProfile> {
    let mut map: BTreeMap<Option<usize>, Vec<&GgufTensorInfo>> = BTreeMap::new();
    for t in tensors {
        map.entry(t.layer_index).or_default().push(t);
    }

    let mut profiles = Vec::with_capacity(map.len());
    for (layer_opt, t_list) in map {
        let name_prefix = match layer_opt {
            Some(idx) => format!("blk.{idx}"),
            None => "embeddings_and_head".to_string(),
        };

        let mut total_weight_bytes = 0u64;
        let mut breakdown: BTreeMap<String, u64> = BTreeMap::new();

        for t in &t_list {
            total_weight_bytes += t.size_bytes;
            *breakdown.entry(t.ggml_type.name().to_string()).or_insert(0) += t.size_bytes;
        }

        profiles.push(LayerWeightProfile {
            layer_index: layer_opt,
            name_prefix,
            tensor_count: t_list.len(),
            total_weight_bytes,
            quantization_breakdown: breakdown,
        });
    }

    profiles
}

/// Serializes a minimal valid binary GGUF v3 file into a byte buffer for tests and fixtures.
pub fn create_test_gguf_bytes(
    architecture: &str,
    layers: u32,
    context_len: u32,
    embedding_len: u32,
    heads: u32,
    heads_kv: u32,
    quant_type: GgmlType,
) -> Vec<u8> {
    let mut buf = Vec::new();

    // 1. Header
    buf.extend_from_slice(&GGUF_MAGIC.to_le_bytes());
    buf.extend_from_slice(&GGUF_VERSION_V3.to_le_bytes());

    // 2 tensors per layer + 2 non-layer tensors (embedding + output)
    let tensor_count = (u64::from(layers) * 2) + 2;
    buf.extend_from_slice(&tensor_count.to_le_bytes());

    // Metadata entries: arch, block_count, context_length, embedding_length, head_count, head_count_kv
    let metadata_count: u64 = 6;
    buf.extend_from_slice(&metadata_count.to_le_bytes());

    // Helper closures
    let write_str = |b: &mut Vec<u8>, s: &str| {
        b.extend_from_slice(&(s.len() as u64).to_le_bytes());
        b.extend_from_slice(s.as_bytes());
    };

    // Metadata 1: general.architecture (String)
    write_str(&mut buf, "general.architecture");
    buf.extend_from_slice(&8u32.to_le_bytes()); // type String
    write_str(&mut buf, architecture);

    // Metadata 2: {arch}.block_count (UInt32)
    write_str(&mut buf, &format!("{architecture}.block_count"));
    buf.extend_from_slice(&4u32.to_le_bytes());
    buf.extend_from_slice(&layers.to_le_bytes());

    // Metadata 3: {arch}.context_length (UInt32)
    write_str(&mut buf, &format!("{architecture}.context_length"));
    buf.extend_from_slice(&4u32.to_le_bytes());
    buf.extend_from_slice(&context_len.to_le_bytes());

    // Metadata 4: {arch}.embedding_length (UInt32)
    write_str(&mut buf, &format!("{architecture}.embedding_length"));
    buf.extend_from_slice(&4u32.to_le_bytes());
    buf.extend_from_slice(&embedding_len.to_le_bytes());

    // Metadata 5: {arch}.attention.head_count (UInt32)
    write_str(&mut buf, &format!("{architecture}.attention.head_count"));
    buf.extend_from_slice(&4u32.to_le_bytes());
    buf.extend_from_slice(&heads.to_le_bytes());

    // Metadata 6: {arch}.attention.head_count_kv (UInt32)
    write_str(&mut buf, &format!("{architecture}.attention.head_count_kv"));
    buf.extend_from_slice(&4u32.to_le_bytes());
    buf.extend_from_slice(&heads_kv.to_le_bytes());

    // Tensors
    let mut current_offset: u64 = 0;

    // Embedding tensor
    write_str(&mut buf, "token_embd.weight");
    buf.extend_from_slice(&2u32.to_le_bytes()); // 2 dims
    buf.extend_from_slice(&32000u64.to_le_bytes());
    buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
    buf.extend_from_slice(&(quant_type as u32).to_le_bytes());
    buf.extend_from_slice(&current_offset.to_le_bytes());
    let embd_size = quant_type.tensor_size_bytes(32000 * u64::from(embedding_len)).unwrap_or(0);
    current_offset += embd_size;

    // Layer tensors
    for i in 0..layers {
        let q_name = format!("blk.{i}.attn_q.weight");
        write_str(&mut buf, &q_name);
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
        buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
        buf.extend_from_slice(&(quant_type as u32).to_le_bytes());
        buf.extend_from_slice(&current_offset.to_le_bytes());
        let q_size = quant_type.tensor_size_bytes(u64::from(embedding_len) * u64::from(embedding_len)).unwrap_or(0);
        current_offset += q_size;

        let v_name = format!("blk.{i}.attn_v.weight");
        write_str(&mut buf, &v_name);
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
        buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
        buf.extend_from_slice(&(quant_type as u32).to_le_bytes());
        buf.extend_from_slice(&current_offset.to_le_bytes());
        let v_size = quant_type.tensor_size_bytes(u64::from(embedding_len) * u64::from(embedding_len)).unwrap_or(0);
        current_offset += v_size;
    }

    // Output head tensor
    write_str(&mut buf, "output.weight");
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&(u64::from(embedding_len)).to_le_bytes());
    buf.extend_from_slice(&32000u64.to_le_bytes());
    buf.extend_from_slice(&(quant_type as u32).to_le_bytes());
    buf.extend_from_slice(&current_offset.to_le_bytes());

    buf
}
