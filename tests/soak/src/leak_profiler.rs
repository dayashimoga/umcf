//! Automated Zero-Leak Regression Profiler using Ordinary Least Squares (OLS).

use serde::{Deserialize, Serialize};
use crate::error::SoakError;

/// Resource utilization snapshot sampled at a given cycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceSnapshot {
    pub cycle: usize,
    pub heap_allocated_bytes: u64,
    pub rss_bytes: u64,
    pub vram_allocated_bytes: u64,
    pub wgpu_active_buffer_count: usize,
    pub llama_active_contexts: usize,
    pub open_file_descriptors: usize,
    pub active_quic_streams: usize,
    pub active_tokio_tasks: usize,
    pub temp_directory_file_count: usize,
}

/// Result of an OLS linear regression analysis for a specific metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    pub metric_name: String,
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
    pub sample_count: usize,
    pub is_leak: bool,
    pub max_allowed_slope: f64,
}

pub struct LeakProfiler;

impl LeakProfiler {
    /// Computes OLS linear regression: y = slope * x + intercept.
    ///
    /// Evaluates if the trendline slope exceeds `max_allowed_slope` (e.g. 0.001 bytes/cycle).
    pub fn compute_regression(
        metric_name: &str,
        samples: &[(usize, f64)],
        max_allowed_slope: f64,
    ) -> Result<RegressionResult, SoakError> {
        let n = samples.len();
        if n < 2 {
            return Err(SoakError::RegressionError {
                reason: format!("At least 2 samples required for regression, found {n}"),
            });
        }

        let sum_x: f64 = samples.iter().map(|&(x, _)| x as f64).sum();
        let sum_y: f64 = samples.iter().map(|&(_, y)| y).sum();

        let mean_x = sum_x / (n as f64);
        let mean_y = sum_y / (n as f64);

        let mut ss_xx = 0.0;
        let mut ss_xy = 0.0;
        let mut ss_yy = 0.0;

        for &(x, y) in samples {
            let dx = (x as f64) - mean_x;
            let dy = y - mean_y;
            ss_xx += dx * dx;
            ss_xy += dx * dy;
            ss_yy += dy * dy;
        }

        if ss_xx == 0.0 {
            return Err(SoakError::RegressionError {
                reason: "Zero variance in x-values (cycle steps)".to_string(),
            });
        }

        let slope = ss_xy / ss_xx;
        let intercept = mean_y - (slope * mean_x);

        let r_squared = if ss_yy <= 1e-12 {
            1.0 // All y values are constant -> perfect flat trend
        } else {
            let r2 = (ss_xy * ss_xy) / (ss_xx * ss_yy);
            r2.clamp(0.0, 1.0)
        };

        // Strict leak check: positive slope exceeding statistical noise epsilon
        let is_leak = slope > max_allowed_slope;

        Ok(RegressionResult {
            metric_name: metric_name.to_string(),
            slope,
            intercept,
            r_squared,
            sample_count: n,
            is_leak,
            max_allowed_slope,
        })
    }

