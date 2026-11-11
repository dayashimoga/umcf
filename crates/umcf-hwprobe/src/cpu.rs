//! `umcf-hwprobe::cpu`: Cross-platform CPU topology, frequency, and SIMD instruction set probing.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::HwProbeError;
use std::fs;
use std::path::Path;
use sysinfo::{CpuRefreshKind, System};
use umcf_core::capabilities::CpuCapability;

/// Real host CPU capability and instruction set prober.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuProber;

impl CpuProber {
    /// Probes the physical host CPU, determining topology, clock frequencies, and SIMD instruction flags.
    ///
    /// # Errors
    /// Returns `HwProbeError` if CPU query fails.
    pub fn probe_cpu() -> Result<CpuCapability, HwProbeError> {
        let sys_lock = crate::system_probe::get_system();
        let mut sys = sys_lock
            .lock()
            .map_err(|e| HwProbeError::CpuProbe(e.to_string()))?;
        sys.refresh_cpu_specifics(CpuRefreshKind::nothing());
        Self::probe_cpu_with_sys(&sys)
    }

    /// Probes CPU capability using an already acquired `System` reference.
    ///
    /// # Errors
    /// Returns `HwProbeError` if probing fails.
    pub fn probe_cpu_with_sys(sys: &System) -> Result<CpuCapability, HwProbeError> {
        let arch = std::env::consts::ARCH.to_string();

        let cpus = sys.cpus();
        let logical_cores = cpus.len() as u32;
        let physical_cores = sys.physical_core_count().unwrap_or(logical_cores as usize) as u32;

        let mut max_freq = 0_u64;
        let mut min_freq = u64::MAX;
        for cpu in cpus {
            let freq = cpu.frequency();
            if freq > max_freq {
                max_freq = freq;
            }
            if freq > 0 && freq < min_freq {
                min_freq = freq;
            }
        }

        if min_freq == u64::MAX {
            min_freq = max_freq;
        }

        // Detect P-cores vs E-cores: if frequency disparity > 25%, split cores
        let (p_cores, e_cores) = if max_freq > 0 && min_freq > 0 && max_freq > min_freq * 125 / 100
        {
            let e_count = cpus
                .iter()
                .filter(|c| c.frequency() < (max_freq + min_freq) / 2)
                .count() as u32;
            let p_count = physical_cores.saturating_sub(e_count).max(1);
            (p_count, e_count)
        } else {
            (physical_cores.max(1), 0)
        };

        // Probe hardware SIMD flags using std::arch and sysfs
        let mut simd_avx2 = false;
        let mut simd_avx512 = false;
        let mut simd_neon = cfg!(target_arch = "aarch64");
        let mut simd_sve = false;

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                simd_avx2 = true;
            }
            if is_x86_feature_detected!("avx512f") {
                simd_avx512 = true;
            }
        }

        // On Linux / Android, inspect /proc/cpuinfo for additional hardware flags
        if Path::new("/proc/cpuinfo").exists() {
            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                let (_, flags) = Self::parse_cpuinfo(&content);
                for flag in flags {
                    let f = flag.to_lowercase();
                    if f == "avx2" {
                        simd_avx2 = true;
                    } else if f == "avx512f" || f == "avx512" {
                        simd_avx512 = true;
                    } else if !simd_neon && (f == "neon" || f == "asimd") {
                        simd_neon = true;
                    } else if f == "sve" || f == "sve2" {
                        simd_sve = true;
                    }
                }
            }
        }

        // Check cgroup v2 cpu.max if containerized
        let mut effective_cores = physical_cores.max(1);
        if let Ok(Some(quota_cores)) = Self::read_cgroup_cpu_limit() {
            let rounded_quota = quota_cores.ceil() as u32;
            if rounded_quota < effective_cores {
                effective_cores = rounded_quota.max(1);
            }
        }

        Ok(CpuCapability {
            architecture: arch,
            physical_cores: effective_cores,
            performance_cores: p_cores.min(effective_cores).max(1),
            efficiency_cores: e_cores.min(effective_cores.saturating_sub(p_cores)),
            simd_avx2,
            simd_avx512,
            simd_neon,
            simd_sve,
            base_clock_mhz: min_freq as u32,
            boost_clock_mhz: max_freq as u32,
        })
    }

    /// Reads and parses cgroup v2 `cpu.max` limit if running in a container.
    pub fn read_cgroup_cpu_limit() -> Result<Option<f32>, HwProbeError> {
        let cgroup_cpu_path = Path::new("/sys/fs/cgroup/cpu.max");
        if cgroup_cpu_path.exists() {
            let content = fs::read_to_string(cgroup_cpu_path)?;
            Ok(Self::parse_cgroup_cpu_max(&content))
        } else {
            Ok(None)
        }
    }

    /// Parses Linux `/proc/cpuinfo` format, returning (core_count, flags).
    #[must_use]
    pub fn parse_cpuinfo(content: &str) -> (u32, Vec<String>) {
        let mut count = 0_u32;
        let mut all_flags = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("processor") && line.contains(':') {
                count += 1;
            } else if (line.starts_with("flags") || line.starts_with("Features"))
                && line.contains(':')
            {
                if let Some(pos) = line.find(':') {
                    let flags_part = &line[pos + 1..];
                    for f in flags_part.split_whitespace() {
                        let flag_s = f.to_string();
                        if !all_flags.contains(&flag_s) {
                            all_flags.push(flag_s);
                        }
                    }
                }
            }
        }

        (count.max(1), all_flags)
    }

    /// Parses cgroup v2 `cpu.max` syntax: `<quota> <period>` (e.g. `200000 100000` -> 2.0 cores).
    #[must_use]
    pub fn parse_cgroup_cpu_max(content: &str) -> Option<f32> {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            if parts[0] == "max" {
                return None;
            }
            if let (Ok(quota), Ok(period)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                if period > 0.0 {
                    return Some(quota / period);
                }
            }
        }
        None
    }
}
