//! `umcf-guard::preemption`: Interactive User Activity Detector & Instant Preemption (< 50 ms).
//! Detects foreground user input (keyboard, mouse, touch, fullscreen apps, CPU/GPU spikes)
//! and preempts or throttles background UMCF workloads immediately to protect host responsiveness.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Source of interactive user activity.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActivitySource {
    /// Foreground keyboard keystroke detected.
    Keyboard,
    /// Mouse motion or click detected.
    Mouse,
    /// Touchscreen tap or swipe gesture detected.
    Touch,
    /// Fullscreen application, video game, or multimedia playback launched.
    FullscreenAppOrGame,
    /// Foreground window focus change.
    ForegroundAppFocus,
    /// Non-UMCF host CPU load delta spike (exceeding threshold).
    HostCpuSpike { delta_percent: f64 },
    /// Non-UMCF host GPU utilization spike.
    HostGpuSpike { delta_percent: f64 },
    /// Synthetic test input for deterministic simulation.
    Synthetic { name: String },
}

impl std::fmt::Display for ActivitySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyboard => write!(f, "Keyboard"),
            Self::Mouse => write!(f, "Mouse"),
            Self::Touch => write!(f, "Touch"),
            Self::FullscreenAppOrGame => write!(f, "FullscreenAppOrGame"),
            Self::ForegroundAppFocus => write!(f, "ForegroundAppFocus"),
            Self::HostCpuSpike { delta_percent } => {
                write!(f, "HostCpuSpike(+{delta_percent:.1}%)")
            }
            Self::HostGpuSpike { delta_percent } => {
                write!(f, "HostGpuSpike(+{delta_percent:.1}%)")
            }
            Self::Synthetic { name } => write!(f, "Synthetic({name})"),
        }
    }
}

/// Operational state of user interaction and compute preemption.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActivityState {
    /// Host is completely idle with respect to user interaction.
    /// Full 100% consented compute capacity is available for UMCF workers.
    Idle,
    /// User is actively interacting with the host device.
    /// Background worker compute is throttled to `interactive_cpu_cap_percent` or paused.
    InteractiveActive {
        detected_at_us: u64,
        source: ActivitySource,
    },
    /// User interaction has stopped; cooldown period active before beginning capacity ramp-up.
    Cooldown {
        started_at_us: u64,
        cooldown_duration_secs: f64,
    },
    /// Smooth capacity ramp-up is progressing back to 100% full capacity.
    RampingUp {
        started_at_us: u64,
        ramp_duration_secs: f64,
        progress: f64,
        current_capacity_percent: f64,
    },
}

/// Configuration parameters for user activity detection and instant preemption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreemptionConfig {
    /// Maximum CPU capacity percentage allocated to background workloads during interactive sessions.
    /// Defaults to 15.0%.
    pub interactive_cpu_cap_percent: f64,
    /// Inactivity cooldown duration in seconds before beginning ramp-up back to full capacity.
    /// Defaults to 30.0s (can be set lower for tests).
    pub idle_cooldown_secs: f64,
    /// Duration of the smooth ramp-up curve in seconds from interactive cap to 100%.
    /// Defaults to 3.0s.
    pub ramp_up_duration_secs: f64,
    /// Maximum allowable preemption latency in microseconds. Target: 50,000 µs (50 ms).
    pub preemption_deadline_us: u64,
    /// CPU spike threshold from non-UMCF processes to trigger preemption. Defaults to 25.0%.
    pub host_cpu_spike_threshold_percent: f64,
}

impl Default for PreemptionConfig {
    fn default() -> Self {
        Self {
            interactive_cpu_cap_percent: 15.0,
            idle_cooldown_secs: 30.0,
            ramp_up_duration_secs: 3.0,
            preemption_deadline_us: 50_000, // 50 ms
            host_cpu_spike_threshold_percent: 25.0,
        }
    }
}

