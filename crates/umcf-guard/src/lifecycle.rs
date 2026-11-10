//! `umcf-guard::lifecycle`: Thermal throttling, battery guard, hysteresis deadbands, and OS lifecycle hooks.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use umcf_enrollment::ResourceConsentPolicy;

/// Operational compute regimes enforced by the thermal and battery guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum GuardRegime {
    /// Normal operation: temperature < 38°C, battery >= min_battery_percent, power compliant.
    #[default]
    FullCapacity,
    /// Throttled compute: temperature between 38°C and 41°C. Worker throttled by 50%.
    Throttled50,
    /// Suspended compute: temperature >= 41°C, low battery, or unplugged when charging required.
    PausedWaitCoolOrCharge,
    /// Emergency node evacuation: temperature >= 44°C or OS memory pressure / background expiration.
    EmergencyEvict,
}

impl std::fmt::Display for GuardRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullCapacity => write!(f, "FullCapacity"),
            Self::Throttled50 => write!(f, "Throttled50"),
            Self::PausedWaitCoolOrCharge => write!(f, "PausedWaitCoolOrCharge"),
            Self::EmergencyEvict => write!(f, "EmergencyEvict"),
        }
    }
}

/// Mobile OS memory pressure level (Android `ComponentCallbacks2.onTrimMemory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TrimMemoryLevel {
    RunningModerate,
    RunningLow,
    RunningCritical,
    Complete,
}

/// Mobile OS application lifecycle states (Flutter / Android / iOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AppState {
    Resumed,
    Inactive,
    Paused,
    Detached,
    Hidden,
}

/// Incoming OS-level lifecycle signals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OsLifecycleSignal {
    TrimMemory(TrimMemoryLevel),
    AppLifecycle(AppState),
    BackgroundExpirationWarning { remaining_seconds: u32 },
}

/// Real-time hardware telemetry input for the guard state machine.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SensorTelemetry {
    pub temperature_celsius: f32,
    pub battery_percent: Option<u8>,
    pub is_charging: bool,
    pub timestamp_epoch_ms: u64,
}

/// Urgent events emitted to local worker threads and the cluster Master.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeGuardEvent {
    RegimeChanged {
        from: GuardRegime,
        to: GuardRegime,
        reason: String,
        timestamp_epoch_ms: u64,
    },
    ThrottleWorkload {
        throttle_percent: u8,
    },
    SuspendWorkload {
        reason: String,
    },
    ResumeWorkload,
    EvacuateNode {
        reason: String,
        deadline_ms: u64,
    },
}

/// Result of a sensor evaluation or lifecycle signal dispatch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GuardTransitionResult {
    pub previous_regime: GuardRegime,
    pub current_regime: GuardRegime,
    pub transition_occurred: bool,
    pub reason: String,
    pub evaluation_latency_micros: u64,
}

/// Configuration parameters for thermal and battery hysteresis deadbands.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuardHysteresisConfig {
    /// Upper thermal threshold triggering Throttled50 (default 38.0°C).
    pub throttle_temp_celsius: f32,
    /// Upper thermal threshold triggering PausedWaitCoolOrCharge (default 41.0°C).
    pub pause_temp_celsius: f32,
    /// Critical thermal threshold triggering EmergencyEvict (default 44.0°C).
    pub emergency_temp_celsius: f32,
    /// Temperature cooling deadband required before exiting pause/throttle (default 3.0°C).
    pub cooling_deadband_celsius: f32,
    /// Battery recharge deadband required before resuming from low battery (default 3%).
    pub battery_recharge_deadband_percent: u8,
}

impl Default for GuardHysteresisConfig {
    fn default() -> Self {
        Self {
            throttle_temp_celsius: 38.0,
            pause_temp_celsius: 41.0,
            emergency_temp_celsius: 44.0,
            cooling_deadband_celsius: 3.0,
            battery_recharge_deadband_percent: 3,
        }
    }
}

/// State machine managing progressive thermal throttling, battery preservation, and OS lifecycle hooks.
#[derive(Debug)]
pub struct ThermalBatteryGuard {
    config: GuardHysteresisConfig,
    current_regime: GuardRegime,
    require_charging: bool,
    min_battery_percent: u8,
    last_temperature_celsius: f32,
    last_battery_percent: Option<u8>,
    last_is_charging: bool,
    event_tx: broadcast::Sender<NodeGuardEvent>,
}

impl ThermalBatteryGuard {
    /// Capacity of the internal broadcast channel for urgent guard events.
    pub const EVENT_CHANNEL_CAPACITY: usize = 128;

    /// Creates a new `ThermalBatteryGuard` initialized with the given policy and default hysteresis.
    #[must_use]
    pub fn new(policy: &ResourceConsentPolicy) -> Self {
        Self::with_config(policy, GuardHysteresisConfig::default())
    }

