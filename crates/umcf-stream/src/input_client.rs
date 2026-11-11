//! Mobile & Desktop Remote Input Return Client (UMCF S6.P3.SP3).
//!
//! Manages monotonic sequence generation, high-frequency event coalescing (500 Hz mouse moves),
//! touch-to-mouse mode mapping, and incoming rumble/cursor feedback processing.

use std::collections::VecDeque;

use crate::error::StreamError;
use crate::input_protocol::{
    CursorShapeType, HostFeedbackEvent, InputEvent, InputEventPacket,
};

/// Active controller rumble motor intensities and duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveControllerRumble {
    pub low_freq_motor: u16,
    pub high_freq_motor: u16,
    pub left_trigger_motor: u16,
    pub right_trigger_motor: u16,
    pub expires_at_us: u64,
}

/// Input Return Client running on the mobile phone, tablet, or web browser.
pub struct InputReturnClient {
    next_seq_no: u64,
    pending_queue: VecDeque<InputEventPacket>,
    pub current_cursor: CursorShapeType,
    pub active_rumbles: [ActiveControllerRumble; 4],
}

impl Default for InputReturnClient {
    fn default() -> Self {
        Self::new()
    }
}

impl InputReturnClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_seq_no: 1,
            pending_queue: VecDeque::with_capacity(64),
            current_cursor: CursorShapeType::DefaultArrow,
            active_rumbles: [ActiveControllerRumble::default(); 4],
        }
    }

    /// Enqueues an input event, assigning monotonic sequence number and timestamp.
    pub fn enqueue_event(&mut self, event: InputEvent, now_us: u64) {
        let seq = self.next_seq_no;
        self.next_seq_no += 1;

        let packet = InputEventPacket {
            seq_no: seq,
            client_timestamp_us: now_us,
            event,
        };
        self.pending_queue.push_back(packet);
    }

    /// Drains the pending queue, applying high-frequency coalescing for relative mouse movements
    /// and discarding stale absolute mouse positions.
    pub fn drain_coalesced_packets(&mut self) -> Vec<InputEventPacket> {
        if self.pending_queue.is_empty() {
            return Vec::new();
        }

        let mut output = Vec::with_capacity(self.pending_queue.len());
        let mut pending_rel_dx: i32 = 0;
        let mut pending_rel_dy: i32 = 0;
        let mut pending_subpixel_dx: i32 = 0;
        let mut pending_subpixel_dy: i32 = 0;
        let mut last_rel_seq = 0;
        let mut last_rel_ts = 0;

        let mut last_abs_packet: Option<InputEventPacket> = None;

        while let Some(packet) = self.pending_queue.pop_front() {
            match packet.event {
                InputEvent::MouseMoveRelative {
                    dx,
                    dy,
                    subpixel_dx,
                    subpixel_dy,
                } => {
                    pending_rel_dx += i32::from(dx);
                    pending_rel_dy += i32::from(dy);
                    pending_subpixel_dx += i32::from(subpixel_dx);
                    pending_subpixel_dy += i32::from(subpixel_dy);
                    last_rel_seq = packet.seq_no;
                    last_rel_ts = packet.client_timestamp_us;
                }
                InputEvent::MouseMoveAbsolute { .. } => {
                    // Update latest absolute position, discarding previous stale ones
                    last_abs_packet = Some(packet);
                }
                other_event => {
                    // Flush any pending relative mouse delta before other discrete events
                    if pending_rel_dx != 0 || pending_rel_dy != 0 {
                        output.push(InputEventPacket {
                            seq_no: last_rel_seq,
                            client_timestamp_us: last_rel_ts,
                            event: InputEvent::MouseMoveRelative {
                                dx: pending_rel_dx.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                                dy: pending_rel_dy.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                                subpixel_dx: pending_subpixel_dx.clamp(-128, 127) as i8,
                                subpixel_dy: pending_subpixel_dy.clamp(-128, 127) as i8,
                            },
                        });
                        pending_rel_dx = 0;
                        pending_rel_dy = 0;
                        pending_subpixel_dx = 0;
                        pending_subpixel_dy = 0;
                    }
                    if let Some(abs_pkt) = last_abs_packet.take() {
                        output.push(abs_pkt);
                    }
                    output.push(InputEventPacket {
                        seq_no: packet.seq_no,
                        client_timestamp_us: packet.client_timestamp_us,
                        event: other_event,
                    });
                }
            }
        }

        // Flush remaining relative mouse moves
        if pending_rel_dx != 0 || pending_rel_dy != 0 {
            output.push(InputEventPacket {
                seq_no: last_rel_seq,
                client_timestamp_us: last_rel_ts,
                event: InputEvent::MouseMoveRelative {
                    dx: pending_rel_dx.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                    dy: pending_rel_dy.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                    subpixel_dx: pending_subpixel_dx.clamp(-128, 127) as i8,
                    subpixel_dy: pending_subpixel_dy.clamp(-128, 127) as i8,
                },
            });
        }

        // Flush remaining absolute mouse move
        if let Some(abs_pkt) = last_abs_packet {
            output.push(abs_pkt);
        }

        output
    }

    /// Handles an incoming host feedback event (e.g. rumble vibration or cursor shape update).
    pub fn handle_host_feedback(
        &mut self,
        feedback: &HostFeedbackEvent,
        now_us: u64,
    ) -> Result<(), StreamError> {
        match feedback {
            HostFeedbackEvent::GamepadRumble {
                controller_idx,
                low_freq_motor,
                high_freq_motor,
                left_trigger_motor,
                right_trigger_motor,
                duration_ms,
            } => {
                let idx = (*controller_idx as usize).min(3);
                self.active_rumbles[idx] = ActiveControllerRumble {
                    low_freq_motor: *low_freq_motor,
                    high_freq_motor: *high_freq_motor,
                    left_trigger_motor: *left_trigger_motor,
                    right_trigger_motor: *right_trigger_motor,
                    expires_at_us: now_us + (u64::from(*duration_ms) * 1000),
                };
            }
            HostFeedbackEvent::CursorShapeUpdate { shape, .. } => {
                self.current_cursor = *shape;
            }
            HostFeedbackEvent::ClipboardSync { .. } => {
                // Clipboard sync can be handled by client platform clipboard manager
            }
        }
        Ok(())
    }

    /// Checks if a controller motor is actively rumbling at the given timestamp.
    #[must_use]
    pub fn is_rumble_active(&self, controller_idx: u8, now_us: u64) -> bool {
        let idx = (controller_idx as usize).min(3);
        let rumble = &self.active_rumbles[idx];
        rumble.expires_at_us > now_us
            && (rumble.low_freq_motor > 0
                || rumble.high_freq_motor > 0
                || rumble.left_trigger_motor > 0
                || rumble.right_trigger_motor > 0)
    }
}