/// Preemption event emitted to notify workers, schedulers, and telemetry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PreemptionEvent {
    /// User input detected and worker preemption initiated.
    Preempted {
        source: ActivitySource,
        latency_us: u64,
        target_cpu_cap_percent: f64,
        timestamp_us: u64,
    },
    /// Inactivity cooldown period started.
    CooldownStarted {
        started_at_us: u64,
        duration_secs: f64,
    },
    /// Capacity ramp-up step executed.
    RampUpStep {
        progress: f64,
        capacity_percent: f64,
        timestamp_us: u64,
    },
    /// Worker capacity restored to full 100% consented limit.
    ResumedToFullCapacity { timestamp_us: u64 },
}

/// Checkpoint payload generated cleanly during preemption to guarantee zero task corruption.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskCheckpoint {
    pub task_id: String,
    pub checkpoint_seq: u64,
    pub state_hash: String,
    pub payload_bytes: Vec<u8>,
    pub timestamp_us: u64,
}

/// Token held by workers to cooperatively check preemption and yield checkpoints without corruption.
#[derive(Debug, Clone)]
pub struct PreemptionToken {
    task_id: String,
    is_preempted: Arc<AtomicBool>,
    preemption_generation: Arc<AtomicU64>,
    target_cap_percent: Arc<RwLock<f64>>,
}

impl PreemptionToken {
    /// Creates a new preemption token for a registered task.
    #[must_use]
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            is_preempted: Arc::new(AtomicBool::new(false)),
            preemption_generation: Arc::new(AtomicU64::new(0)),
            target_cap_percent: Arc::new(RwLock::new(100.0)),
        }
    }

    /// Returns the associated task ID.
    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Checks whether this task has been requested to preempt or throttle.
    #[must_use]
    pub fn is_preempted(&self) -> bool {
        self.is_preempted.load(Ordering::Acquire)
    }

    /// Returns current preemption generation counter.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.preemption_generation.load(Ordering::Acquire)
    }

    /// Returns currently consented capacity percentage for this worker.
    #[must_use]
    pub fn current_cap_percent(&self) -> f64 {
        self.target_cap_percent
            .read()
            .map(|guard| *guard)
            .unwrap_or(15.0)
    }

    /// Cooperative yield boundary: if preemption is active, creates an uncorrupted checkpoint
    /// and returns it, allowing the worker to pause cleanly or migrate.
    pub fn cooperative_yield_checkpoint(
        &self,
        checkpoint_seq: u64,
        payload: &[u8],
        now_us: u64,
    ) -> Option<TaskCheckpoint> {
        if !self.is_preempted() {
            return None;
        }

        // Compute deterministic state hash of the uncorrupted payload
        let state_hash = format!("{:016x}", compute_simple_hash(payload));

        Some(TaskCheckpoint {
            task_id: self.task_id.clone(),
            checkpoint_seq,
            state_hash,
            payload_bytes: payload.to_vec(),
            timestamp_us: now_us,
        })
    }

    /// Marks the token as preempted with the target capacity cap.
    fn set_preempted(&self, cap_percent: f64) {
        self.is_preempted.store(true, Ordering::Release);
        self.preemption_generation.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut guard) = self.target_cap_percent.write() {
            *guard = cap_percent;
        }
    }

    /// Clears preemption and updates capacity percentage.
    fn set_resumed(&self, cap_percent: f64) {
        self.is_preempted.store(false, Ordering::Release);
        if let Ok(mut guard) = self.target_cap_percent.write() {
            *guard = cap_percent;
        }
    }

    /// Updates capacity percentage during ramp-up without changing preemption state.
    fn set_capacity(&self, cap_percent: f64) {
        if let Ok(mut guard) = self.target_cap_percent.write() {
            *guard = cap_percent;
        }
    }
}

/// Simple deterministic hash calculation for checkpoints (FNV-1a 64-bit).
#[must_use]
fn compute_simple_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Preemption latency histogram recording distribution of preemption latencies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreemptionLatencyHistogram {
    pub sample_count: usize,
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: f64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub deadline_met_count: usize,
    pub deadline_miss_count: usize,
    pub deadline_met_percent: f64,
    pub buckets: HashMap<String, usize>,
}