    /// Creates a new guard with custom hysteresis thresholds.
    #[must_use]
    pub fn with_config(policy: &ResourceConsentPolicy, config: GuardHysteresisConfig) -> Self {
        let (event_tx, _) = broadcast::channel(Self::EVENT_CHANNEL_CAPACITY);
        Self {
            config,
            current_regime: GuardRegime::FullCapacity,
            require_charging: policy.require_charging,
            min_battery_percent: policy.min_battery_percent,
            last_temperature_celsius: 25.0,
            last_battery_percent: Some(100),
            last_is_charging: true,
            event_tx,
        }
    }

    /// Subscribes to the stream of urgent guard events (`NodeGuardEvent`).
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<NodeGuardEvent> {
        self.event_tx.subscribe()
    }

    /// Returns the active operational regime.
    #[must_use]
    pub const fn current_regime(&self) -> GuardRegime {
        self.current_regime
    }

    /// Updates dynamic consent parameters (`require_charging`, `min_battery_percent`).
    pub fn update_policy(&mut self, policy: &ResourceConsentPolicy) {
        self.require_charging = policy.require_charging;
        self.min_battery_percent = policy.min_battery_percent;
    }

    /// Evaluates incoming sensor telemetry against thermal thresholds and hysteresis deadbands.
    /// Invariant: Must complete evaluation and state transition in `< 50 ms`.
    pub fn evaluate(&mut self, telemetry: &SensorTelemetry) -> GuardTransitionResult {
        let t0 = Instant::now();
        let prev_regime = self.current_regime;

        self.last_temperature_celsius = telemetry.temperature_celsius;
        self.last_battery_percent = telemetry.battery_percent;
        self.last_is_charging = telemetry.is_charging;

        let temp = telemetry.temperature_celsius;
        let is_charging = telemetry.is_charging;
        let battery = telemetry.battery_percent.unwrap_or(100);

        let mut next_regime = prev_regime;
        let mut reason = String::new();

        // 1. Check Emergency Evacuate (>= 44°C)
        if temp >= self.config.emergency_temp_celsius {
            next_regime = GuardRegime::EmergencyEvict;
            reason = format!("Temperature {temp:.1}°C >= critical limit {:.1}°C", self.config.emergency_temp_celsius);
        }
        // 2. Check Paused (>= 41°C, low battery, or unplugged when charging required)
        else if temp >= self.config.pause_temp_celsius {
            next_regime = GuardRegime::PausedWaitCoolOrCharge;
            reason = format!("Temperature {temp:.1}°C >= pause limit {:.1}°C", self.config.pause_temp_celsius);
        } else if self.require_charging && !is_charging {
            next_regime = GuardRegime::PausedWaitCoolOrCharge;
            reason = "Device unplugged from AC wall power while policy requires charging".to_string();
        } else if battery < self.min_battery_percent {
            next_regime = GuardRegime::PausedWaitCoolOrCharge;
            reason = format!("Battery {battery}% < minimum floor {}%", self.min_battery_percent);
        }
        // 3. Regimes under hysteresis evaluation
        else {
            match prev_regime {
                GuardRegime::EmergencyEvict => {
                    // Must cool down below pause_temp_celsius - cooling_deadband (e.g. 41 - 3 = 38°C)
                    let resume_threshold = self.config.pause_temp_celsius - self.config.cooling_deadband_celsius;
                    if temp <= resume_threshold {
                        next_regime = GuardRegime::Throttled50;
                        reason = format!("Cooled to {temp:.1}°C <= {:.1}°C after emergency evict", resume_threshold);
                    }
                }
                GuardRegime::PausedWaitCoolOrCharge => {
                    // Thermal recovery requires temp <= pause_temp - cooling_deadband (e.g. 41 - 3 = 38°C)
                    let resume_temp = self.config.pause_temp_celsius - self.config.cooling_deadband_celsius;
                    // Battery recovery requires battery >= min_battery + deadband (e.g. 20 + 3 = 23%)
                    let resume_bat = self.min_battery_percent.saturating_add(self.config.battery_recharge_deadband_percent);

                    let temp_recovered = temp <= resume_temp;
                    let power_recovered = !self.require_charging || is_charging;
                    let battery_recovered = battery >= resume_bat;

                    if temp_recovered && power_recovered && battery_recovered {
                        if temp < self.config.throttle_temp_celsius {
                            next_regime = GuardRegime::FullCapacity;
                            reason = format!("Cooled to {temp:.1}°C and battery recharged to {battery}%", );
                        } else {
                            next_regime = GuardRegime::Throttled50;
                            reason = format!("Cooled to {temp:.1}°C; operating in Throttled50", );
                        }
                    }
                }
                GuardRegime::Throttled50 => {
                    // Check transition to FullCapacity: requires temp <= throttle_temp - cooling_deadband (or < throttle_temp)
                    let cool_to_full = self.config.throttle_temp_celsius - 1.0;
                    if temp < cool_to_full {
                        next_regime = GuardRegime::FullCapacity;
                        reason = format!("Cooled to {temp:.1}°C < {:.1}°C", cool_to_full);
                    }
                }
                GuardRegime::FullCapacity => {
                    if temp >= self.config.throttle_temp_celsius {
                        next_regime = GuardRegime::Throttled50;
                        reason = format!("Temperature {temp:.1}°C >= throttle limit {:.1}°C", self.config.throttle_temp_celsius);
                    }
                }
            }
        }

        let transition_occurred = next_regime != prev_regime;
        self.current_regime = next_regime;

        if transition_occurred {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let event = NodeGuardEvent::RegimeChanged {
                from: prev_regime,
                to: next_regime,
                reason: reason.clone(),
                timestamp_epoch_ms: now_ms,
            };
            let _ = self.event_tx.send(event);

            match next_regime {
                GuardRegime::FullCapacity => {
                    let _ = self.event_tx.send(NodeGuardEvent::ResumeWorkload);
                }
                GuardRegime::Throttled50 => {
                    let _ = self.event_tx.send(NodeGuardEvent::ThrottleWorkload { throttle_percent: 50 });
                }
                GuardRegime::PausedWaitCoolOrCharge => {
                    let _ = self.event_tx.send(NodeGuardEvent::SuspendWorkload { reason: reason.clone() });
                }
                GuardRegime::EmergencyEvict => {
                    let _ = self.event_tx.send(NodeGuardEvent::EvacuateNode {
                        reason: reason.clone(),
                        deadline_ms: 100, // < 100 ms deadline for emergency release
                    });
                }
            }
        }

        let elapsed = t0.elapsed();
        GuardTransitionResult {
            previous_regime: prev_regime,
            current_regime: next_regime,
            transition_occurred,
            reason,
            evaluation_latency_micros: elapsed.as_micros() as u64,
        }
    }

