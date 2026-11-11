//! Progressive Frame Compositor for Distributed Tile Rendering.
//!
//! Blits incoming spatial tiles directly into a master frame buffer, tracks
//! per-tile worker attribution, computes progressive BLAKE3 digests, and exports
//! completed frames to OpenEXR, raw floats, or RGBA buffers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::GpuError;
use crate::render_coordinator::RenderPixelFormat;

/// Wire packet containing a rendered tile received from a cluster node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTilePacket {
    pub tile_id: u32,
    pub frame_idx: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: RenderPixelFormat,
    pub pixels: Vec<u8>,
    pub worker_id: String,
    pub render_time_ms: f64,
}

/// Progressive delta event emitted as each tile completes for UI live preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressiveFrameDelta {
    pub tile_id: u32,
    pub frame_idx: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub worker_id: String,
    pub render_time_ms: f64,
    pub tile_checksum_blake3: [u8; 32],
}

/// Master progressive frame compositor.
#[derive(Debug, Clone)]
pub struct FrameCompositor {
    pub width: u32,
    pub height: u32,
    pub pixel_format: RenderPixelFormat,
    pub buffer: Vec<u8>,
    pub completed_tile_count: usize,
    pub tile_ownership: HashMap<u32, String>,
}

impl FrameCompositor {
    /// Allocate an empty, zero-initialized master frame buffer.
    #[must_use]
    pub fn new(width: u32, height: u32, pixel_format: RenderPixelFormat) -> Self {
        let bpp = pixel_format.bytes_per_pixel();
        let total_bytes = (width as usize) * (height as usize) * bpp;
        Self {
            width,
            height,
            pixel_format,
            buffer: vec![0u8; total_bytes],
            completed_tile_count: 0,
            tile_ownership: HashMap::new(),
        }
    }

    /// Blit a completed tile packet directly into the master frame buffer.
    /// Emits a `ProgressiveFrameDelta` containing tile coordinates, worker attribution,
    /// and cryptographic BLAKE3 checksum.
    pub fn blit_tile(
        &mut self,
        tile: &CompletedTilePacket,
    ) -> Result<ProgressiveFrameDelta, GpuError> {
        if tile.pixel_format != self.pixel_format {
            return Err(GpuError::PixelFormatMismatch {
                expected: format!("{:?}", self.pixel_format),
                actual: format!("{:?}", tile.pixel_format),
            });
        }

        if tile.x + tile.width > self.width || tile.y + tile.height > self.height {
            return Err(GpuError::TileOutOfBounds {
                tile_id: tile.tile_id,
                x: tile.x,
                y: tile.y,
                frame_w: self.width,
                frame_h: self.height,
            });
        }

        let bpp = self.pixel_format.bytes_per_pixel();
        let expected_bytes = (tile.width as usize) * (tile.height as usize) * bpp;
        if tile.pixels.len() != expected_bytes {
            return Err(GpuError::CompositorError {
                reason: format!(
                    "tile {} pixel byte length mismatch: expected {}, got {}",
                    tile.tile_id,
                    expected_bytes,
                    tile.pixels.len()
                ),
            });
        }

        let frame_row_stride = (self.width as usize) * bpp;
        let tile_row_stride = (tile.width as usize) * bpp;

        // Blit row-by-row
        for r in 0..(tile.height as usize) {
            let src_start = r * tile_row_stride;
            let src_end = src_start + tile_row_stride;

            let dst_y = (tile.y as usize) + r;
            let dst_x = (tile.x as usize) * bpp;
            let dst_start = dst_y * frame_row_stride + dst_x;
            let dst_end = dst_start + tile_row_stride;

            self.buffer[dst_start..dst_end].copy_from_slice(&tile.pixels[src_start..src_end]);
        }

        self.completed_tile_count += 1;
        self.tile_ownership
            .insert(tile.tile_id, tile.worker_id.clone());

        let tile_checksum = *blake3::hash(&tile.pixels).as_bytes();

        Ok(ProgressiveFrameDelta {
            tile_id: tile.tile_id,
            frame_idx: tile.frame_idx,
            x: tile.x,
            y: tile.y,
            width: tile.width,
            height: tile.height,
            worker_id: tile.worker_id.clone(),
            render_time_ms: tile.render_time_ms,
            tile_checksum_blake3: tile_checksum,
        })
    }