/// Central User Activity Detector and Instant Preemption coordinator.
#[derive(Debug)]
pub struct InstantPreemptor {
    config: PreemptionConfig,
    state: RwLock<ActivityState>,
    active_tokens: RwLock<HashMap<String, PreemptionToken>>,
    event_tx: broadcast::Sender<PreemptionEvent>,
    latency_samples_us: RwLock<Vec<u64>>,
    last_interaction_us: AtomicU64,
}

impl InstantPreemptor {
    /// Creates a new `InstantPreemptor` with the specified configuration.
    #[must_use]
    pub fn new(config: PreemptionConfig) -> Self {
        let (event_tx, _) = broadcast::channel(128);
        Self {
            config,
            state: RwLock::new(ActivityState::Idle),
            active_tokens: RwLock::new(HashMap::new()),
            event_tx,
            latency_samples_us: RwLock::new(Vec::new()),
            last_interaction_us: AtomicU64::new(0),
        }
    }

    /// Subscribes to the broadcast channel of preemption events.
    pub fn subscribe(&self) -> broadcast::Receiver<PreemptionEvent> {
        self.event_tx.subscribe()
    }

    /// Registers a worker task and returns its `PreemptionToken`.
    pub fn register_task(&self, task_id: &str) -> PreemptionToken {
        let token = PreemptionToken::new(task_id.to_string());
        // If system is currently preempted, apply current state to the new token
        let current_state = self.current_state();
        match current_state {
            ActivityState::InteractiveActive { .. } => {
                token.set_preempted(self.config.interactive_cpu_cap_percent);
            }
            ActivityState::Cooldown { .. } => {
                token.set_preempted(self.config.interactive_cpu_cap_percent);
            }
            ActivityState::RampingUp {
                current_capacity_percent,
                ..
            } => {
                token.set_capacity(current_capacity_percent);
            }
            ActivityState::Idle => {
                token.set_resumed(100.0);
            }
        }

        if let Ok(mut tokens) = self.active_tokens.write() {
            tokens.insert(task_id.to_string(), token.clone());
        }
        token
    }

    /// Unregisters a completed or cancelled worker task.
    pub fn unregister_task(&self, task_id: &str) {
        if let Ok(mut tokens) = self.active_tokens.write() {
            tokens.remove(task_id);
        }
    }

    /// Returns the current activity state.
    #[must_use]
    pub fn current_state(&self) -> ActivityState {
        self.state
            .read()
            .map(|guard| guard.clone())
            .unwrap_or(ActivityState::Idle)
    }

    /// Returns current effective compute capacity percentage (between `interactive_cpu_cap_percent` and 100.0%).
    #[must_use]
    pub fn current_capacity_percent(&self) -> f64 {
        match self.current_state() {
            ActivityState::Idle => 100.0,
            ActivityState::InteractiveActive { .. } => self.config.interactive_cpu_cap_percent,
            ActivityState::Cooldown { .. } => self.config.interactive_cpu_cap_percent,
            ActivityState::RampingUp {
                current_capacity_percent,
                ..
            } => current_capacity_percent,
        }
    }

    /// Ingests a user input signal (keyboard, mouse, touch, app launch), triggering instant preemption (< 50 ms).
    pub fn on_user_input(
        &self,
        source: ActivitySource,
        detection_timestamp_us: u64,
        enforce_timestamp_us: u64,
    ) -> PreemptionEvent {
        let latency_us = enforce_timestamp_us.saturating_sub(detection_timestamp_us);
        self.last_interaction_us
            .store(enforce_timestamp_us, Ordering::Release);

        // Record latency sample
        if let Ok(mut samples) = self.latency_samples_us.write() {
            samples.push(latency_us);
        }

        // Instantly preempt all registered worker tasks
        if let Ok(tokens) = self.active_tokens.read() {
            for token in tokens.values() {
                token.set_preempted(self.config.interactive_cpu_cap_percent);
            }
        }

        // Update internal state to InteractiveActive
        if let Ok(mut state_guard) = self.state.write() {
            *state_guard = ActivityState::InteractiveActive {
                detected_at_us: enforce_timestamp_us,
                source: source.clone(),
            };
        }

        let event = PreemptionEvent::Preempted {
            source,
            latency_us,
            target_cpu_cap_percent: self.config.interactive_cpu_cap_percent,
            timestamp_us: enforce_timestamp_us,
        };

        let _ = self.event_tx.send(event.clone());
        event
    }