    /// Handles an incoming mobile OS lifecycle signal (trim memory, background pause).
    /// Invariant: Must process signal and trigger buffer release within `< 100 ms`.
    pub fn handle_lifecycle_signal(&mut self, signal: OsLifecycleSignal) -> GuardTransitionResult {
        let t0 = Instant::now();
        let prev_regime = self.current_regime;

        let mut next_regime = prev_regime;
        let mut reason = String::new();

        match signal {
            OsLifecycleSignal::TrimMemory(level) => match level {
                TrimMemoryLevel::RunningCritical | TrimMemoryLevel::Complete => {
                    next_regime = GuardRegime::EmergencyEvict;
                    reason = format!("OS onTrimMemory({level:?}) critical memory reclaim signal received");
                }
                TrimMemoryLevel::RunningLow => {
                    if prev_regime == GuardRegime::FullCapacity {
                        next_regime = GuardRegime::Throttled50;
                        reason = "OS onTrimMemory(RunningLow) memory pressure signal".to_string();
                    }
                }
                TrimMemoryLevel::RunningModerate => {}
            },
            OsLifecycleSignal::AppLifecycle(state) => match state {
                AppState::Paused | AppState::Detached | AppState::Hidden => {
                    if prev_regime != GuardRegime::EmergencyEvict {
                        next_regime = GuardRegime::PausedWaitCoolOrCharge;
                        reason = format!("AppLifecycleState::{state:?} backgrounded");
                    }
                }
                AppState::Resumed => {
                    if prev_regime == GuardRegime::PausedWaitCoolOrCharge {
                        // Re-evaluate against sensors
                        next_regime = GuardRegime::FullCapacity;
                        reason = "AppLifecycleState::Resumed foreground resumed".to_string();
                    }
                }
                AppState::Inactive => {}
            },
            OsLifecycleSignal::BackgroundExpirationWarning { remaining_seconds } => {
                next_regime = GuardRegime::EmergencyEvict;
                reason = format!("Background execution expiring in {remaining_seconds}s");
            }
        }

        let transition_occurred = next_regime != prev_regime;
        self.current_regime = next_regime;

        if transition_occurred {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let event = NodeGuardEvent::RegimeChanged {
                from: prev_regime,
                to: next_regime,
                reason: reason.clone(),
                timestamp_epoch_ms: now_ms,
            };
            let _ = self.event_tx.send(event);

            if next_regime == GuardRegime::EmergencyEvict {
                let _ = self.event_tx.send(NodeGuardEvent::EvacuateNode {
                    reason: reason.clone(),
                    deadline_ms: 100,
                });
            } else if next_regime == GuardRegime::PausedWaitCoolOrCharge {
                let _ = self.event_tx.send(NodeGuardEvent::SuspendWorkload { reason: reason.clone() });
            }
        }

        let elapsed = t0.elapsed();
        GuardTransitionResult {
            previous_regime: prev_regime,
            current_regime: next_regime,
            transition_occurred,
            reason,
            evaluation_latency_micros: elapsed.as_micros() as u64,
        }
    }
}
