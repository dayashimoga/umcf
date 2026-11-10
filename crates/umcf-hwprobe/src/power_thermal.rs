//! `umcf-hwprobe::power_thermal`: Cross-platform battery status and thermal state probing.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::HwProbeError;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::Components;
use umcf_core::capabilities::{PowerThermalState, ThermalState};

struct CachedComponents {
    components: Components,
    last_refresh: Instant,
}

static GLOBAL_COMPONENTS: OnceLock<Mutex<CachedComponents>> = OnceLock::new();

/// Returns a reference to the global lazily initialized `Components` sensor list.
fn get_components() -> &'static Mutex<CachedComponents> {
    GLOBAL_COMPONENTS.get_or_init(|| {
        Mutex::new(CachedComponents {
            components: Components::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        })
    })
}

/// Warms up the global components sensor cache.
pub fn warmup_components() {
    let _ = get_components();
}

/// Battery power and device thermal prober.
#[derive(Debug, Clone, Copy, Default)]
pub struct PowerThermalProber;

impl PowerThermalProber {
    /// Probes battery level, charging state, and device thermal conditions.
    ///
    /// # Errors
    /// Returns `HwProbeError` if power or thermal probing fails.
    pub fn probe_power_thermal() -> Result<PowerThermalState, HwProbeError> {
        let (battery_percent, is_charging) = Self::probe_battery()?;
        let (thermal_state, temp_milli_celsius) = Self::probe_thermal()?;

        Ok(PowerThermalState {
            battery_percent,
            is_charging,
            thermal_state,
            temperature_milli_celsius: temp_milli_celsius,
        })
    }

    /// Probes battery state using Linux/Android sysfs or defaults to AC-powered for desktop/VM.
    pub fn probe_battery() -> Result<(Option<u8>, bool), HwProbeError> {
        let power_supply_base = Path::new("/sys/class/power_supply");
        if power_supply_base.exists() && power_supply_base.is_dir() {
            if let Some(result) = Self::read_sysfs_battery(power_supply_base)? {
                return Ok(result);
            }
        }

        // On desktop/server systems or containers without battery access, default to AC power
        Ok((None, true))
    }

    /// Reads battery status from a power_supply sysfs directory.
    pub fn read_sysfs_battery(base_dir: &Path) -> Result<Option<(Option<u8>, bool)>, HwProbeError> {
        if let Ok(entries) = fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                if file_name.starts_with("BAT") || file_name.contains("battery") {
                    let cap_path = path.join("capacity");
                    let status_path = path.join("status");

                    if cap_path.exists() {
                        let cap_str = fs::read_to_string(cap_path).unwrap_or_default();
                        let status_str = if status_path.exists() {
                            fs::read_to_string(status_path).unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let (pct, charging) = Self::parse_battery_sysfs(&cap_str, &status_str);
                        return Ok(Some((pct, charging)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Parses battery capacity and charging status strings.
    #[must_use]
    pub fn parse_battery_sysfs(capacity_str: &str, status_str: &str) -> (Option<u8>, bool) {
        let pct = capacity_str.trim().parse::<u8>().ok();
        let status = status_str.trim().to_lowercase();
        let is_charging = status == "charging" || status == "full" || status.is_empty();
        (pct, is_charging)
    }

    /// Probes device thermal sensors via `/sys/class/thermal` or `sysinfo::Components`.
    pub fn probe_thermal() -> Result<(ThermalState, Option<i32>), HwProbeError> {
        // 1. Try Linux / Android sysfs `/sys/class/thermal`
        let thermal_base = Path::new("/sys/class/thermal");
        if thermal_base.exists() && thermal_base.is_dir() {
            if let Some(temp_mc) = Self::read_sysfs_thermal(thermal_base)? {
                let state = Self::temperature_to_thermal_state(temp_mc);
                return Ok((state, Some(temp_mc)));
            }
        }

        // 2. Try sysinfo Components (works on Windows, macOS, Linux)
        let comp_lock = get_components();
        let mut cached = comp_lock
            .lock()
            .map_err(|e| HwProbeError::PowerThermalProbe(e.to_string()))?;

        if cached.last_refresh.elapsed() > Duration::from_millis(500) {
            cached.components.refresh(false);
            cached.last_refresh = Instant::now();
        }

        let mut max_temp_c = f32::MIN;
        let mut found = false;

        for comp in &cached.components {
            if let Some(temp) = comp.temperature() {
                if temp > max_temp_c && temp > 0.0 {
                    max_temp_c = temp;
                    found = true;
                }
            }
        }

        if found {
            let temp_mc = (max_temp_c * 1000.0) as i32;
            let state = Self::temperature_to_thermal_state(temp_mc);
            Ok((state, Some(temp_mc)))
        } else {
            // Safe nominal default for containers, VMs, or boards without temperature sensors
            Ok((ThermalState::Nominal, None))
        }
    }

    /// Reads maximum temperature from `/sys/class/thermal` directory in milli-Celsius.
    pub fn read_sysfs_thermal(base_dir: &Path) -> Result<Option<i32>, HwProbeError> {
        let mut max_temp: Option<i32> = None;

        if let Ok(entries) = fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                if file_name.starts_with("thermal_zone") {
                    let temp_path = path.join("temp");
                    if temp_path.exists() {
                        if let Ok(content) = fs::read_to_string(temp_path) {
                            if let Some(temp_mc) = Self::parse_thermal_temp(&content) {
                                max_temp = Some(max_temp.map_or(temp_mc, |m| m.max(temp_mc)));
                            }
                        }
                    }
                }
            }
        }

        Ok(max_temp)
    }

    /// Parses thermal sensor temperature string (milli-Celsius).
    #[must_use]
    pub fn parse_thermal_temp(content: &str) -> Option<i32> {
        content.trim().parse::<i32>().ok()
    }

    /// Maps temperature in milli-Celsius to discrete `ThermalState`.
    /// - < 60°C (60_000 mC) -> Nominal
    /// - 60°C - 75°C (60_000 - 75_000 mC) -> Fair
    /// - 75°C - 90°C (75_000 - 90_000 mC) -> Serious
    /// - >= 90°C (90_000 mC) -> Critical
    #[must_use]
    pub const fn temperature_to_thermal_state(milli_celsius: i32) -> ThermalState {
        if milli_celsius < 60_000 {
            ThermalState::Nominal
        } else if milli_celsius < 75_000 {
            ThermalState::Fair
        } else if milli_celsius < 90_000 {
            ThermalState::Serious
        } else {
            ThermalState::Critical
        }
    }
}