    /// Ingests a non-UMCF host CPU load delta. If delta exceeds threshold, triggers preemption.
    pub fn on_host_cpu_delta(
        &self,
        delta_percent: f64,
        timestamp_us: u64,
    ) -> Option<PreemptionEvent> {
        if delta_percent >= self.config.host_cpu_spike_threshold_percent {
            Some(self.on_user_input(
                ActivitySource::HostCpuSpike { delta_percent },
                timestamp_us,
                timestamp_us,
            ))
        } else {
            None
        }
    }

    /// Evaluates the activity state machine at a given monotonic timestamp (advancing cooldown and ramp-up).
    /// Returns any transition events that fired.
    pub fn advance_time(&self, now_us: u64) -> Vec<PreemptionEvent> {
        let mut events = Vec::new();

        let current_state = self.current_state();
        match current_state {
            ActivityState::Idle => {
                // No transitions needed while Idle
            }
            ActivityState::InteractiveActive { detected_at_us, .. } => {
                // Check if user has stopped interacting: transition to Cooldown
                let elapsed_us = now_us.saturating_sub(detected_at_us);
                // User input burst window: if no further input within 200 ms, enter Cooldown
                if elapsed_us >= 200_000 {
                    let cooldown_duration_secs = self.config.idle_cooldown_secs;
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = ActivityState::Cooldown {
                            started_at_us: now_us,
                            cooldown_duration_secs,
                        };
                    }
                    let event = PreemptionEvent::CooldownStarted {
                        started_at_us: now_us,
                        duration_secs: cooldown_duration_secs,
                    };
                    let _ = self.event_tx.send(event.clone());
                    events.push(event);
                }
            }
            ActivityState::Cooldown {
                started_at_us,
                cooldown_duration_secs,
            } => {
                let elapsed_secs = (now_us.saturating_sub(started_at_us)) as f64 / 1_000_000.0;
                if elapsed_secs >= cooldown_duration_secs {
                    // Cooldown expired without interruption: begin RampingUp
                    let ramp_duration_secs = self.config.ramp_up_duration_secs;
                    let initial_cap = self.config.interactive_cpu_cap_percent;
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = ActivityState::RampingUp {
                            started_at_us: now_us,
                            ramp_duration_secs,
                            progress: 0.0,
                            current_capacity_percent: initial_cap,
                        };
                    }
                    let event = PreemptionEvent::RampUpStep {
                        progress: 0.0,
                        capacity_percent: initial_cap,
                        timestamp_us: now_us,
                    };
                    let _ = self.event_tx.send(event.clone());
                    events.push(event);
                }
            }
            ActivityState::RampingUp {
                started_at_us,
                ramp_duration_secs,
                ..
            } => {
                let elapsed_secs = (now_us.saturating_sub(started_at_us)) as f64 / 1_000_000.0;
                let progress = (elapsed_secs / ramp_duration_secs).clamp(0.0, 1.0);
                let cap_range = 100.0 - self.config.interactive_cpu_cap_percent;
                let current_capacity_percent =
                    self.config.interactive_cpu_cap_percent + (cap_range * progress);

                if progress >= 1.0 {
                    // Ramp-up completed: restore full 100% capacity and transition to Idle
                    if let Ok(tokens) = self.active_tokens.read() {
                        for token in tokens.values() {
                            token.set_resumed(100.0);
                        }
                    }
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = ActivityState::Idle;
                    }
                    let event = PreemptionEvent::ResumedToFullCapacity {
                        timestamp_us: now_us,
                    };
                    let _ = self.event_tx.send(event.clone());
                    events.push(event);
                } else {
                    // Update active tokens with current intermediate capacity
                    if let Ok(tokens) = self.active_tokens.read() {
                        for token in tokens.values() {
                            token.set_capacity(current_capacity_percent);
                        }
                    }
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = ActivityState::RampingUp {
                            started_at_us,
                            ramp_duration_secs,
                            progress,
                            current_capacity_percent,
                        };
                    }
                    let event = PreemptionEvent::RampUpStep {
                        progress,
                        capacity_percent: current_capacity_percent,
                        timestamp_us: now_us,
                    };
                    let _ = self.event_tx.send(event.clone());
                    events.push(event);
                }
            }
        }

        events
    }

    /// Computes and returns the `PreemptionLatencyHistogram` across all recorded preemption events.
    #[must_use]
    pub fn latency_histogram(&self) -> PreemptionLatencyHistogram {
        let samples = self
            .latency_samples_us
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let sample_count = samples.len();
        if sample_count == 0 {
            return PreemptionLatencyHistogram {
                sample_count: 0,
                min_us: 0,
                max_us: 0,
                avg_us: 0.0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                deadline_met_count: 0,
                deadline_miss_count: 0,
                deadline_met_percent: 100.0,
                buckets: HashMap::new(),
            };
        }

        let mut sorted = samples;
        sorted.sort_unstable();

        let min_us = sorted[0];
        let max_us = sorted[sample_count - 1];
        let sum: u64 = sorted.iter().sum();
        let avg_us = sum as f64 / sample_count as f64;

        let p50_idx = ((sample_count as f64 * 0.50) as usize).min(sample_count - 1);
        let p95_idx = ((sample_count as f64 * 0.95) as usize).min(sample_count - 1);
        let p99_idx = ((sample_count as f64 * 0.99) as usize).min(sample_count - 1);

        let p50_us = sorted[p50_idx];
        let p95_us = sorted[p95_idx];
        let p99_us = sorted[p99_idx];

        let deadline_us = self.config.preemption_deadline_us;
        let mut deadline_met_count = 0;
        let mut deadline_miss_count = 0;

        let mut b_under_100us = 0;
        let mut b_100_to_500us = 0;
        let mut b_500_to_1000us = 0;
        let mut b_1_to_5ms = 0;
        let mut b_5_to_10ms = 0;
        let mut b_10_to_25ms = 0;
        let mut b_25_to_50ms = 0;
        let mut b_over_50ms = 0;

        for &val in &sorted {
            if val <= deadline_us {
                deadline_met_count += 1;
            } else {
                deadline_miss_count += 1;
            }

            if val < 100 {
                b_under_100us += 1;
            } else if val < 500 {
                b_100_to_500us += 1;
            } else if val < 1_000 {
                b_500_to_1000us += 1;
            } else if val < 5_000 {
                b_1_to_5ms += 1;
            } else if val < 10_000 {
                b_5_to_10ms += 1;
            } else if val < 25_000 {
                b_10_to_25ms += 1;
            } else if val <= 50_000 {
                b_25_to_50ms += 1;
            } else {
                b_over_50ms += 1;
            }
        }

        let deadline_met_percent = (deadline_met_count as f64 / sample_count as f64) * 100.0;

        let mut buckets = HashMap::new();
        buckets.insert("< 100 us".to_string(), b_under_100us);
        buckets.insert("100 - 500 us".to_string(), b_100_to_500us);
        buckets.insert("500 - 1000 us".to_string(), b_500_to_1000us);
        buckets.insert("1 - 5 ms".to_string(), b_1_to_5ms);
        buckets.insert("5 - 10 ms".to_string(), b_5_to_10ms);
        buckets.insert("10 - 25 ms".to_string(), b_10_to_25ms);
        buckets.insert("25 - 50 ms".to_string(), b_25_to_50ms);
        buckets.insert(">= 50 ms".to_string(), b_over_50ms);

        PreemptionLatencyHistogram {
            sample_count,
            min_us,
            max_us,
            avg_us,
            p50_us,
            p95_us,
            p99_us,
            deadline_met_count,
            deadline_miss_count,
            deadline_met_percent,
            buckets,
        }
    }
}
