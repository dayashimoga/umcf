//! Adaptive Tile & Frame Decomposition and Distributed Rendering Coordinator.
//!
//! Provides spatial tile decomposition along space-filling curves (Morton/Center-Spiral),
//! adaptive batch sizing by worker EWMA throughput, dynamic work-stealing, and
//! speculative tail execution for straggler elimination.

use std::collections::{HashMap, VecDeque};
use std::ops::RangeInclusive;
use serde::{Deserialize, Serialize};

use crate::error::GpuError;
use crate::shader_sandbox::ValidatedShader;

/// Supported pixel formats for distributed render targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderPixelFormat {
    Rgba8UnormSrgb,
    Rgba16Float,
    Rgba32Float,
}

impl RenderPixelFormat {
    #[must_use]
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba8UnormSrgb => 4,
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
        }
    }
}

/// Specification for a distributed rendering job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJobSpec {
    pub job_id: String,
    pub resolution: (u32, u32),
    pub frame_range: RangeInclusive<u32>,
    pub samples_per_pixel: u32,
    pub tile_size: (u32, u32),
    pub pixel_format: RenderPixelFormat,
    pub scene_asset_root: [u8; 32],
    pub shader_module: ValidatedShader,
}

/// Description of an individual rendering tile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileDescriptor {
    pub tile_id: u32,
    pub frame_idx: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub morton_index: u64,
    pub distance_to_center: f64,
}

impl TileDescriptor {
    #[must_use]
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }
}

/// State of a tile in the distributed execution lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TileState {
    Pending,
    InFlight {
        worker_id: String,
        dispatched_at_ms: u64,
        is_speculative: bool,
    },
    Completed {
        completed_by: String,
        duration_ms: f64,
    },
}

/// Interleave bits for 2D Morton (Z-order) curve index calculation.
fn part1by1(mut n: u32) -> u64 {
    n &= 0x0000_ffff;
    let mut x = u64::from(n);
    x = (x | (x << 8)) & 0x00ff_00ff;
    x = (x | (x << 4)) & 0x0f0f_0f0f;
    x = (x | (x << 2)) & 0x3333_3333;
    x = (x | (x << 1)) & 0x5555_5555;
    x
}

/// Calculate 2D Morton index (Z-order) for tile grid coordinates.
#[must_use]
pub fn morton_2d(x: u32, y: u32) -> u64 {
    (part1by1(y) << 1) | part1by1(x)
}

/// Performance record and EWMA throughput tracker for a worker node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerThroughputTracker {
    pub worker_id: String,
    pub name: String,
    pub relative_tflops: f64,
    pub ewma_pixels_per_ms: f64,
    pub completed_tiles_count: usize,
    pub total_pixels_rendered: u64,
}

impl WorkerThroughputTracker {
    #[must_use]
    pub fn new(worker_id: impl Into<String>, name: impl Into<String>, relative_tflops: f64) -> Self {
        // Initial estimate based on relative TFLOPS (baseline 1.0 TFLOPS ~= 20,000 pixels/ms)
        let initial_pixels_per_ms = (relative_tflops * 20_000.0).max(100.0);
        Self {
            worker_id: worker_id.into(),
            name: name.into(),
            relative_tflops,
            ewma_pixels_per_ms: initial_pixels_per_ms,
            completed_tiles_count: 0,
            total_pixels_rendered: 0,
        }
    }

    /// Update EWMA with a new measurement (alpha = 0.25).
    pub fn record_tile_completion(&mut self, pixels: u32, duration_ms: f64) {
        if duration_ms > 0.0 {
            let measured_rate = f64::from(pixels) / duration_ms;
            let alpha = 0.25;
            self.ewma_pixels_per_ms = alpha * measured_rate + (1.0 - alpha) * self.ewma_pixels_per_ms;
        }
        self.completed_tiles_count += 1;
        self.total_pixels_rendered += u64::from(pixels);
    }
}

/// Dynamic render coordinator managing tile queues, adaptive assignments,
/// and speculative tail mitigation.
#[derive(Debug)]
pub struct DistributedRenderCoordinator {
    pub spec: RenderJobSpec,
    pub tiles: Vec<TileDescriptor>,
    pub tile_states: Vec<TileState>,
    pub pending_queue: VecDeque<u32>,
    pub workers: HashMap<String, WorkerThroughputTracker>,
    pub speculative_dispatches_count: usize,
    pub canceled_redundant_tiles_count: usize,
    pub current_time_ms: u64,
}