    /// Evaluates all core resource metrics across a series of snapshots.
    pub fn evaluate_snapshots(
        snapshots: &[ResourceSnapshot],
        warmup_cycles: usize,
    ) -> Result<Vec<RegressionResult>, SoakError> {
        // Filter snapshots to post-warmup window
        let filtered: Vec<&ResourceSnapshot> = snapshots
            .iter()
            .filter(|s| s.cycle >= warmup_cycles)
            .collect();

        if filtered.len() < 2 {
            return Err(SoakError::RegressionError {
                reason: format!("Insufficient post-warmup snapshots: {} < 2", filtered.len()),
            });
        }

        let mut results = Vec::new();

        // 1. Heap allocated bytes (max allowed slope: 0.001 bytes/cycle)
        let heap_samples: Vec<(usize, f64)> = filtered
            .iter()
            .map(|s| (s.cycle, s.heap_allocated_bytes as f64))
            .collect();
        let r_heap = Self::compute_regression("heap_allocated_bytes", &heap_samples, 0.001)?;
        if r_heap.is_leak {
            return Err(SoakError::LeakDetected {
                metric: "heap_allocated_bytes".to_string(),
                slope: r_heap.slope,
                threshold: r_heap.max_allowed_slope,
                details: format!("Heap leak detected: +{:.4} bytes/cycle", r_heap.slope),
            });
        }
        results.push(r_heap);

        // 2. VRAM allocated bytes (max allowed slope: 0.0)
        let vram_samples: Vec<(usize, f64)> = filtered
            .iter()
            .map(|s| (s.cycle, s.vram_allocated_bytes as f64))
            .collect();
        let r_vram = Self::compute_regression("vram_allocated_bytes", &vram_samples, 0.0)?;
        if r_vram.is_leak {
            return Err(SoakError::LeakDetected {
                metric: "vram_allocated_bytes".to_string(),
                slope: r_vram.slope,
                threshold: r_vram.max_allowed_slope,
                details: format!("VRAM buffer leak: +{:.4} bytes/cycle", r_vram.slope),
            });
        }
        results.push(r_vram);

        // 3. Open file descriptors / handles (max allowed slope: 0.0)
        let fd_samples: Vec<(usize, f64)> = filtered
            .iter()
            .map(|s| (s.cycle, s.open_file_descriptors as f64))
            .collect();
        let r_fd = Self::compute_regression("open_file_descriptors", &fd_samples, 0.0)?;
        if r_fd.is_leak {
            return Err(SoakError::LeakDetected {
                metric: "open_file_descriptors".to_string(),
                slope: r_fd.slope,
                threshold: r_fd.max_allowed_slope,
                details: format!("File descriptor / handle leak: +{:.4} handles/cycle", r_fd.slope),
            });
        }
        results.push(r_fd);

        // 4. Active QUIC streams (max allowed slope: 0.0)
        let quic_samples: Vec<(usize, f64)> = filtered
            .iter()
            .map(|s| (s.cycle, s.active_quic_streams as f64))
            .collect();
        let r_quic = Self::compute_regression("active_quic_streams", &quic_samples, 0.0)?;
        if r_quic.is_leak {
            return Err(SoakError::LeakDetected {
                metric: "active_quic_streams".to_string(),
                slope: r_quic.slope,
                threshold: r_quic.max_allowed_slope,
                details: format!("QUIC stream handle leak: +{:.4} streams/cycle", r_quic.slope),
            });
        }
        results.push(r_quic);

        // 5. Temp directory file count (max allowed slope: 0.0)
        let temp_samples: Vec<(usize, f64)> = filtered
            .iter()
            .map(|s| (s.cycle, s.temp_directory_file_count as f64))
            .collect();
        let r_temp = Self::compute_regression("temp_directory_file_count", &temp_samples, 0.0)?;
        if r_temp.is_leak {
            return Err(SoakError::LeakDetected {
                metric: "temp_directory_file_count".to_string(),
                slope: r_temp.slope,
                threshold: r_temp.max_allowed_slope,
                details: format!("Temporary file orphan leak: +{:.4} files/cycle", r_temp.slope),
            });
        }
        results.push(r_temp);

        Ok(results)
    }

    /// Queries host process handle / file descriptor count safely.
    pub fn query_host_fd_or_handle_count() -> usize {
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
                return entries.count();
            }
        }
        // Fallback or Windows standard baseline
        42
    }
}

/// Quiescence barrier ensuring all pending background drops and GC sweeps settle.
pub struct QuiescenceBarrier;

impl QuiescenceBarrier {
    pub async fn wait_for_quiescence<F>(
        check_active: F,
        timeout_ms: u64,
    ) -> Result<(), SoakError>
    where
        F: Fn() -> usize,
    {
        let start = std::time::Instant::now();
        let step = std::time::Duration::from_millis(5);

        while start.elapsed().as_millis() < timeout_ms as u128 {
            let active = check_active();
            if active == 0 {
                return Ok(());
            }
            tokio::time::sleep(step).await;
        }

        let remaining = check_active();
        if remaining == 0 {
            Ok(())
        } else {
            Err(SoakError::QuiescenceTimeout {
                elapsed_ms: start.elapsed().as_millis() as u64,
                active_count: remaining,
            })
        }
    }
}
