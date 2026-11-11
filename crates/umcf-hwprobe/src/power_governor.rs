//! Mobile battery & radio wakeup governor for UMCF nodes.
//! Dynamically throttles task execution and coalesces network keepalives into deep-sleep radio windows.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use umcf_core::capabilities::ThermalState;

/// Errors relating to power governor decisions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PowerGovernorError {
    #[error("Task rejected due to power state {0:?}: battery conservation active")]
    TaskRejectedPowerSaving(PowerState),

    #[error("Device thermal throttling critical: temperature {0:?}")]
    ThermalEmergency(ThermalState),
}

/// Power state governing node participation in distributed workloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerState {
    /// Connected to AC charger on unmetered link: unrestricted maximum throughput.
    AcPoweredUnrestricted,
    /// Operating on battery with user actively interacting with the foreground app.
    BatteryActiveInteractive,
    /// Operating on battery with app backgrounded (> 25% battery): 60s radio coalescing.
    BatteryBackgroundEco,
    /// Operating on low battery (<= 20%) or OS low power mode: passive mode, 300s window.
    BatteryDeepSaver,
}

/// Category of computational work submitted to the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskKind {
    LightweightMetadata,
    LocalUserRequest,
    RemoteWasmJob,
    RemoteGpuJob,
    RemoteMediaTranscode,
    StorageScrubRepair,
}

/// Physical power and platform context observed from OS drivers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceTelemetry {
    pub battery_percent: f32,
    pub is_charging: bool,
    pub is_app_interactive: bool,
    pub os_low_power_mode: bool,
    pub thermal_state: ThermalState,
}

/// Timer coalescer synchronizing disparate keepalive loops into single radio wakeup windows.
#[derive(Debug, Clone, Default)]
pub struct TimerCoalescer {
    window_interval_secs: u64,
    registered_timers: HashMap<String, u64>,
}

impl TimerCoalescer {
    /// Constructs a timer coalescer with the specified synchronization window interval.
    pub fn new(window_interval_secs: u64) -> Self {
        Self {
            window_interval_secs: window_interval_secs.max(1),
            registered_timers: HashMap::new(),
        }
    }

    /// Registers a background task with an approximate desired period.
    pub fn register_task(&mut self, task_name: impl Into<String>, desired_period_secs: u64) {
        self.registered_timers
            .insert(task_name.into(), desired_period_secs);
    }

    /// Computes the next synchronized wall-clock wakeup tick.
    pub fn next_wakeup_tick(&self, current_time_secs: u64) -> u64 {
        let remainder = current_time_secs % self.window_interval_secs;
        if remainder == 0 {
            current_time_secs + self.window_interval_secs
        } else {
            current_time_secs + (self.window_interval_secs - remainder)
        }
    }

    /// Calculates total radio wakeup events over a given duration.
    pub fn total_wakeups_in_duration(&self, duration_secs: u64) -> u64 {
        (duration_secs / self.window_interval_secs) + 1
    }
}

/// Mobile Power Governor enforcing strict battery conservation invariants.
#[derive(Debug, Clone)]
pub struct MobilePowerGovernor {
    current_state: PowerState,
    coalescer: TimerCoalescer,
    total_wakeups_recorded: u64,
}

impl Default for MobilePowerGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl MobilePowerGovernor {
    /// Constructs a new MobilePowerGovernor starting in `AcPoweredUnrestricted`.
    pub fn new() -> Self {
        let mut coalescer = TimerCoalescer::new(5);
        coalescer.register_task("gossip_heartbeat", 5);
        coalescer.register_task("telemetry_pulse", 5);

        Self {
            current_state: PowerState::AcPoweredUnrestricted,
            coalescer,
            total_wakeups_recorded: 0,
        }
    }

    /// Returns the current power state.
    pub fn current_state(&self) -> PowerState {
        self.current_state
    }

    /// Returns the total number of wakeups recorded.
    pub fn total_wakeups(&self) -> u64 {
        self.total_wakeups_recorded
    }

    /// Records a radio wakeup event.
    pub fn record_wakeup(&mut self) {
        self.total_wakeups_recorded += 1;
    }

    /// Evaluates device telemetry and transitions the power state machine.
    pub fn evaluate_state(&mut self, telemetry: &DeviceTelemetry) -> PowerState {
        let next_state = if telemetry.is_charging {
            PowerState::AcPoweredUnrestricted
        } else if telemetry.os_low_power_mode || telemetry.battery_percent <= 20.0 {
            PowerState::BatteryDeepSaver
        } else if telemetry.is_app_interactive {
            PowerState::BatteryActiveInteractive
        } else {
            PowerState::BatteryBackgroundEco
        };

        if next_state != self.current_state {
            self.current_state = next_state;
            self.reconfigure_coalescer(next_state);
        }

        self.current_state
    }

    /// Reconfigures timer coalescing windows according to active power state.
    fn reconfigure_coalescer(&mut self, state: PowerState) {
        let window_secs = match state {
            PowerState::AcPoweredUnrestricted => 5,
            PowerState::BatteryActiveInteractive => 15,
            PowerState::BatteryBackgroundEco => 60,
            PowerState::BatteryDeepSaver => 300,
        };

        self.coalescer = TimerCoalescer::new(window_secs);
        self.coalescer
            .register_task("gossip_heartbeat", window_secs);
        self.coalescer.register_task("telemetry_pulse", window_secs);
    }

    /// Checks if a task of a given kind can be accepted under the current power state.
    pub fn can_accept_task(&self, task: TaskKind) -> Result<(), PowerGovernorError> {
        match self.current_state {
            PowerState::AcPoweredUnrestricted => Ok(()),
            PowerState::BatteryActiveInteractive => match task {
                TaskKind::StorageScrubRepair => Err(PowerGovernorError::TaskRejectedPowerSaving(
                    self.current_state,
                )),
                _ => Ok(()),
            },
            PowerState::BatteryBackgroundEco => match task {
                TaskKind::LightweightMetadata | TaskKind::LocalUserRequest => Ok(()),
                _ => Err(PowerGovernorError::TaskRejectedPowerSaving(
                    self.current_state,
                )),
            },
            PowerState::BatteryDeepSaver => match task {
                TaskKind::LocalUserRequest => Ok(()),
                _ => Err(PowerGovernorError::TaskRejectedPowerSaving(
                    self.current_state,
                )),
            },
        }
    }

    /// Returns the active gossip / heartbeat interval in seconds.
    pub fn heartbeat_interval_secs(&self) -> u64 {
        match self.current_state {
            PowerState::AcPoweredUnrestricted => 5,
            PowerState::BatteryActiveInteractive => 15,
            PowerState::BatteryBackgroundEco => 60,
            PowerState::BatteryDeepSaver => 300,
        }
    }

    /// Returns estimated battery impact in percentage points per hour.
    pub fn estimated_battery_drain_rate_pct_per_hr(&self) -> f32 {
        match self.current_state {
            PowerState::AcPoweredUnrestricted => 0.0,
            PowerState::BatteryActiveInteractive => 5.2,
            PowerState::BatteryBackgroundEco => 0.65, // < 1.0%/hr SLA
            PowerState::BatteryDeepSaver => 0.18,
        }
    }

    /// Accesses the underlying timer coalescer.
    pub fn coalescer(&self) -> &TimerCoalescer {
        &self.coalescer
    }
}
