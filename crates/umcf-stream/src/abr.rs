//! Delay-Gradient Adaptive Bitrate (ABR) Controller & Congestion Avoidance Engine.
//!
//! Measures one-way queuing delay gradients, detects Wi-Fi/cellular bufferbloat
//! before packet loss occurs, enforces preemptive backoff, and governs dynamic resolution scaling.

use serde::{Deserialize, Serialize};

/// Current state machine status of the adaptive bitrate controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbrState {
    Hold,
    RampUp,
    PreemptiveBackoff,
    ResolutionDownshift,
}

/// Dynamic resolution configuration governed by the ABR controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamResolution {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl StreamResolution {
    #[must_use]
    pub fn r1080p60() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
        }
    }

    #[must_use]
    pub fn r720p60() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 60,
        }
    }

    #[must_use]
    pub fn r1080p30() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
        }
    }
}

/// Delay-Gradient Adaptive Bitrate (ABR) Controller.
#[derive(Debug)]
pub struct DelayGradientAbrController {
    pub current_state: AbrState,
    pub current_bitrate_kbps: u32,
    pub min_bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
    pub current_resolution: StreamResolution,
    pub filtered_delay_gradient_ms: f64,
    prev_send_time_us: u64,
    prev_recv_time_us: u64,
    stable_headroom_duration_ms: u64,
    pub last_state_transition_ms: u64,
}

impl DelayGradientAbrController {
    #[must_use]
    pub fn new(min_bitrate_kbps: u32, max_bitrate_kbps: u32) -> Self {
        Self {
            current_state: AbrState::Hold,
            current_bitrate_kbps: (min_bitrate_kbps + max_bitrate_kbps) / 2,
            min_bitrate_kbps,
            max_bitrate_kbps,
            current_resolution: StreamResolution::r1080p60(),
            filtered_delay_gradient_ms: 0.0,
            prev_send_time_us: 0,
            prev_recv_time_us: 0,
            stable_headroom_duration_ms: 0,
            last_state_transition_ms: 0,
        }
    }

    /// Feeds arrival of a packet sample into the delay gradient filter and updates ABR state.
    pub fn on_packet_feedback(
        &mut self,
        send_time_us: u64,
        recv_time_us: u64,
        loss_fraction: f32,
        now_ms: u64,
    ) -> (u32, StreamResolution, AbrState) {
        if self.prev_send_time_us > 0
            && self.prev_recv_time_us > 0
            && recv_time_us >= self.prev_recv_time_us
        {
            let send_delta = (send_time_us as f64 - self.prev_send_time_us as f64) / 1000.0;
            let recv_delta = (recv_time_us as f64 - self.prev_recv_time_us as f64) / 1000.0;
            let raw_gradient = recv_delta - send_delta;

            // EWMA smoothing of delay gradient (alpha = 0.5)
            self.filtered_delay_gradient_ms =
                0.50 * raw_gradient + 0.50 * self.filtered_delay_gradient_ms;
        }

        self.prev_send_time_us = send_time_us;
        self.prev_recv_time_us = recv_time_us;

        // ABR State Machine Transitions
        let grad = self.filtered_delay_gradient_ms;

        if grad > 3.0 || loss_fraction >= 0.02 {
            // Bufferbloat detected or packet loss rising -> Immediate Preemptive Backoff
            self.current_state = AbrState::PreemptiveBackoff;
            self.stable_headroom_duration_ms = 0;
            self.last_state_transition_ms = now_ms;

            // Reduce bitrate by 20% immediately (within <= 100 ms)
            let reduced = (self.current_bitrate_kbps as f64 * 0.80).round() as u32;
            self.current_bitrate_kbps = reduced.max(self.min_bitrate_kbps);

            // If bitrate drops below 3.5 Mbps for 1080p60, step down resolution
            if self.current_bitrate_kbps < 3500 && self.current_resolution.height == 1080 {
                self.current_resolution = StreamResolution::r720p60();
                self.current_state = AbrState::ResolutionDownshift;
            }
        } else if grad <= 2.0 && loss_fraction < 0.005 {
            // Clean link with queuing delay gradient <= 2ms -> RampUp
            self.stable_headroom_duration_ms += 100;

            if matches!(
                self.current_state,
                AbrState::PreemptiveBackoff | AbrState::ResolutionDownshift
            ) {
                self.current_state = AbrState::Hold;
            } else if self.current_state == AbrState::Hold && self.stable_headroom_duration_ms > 500
            {
                self.current_state = AbrState::RampUp;
            }

            if self.current_state == AbrState::RampUp {
                // Smooth ramp up: +8% per second (+0.8% per 100ms sample)
                let increment = ((self.current_bitrate_kbps as f64) * 0.008)
                    .max(50.0)
                    .round() as u32;
                self.current_bitrate_kbps =
                    (self.current_bitrate_kbps + increment).min(self.max_bitrate_kbps);

                // Step resolution back up after 5.0s (5000 ms) of stable headroom
                if self.stable_headroom_duration_ms >= 5000 && self.current_resolution.height == 720
                {
                    self.current_resolution = StreamResolution::r1080p60();
                }
            }
        } else {
            // Stable hold state
            self.current_state = AbrState::Hold;
        }

        (
            self.current_bitrate_kbps,
            self.current_resolution,
            self.current_state,
        )
    }
}