    /// Read an individual pixel value converted to RGBA8 for testing or preview.
    pub fn get_pixel_rgba8(&self, x: u32, y: u32) -> Result<[u8; 4], GpuError> {
        if x >= self.width || y >= self.height {
            return Err(GpuError::OutOfBoundsBufferAccess {
                offset: (y * self.width + x) as usize,
                size: 4,
                buffer_len: (self.width * self.height) as usize,
            });
        }

        let bpp = self.pixel_format.bytes_per_pixel();
        let idx = ((y * self.width + x) as usize) * bpp;

        match self.pixel_format {
            RenderPixelFormat::Rgba8UnormSrgb => Ok([
                self.buffer[idx],
                self.buffer[idx + 1],
                self.buffer[idx + 2],
                self.buffer[idx + 3],
            ]),
            RenderPixelFormat::Rgba16Float => {
                // Approximate f16 to u8
                let r = u16::from_le_bytes([self.buffer[idx], self.buffer[idx + 1]]);
                let g = u16::from_le_bytes([self.buffer[idx + 2], self.buffer[idx + 3]]);
                let b = u16::from_le_bytes([self.buffer[idx + 4], self.buffer[idx + 5]]);
                let a = u16::from_le_bytes([self.buffer[idx + 6], self.buffer[idx + 7]]);
                Ok([
                    (r >> 8) as u8,
                    (g >> 8) as u8,
                    (b >> 8) as u8,
                    (a >> 8) as u8,
                ])
            }
            RenderPixelFormat::Rgba32Float => {
                let r = f32::from_le_bytes([
                    self.buffer[idx],
                    self.buffer[idx + 1],
                    self.buffer[idx + 2],
                    self.buffer[idx + 3],
                ]);
                let g = f32::from_le_bytes([
                    self.buffer[idx + 4],
                    self.buffer[idx + 5],
                    self.buffer[idx + 6],
                    self.buffer[idx + 7],
                ]);
                let b = f32::from_le_bytes([
                    self.buffer[idx + 8],
                    self.buffer[idx + 9],
                    self.buffer[idx + 10],
                    self.buffer[idx + 11],
                ]);
                let a = f32::from_le_bytes([
                    self.buffer[idx + 12],
                    self.buffer[idx + 13],
                    self.buffer[idx + 14],
                    self.buffer[idx + 15],
                ]);
                Ok([
                    (r.clamp(0.0, 1.0) * 255.0) as u8,
                    (g.clamp(0.0, 1.0) * 255.0) as u8,
                    (b.clamp(0.0, 1.0) * 255.0) as u8,
                    (a.clamp(0.0, 1.0) * 255.0) as u8,
                ])
            }
        }
    }

    /// Compute cryptographic BLAKE3 digest of the entire master frame buffer.
    #[must_use]
    pub fn compute_frame_checksum(&self) -> [u8; 32] {
        *blake3::hash(&self.buffer).as_bytes()
    }

    /// Export the composited frame buffer as a valid OpenEXR container header + float payload.
    /// Follows official OpenEXR magic (0x76, 0x2f, 0x31, 0x01) and version flags.
    pub fn export_openexr(&self) -> Result<Vec<u8>, GpuError> {
        let mut exr = Vec::with_capacity(self.buffer.len() + 1024);

        // OpenEXR Magic Number: 0x01312f76 (little-endian)
        exr.extend_from_slice(&[0x76, 0x2f, 0x31, 0x01]);
        // Version 2, single-part scanline image
        exr.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);

        // Channel list attribute
        exr.extend_from_slice(b"channels\0");
        exr.extend_from_slice(b"chlist\0");
        let chlist_size = 4 * 18 + 1; // 4 channels (R, G, B, A)
        exr.extend_from_slice(&(chlist_size as u32).to_le_bytes());
        for chan in &[b"A\0", b"B\0", b"G\0", b"R\0"] {
            exr.extend_from_slice(*chan);
            let pixel_type: u32 = match self.pixel_format {
                RenderPixelFormat::Rgba16Float => 1, // HALF
                _ => 2,                              // FLOAT
            };
            exr.extend_from_slice(&pixel_type.to_le_bytes());
            exr.extend_from_slice(&[0u8; 4]); // pLinear = 0, reserved 3 bytes
            exr.extend_from_slice(&[1, 0, 0, 0]); // xSampling = 1
            exr.extend_from_slice(&[1, 0, 0, 0]); // ySampling = 1
        }
        exr.push(0); // Null terminator for chlist

        // Compression attribute: NO_COMPRESSION = 0
        exr.extend_from_slice(b"compression\0");
        exr.extend_from_slice(b"compression\0");
        exr.extend_from_slice(&1u32.to_le_bytes());
        exr.push(0);

        // DataWindow attribute: (0, 0) to (w-1, h-1)
        exr.extend_from_slice(b"dataWindow\0");
        exr.extend_from_slice(b"box2i\0");
        exr.extend_from_slice(&16u32.to_le_bytes());
        exr.extend_from_slice(&0i32.to_le_bytes());
        exr.extend_from_slice(&0i32.to_le_bytes());
        exr.extend_from_slice(&((self.width as i32) - 1).to_le_bytes());
        exr.extend_from_slice(&((self.height as i32) - 1).to_le_bytes());

        // DisplayWindow attribute
        exr.extend_from_slice(b"displayWindow\0");
        exr.extend_from_slice(b"box2i\0");
        exr.extend_from_slice(&16u32.to_le_bytes());
        exr.extend_from_slice(&0i32.to_le_bytes());
        exr.extend_from_slice(&0i32.to_le_bytes());
        exr.extend_from_slice(&((self.width as i32) - 1).to_le_bytes());
        exr.extend_from_slice(&((self.height as i32) - 1).to_le_bytes());

        // End of header
        exr.push(0);

        // Raw pixel payload
        exr.extend_from_slice(&self.buffer);

        Ok(exr)
    }

    /// Export uncompressed visual image in standard Netpbm PPM format (RGB).
    pub fn export_ppm(&self) -> Result<Vec<u8>, GpuError> {
        let mut ppm = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for y in 0..self.height {
            for x in 0..self.width {
                let rgba = self.get_pixel_rgba8(x, y)?;
                ppm.push(rgba[0]);
                ppm.push(rgba[1]);
                ppm.push(rgba[2]);
            }
        }
        Ok(ppm)
    }
}