impl DistributedRenderCoordinator {
    /// Create a new coordinator and decompose the frame into center-spiral ordered tiles.
    pub fn new(spec: RenderJobSpec) -> Result<Self, GpuError> {
        let (res_w, res_h) = spec.resolution;
        let (tile_w, tile_h) = spec.tile_size;

        if res_w == 0 || res_h == 0 {
            return Err(GpuError::InvalidInput {
                reason: "render resolution must be greater than 0".to_string(),
            });
        }
        if tile_w == 0 || tile_h == 0 {
            return Err(GpuError::InvalidInput {
                reason: "tile size must be greater than 0".to_string(),
            });
        }

        let num_tiles_x = res_w.div_ceil(tile_w);
        let num_tiles_y = res_h.div_ceil(tile_h);
        let mut tiles = Vec::with_capacity((num_tiles_x * num_tiles_y) as usize);

        let center_x = f64::from(res_w) / 2.0;
        let center_y = f64::from(res_h) / 2.0;

        let frame_idx = *spec.frame_range.start();
        let mut tile_id = 0u32;

        for ty in 0..num_tiles_y {
            for tx in 0..num_tiles_x {
                let x = tx * tile_w;
                let y = ty * tile_h;
                let width = (res_w - x).min(tile_w);
                let height = (res_h - y).min(tile_h);

                let tile_cx = f64::from(x) + (f64::from(width) / 2.0);
                let tile_cy = f64::from(y) + (f64::from(height) / 2.0);
                let distance_to_center = ((tile_cx - center_x).powi(2) + (tile_cy - center_y).powi(2)).sqrt();
                let morton_index = morton_2d(tx, ty);

                tiles.push(TileDescriptor {
                    tile_id,
                    frame_idx,
                    x,
                    y,
                    width,
                    height,
                    morton_index,
                    distance_to_center,
                });

                tile_id += 1;
            }
        }

        // Sort tiles in center-spiral order so focal center regions render first
        tiles.sort_by(|a, b| {
            a.distance_to_center
                .partial_cmp(&b.distance_to_center)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Reassign continuous tile IDs reflecting render priority order
        for (idx, tile) in tiles.iter_mut().enumerate() {
            tile.tile_id = idx as u32;
        }

        let total_tiles = tiles.len();
        let tile_states = vec![TileState::Pending; total_tiles];
        let pending_queue = (0..total_tiles as u32).collect();

        Ok(Self {
            spec,
            tiles,
            tile_states,
            pending_queue,
            workers: HashMap::new(),
            speculative_dispatches_count: 0,
            canceled_redundant_tiles_count: 0,
            current_time_ms: 0,
        })
    }

    /// Register a worker node.
    pub fn register_worker(&mut self, worker_id: impl Into<String>, name: impl Into<String>, relative_tflops: f64) {
        let tracker = WorkerThroughputTracker::new(worker_id, name, relative_tflops);
        self.workers.insert(tracker.worker_id.clone(), tracker);
    }

    /// Compute adaptive batch size (number of tiles) for a worker based on its relative TFLOPS.
    #[must_use]
    pub fn adaptive_batch_size(&self, worker_id: &str) -> usize {
        if let Some(w) = self.workers.get(worker_id) {
            if w.relative_tflops >= 8.0 {
                4 // Powerful desktop GPU gets batches of 4 tiles
            } else if w.relative_tflops >= 3.0 {
                2 // Mid-range GPU gets batches of 2 tiles
            } else {
                1 // Mobile or entry GPU gets 1 tile
            }
        } else {
            1
        }
    }

    /// Advance simulation or wall-clock timestamp.
    pub fn set_current_time_ms(&mut self, time_ms: u64) {
        self.current_time_ms = time_ms;
    }

    /// Check if speculative tail execution should trigger for a worker.
    /// Triggers when >= 95% of tiles are completed and an in-flight tile is running
    /// past 1.5x expected completion time.
    #[must_use]
    pub fn find_straggler_tile(&self) -> Option<u32> {
        let total = self.tiles.len();
        if total == 0 {
            return None;
        }

        let completed = self
            .tile_states
            .iter()
            .filter(|s| matches!(s, TileState::Completed { .. }))
            .count();

        let completion_ratio = completed as f64 / total as f64;
        if completion_ratio < 0.95 {
            return None; // Speculative execution only on the tail >= 95%
        }

        // Find fastest worker's expected tile duration
        let fastest_rate = self
            .workers
            .values()
            .map(|w| w.ewma_pixels_per_ms)
            .fold(0.0f64, f64::max);

        let default_tile_pixels = f64::from(self.spec.tile_size.0 * self.spec.tile_size.1);
        let fastest_tile_ms = if fastest_rate > 0.0 {
            default_tile_pixels / fastest_rate
        } else {
            10.0
        };

        // If in-flight duration > 1.5 * fastest_tile_ms, mark as straggler
        for (tile_id, state) in self.tile_states.iter().enumerate() {
            if let TileState::InFlight {
                dispatched_at_ms,
                is_speculative,
                ..
            } = state
            {
                if !*is_speculative {
                    let elapsed = self.current_time_ms.saturating_sub(*dispatched_at_ms) as f64;
                    if elapsed > (1.5 * fastest_tile_ms) {
                        return Some(tile_id as u32);
                    }
                }
            }
        }

        None
    }

    /// Dispatch next batch of tiles for a worker.
    /// If pending queue is empty and stragglers exist in the tail (>= 95%),
    /// speculatively dispatches the straggler tile to an idle fast worker.
    pub fn dispatch_next_batch(&mut self, worker_id: &str) -> Vec<TileDescriptor> {
        let batch_size = self.adaptive_batch_size(worker_id);
        let mut dispatched = Vec::with_capacity(batch_size);

        // 1. Try to take from pending queue
        while dispatched.len() < batch_size {
            if let Some(tile_id) = self.pending_queue.pop_front() {
                let id = tile_id as usize;
                self.tile_states[id] = TileState::InFlight {
                    worker_id: worker_id.to_string(),
                    dispatched_at_ms: self.current_time_ms,
                    is_speculative: false,
                };
                dispatched.push(self.tiles[id].clone());
            } else {
                break;
            }
        }

        // 2. If worker still has capacity, check for straggler tail tiles for speculative re-dispatch
        if dispatched.is_empty() {
            if let Some(straggler_tile_id) = self.find_straggler_tile() {
                let id = straggler_tile_id as usize;
                self.tile_states[id] = TileState::InFlight {
                    worker_id: worker_id.to_string(),
                    dispatched_at_ms: self.current_time_ms,
                    is_speculative: true,
                };
                self.speculative_dispatches_count += 1;
                dispatched.push(self.tiles[id].clone());
            }
        }

        dispatched
    }

    /// Record tile completion from a worker.
    /// If tile was already completed by a faster or duplicate speculative worker,
    /// discards redundant result.
    pub fn record_tile_completion(
        &mut self,
        tile_id: u32,
        worker_id: &str,
        duration_ms: f64,
    ) -> Result<bool, GpuError> {
        let id = tile_id as usize;
        if id >= self.tiles.len() {
            return Err(GpuError::TileOutOfBounds {
                tile_id,
                x: 0,
                y: 0,
                frame_w: self.spec.resolution.0,
                frame_h: self.spec.resolution.1,
            });
        }

        let tile = &self.tiles[id];
        let pixels = tile.pixel_count();

        match &self.tile_states[id] {
            TileState::Completed { .. } => {
                // Redundant completion from slower or canceled worker
                self.canceled_redundant_tiles_count += 1;
                Ok(false)
            }
            TileState::InFlight { .. } | TileState::Pending => {
                self.tile_states[id] = TileState::Completed {
                    completed_by: worker_id.to_string(),
                    duration_ms,
                };

                if let Some(worker) = self.workers.get_mut(worker_id) {
                    worker.record_tile_completion(pixels, duration_ms);
                }

                Ok(true)
            }
        }
    }

    /// Check if all tiles in the current frame are completed.
    #[must_use]
    pub fn is_frame_complete(&self) -> bool {
        self.tile_states
            .iter()
            .all(|s| matches!(s, TileState::Completed { .. }))
    }

    /// Compute overall progress (0.0 to 1.0) and tile counts.
    #[must_use]
    pub fn progress(&self) -> (usize, usize, f64) {
        let total = self.tiles.len();
        let completed = self
            .tile_states
            .iter()
            .filter(|s| matches!(s, TileState::Completed { .. }))
            .count();
        let ratio = if total > 0 {
            completed as f64 / total as f64
        } else {
            1.0
        };
        (completed, total, ratio)
    }
}
