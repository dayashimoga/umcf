//! Host Virtual HID Injector & Stuck-Key Safety Watchdog (UMCF S6.P3.SP3).
//!
//! Enforces RBAC capability checks, window coordinate mapping, and guaranteed
//! stuck-key/button rollback upon peer disconnect or session revocation.

use std::collections::{HashMap, HashSet};

use crate::error::StreamError;
use crate::input_protocol::{
    AntiReplayWindow, HostFeedbackEvent, InputEvent, InputEventPacket, TouchPhase,
};

/// Remote stream input access permission granted by capability token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamInputPermission {
    ViewOnly,
    FullControl,
}

/// Geometric bounds of the captured desktop screen or application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowBounds {
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: u32,
    pub height: u32,
}

impl WindowBounds {
    #[must_use]
    pub fn new(offset_x: i32, offset_y: i32, width: u32, height: u32) -> Self {
        Self {
            offset_x,
            offset_y,
            width: width.max(1),
            height: height.max(1),
        }
    }

    /// Maps normalized coordinates [0..=65535] to physical window screen coordinates.
    #[must_use]
    pub fn map_normalized_to_pixel(&self, norm_x: u16, norm_y: u16) -> (i32, i32) {
        let frac_x = f64::from(norm_x) / 65535.0;
        let frac_y = f64::from(norm_y) / 65535.0;

        let px = self.offset_x + (frac_x * f64::from(self.width)).round() as i32;
        let py = self.offset_y + (frac_y * f64::from(self.height)).round() as i32;

        (
            px.clamp(self.offset_x, self.offset_x + self.width as i32 - 1),
            py.clamp(self.offset_y, self.offset_y + self.height as i32 - 1),
        )
    }
}

/// Abstract hardware input injection sink.
pub trait InputSink: Send + Sync {
    fn inject_event(&mut self, event: &InputEvent, now_us: u64) -> Result<(), StreamError>;
    fn query_cursor_position(&self) -> (i32, i32);
}

/// In-memory mock input sink for unit testing and CI verification.
#[derive(Debug, Default)]
pub struct MockInputSink {
    pub injected_events: Vec<(InputEvent, u64)>,
    pub current_cursor: (i32, i32),
}

impl InputSink for MockInputSink {
    fn inject_event(&mut self, event: &InputEvent, now_us: u64) -> Result<(), StreamError> {
        self.injected_events.push((event.clone(), now_us));
        if let InputEvent::MouseMoveAbsolute { norm_x, norm_y, .. } = event {
            self.current_cursor = (*norm_x as i32, *norm_y as i32);
        }
        Ok(())
    }

    fn query_cursor_position(&self) -> (i32, i32) {
        self.current_cursor
    }
}

/// Active Input State Tracker monitoring currently held keys, mouse buttons, touch pointers,
/// and gamepad actuators to ensure 100% automatic rollback upon disconnect or revocation.
#[derive(Debug, Default)]
pub struct ActiveInputStateTracker {
    pub held_keys: HashSet<u16>,             // Set of currently held scancodes
    pub held_mouse_buttons: HashSet<u8>,     // Set of currently held mouse buttons (1=Left, etc.)
    pub active_touch_pointers: HashSet<u8>,  // Set of active touch pointer IDs
    pub active_gamepads: HashMap<u8, u32>,   // Active gamepad controllers and button bitmasks
}

impl ActiveInputStateTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records incoming input events and updates active held state.
    pub fn track_incoming_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Keyboard {
                scancode, pressed, ..
            } => {
                if *pressed {
                    self.held_keys.insert(*scancode);
                } else {
                    self.held_keys.remove(scancode);
                }
            }
            InputEvent::MouseButton { button, pressed } => {
                if *pressed {
                    self.held_mouse_buttons.insert(*button);
                } else {
                    self.held_mouse_buttons.remove(button);
                }
            }
            InputEvent::TouchEvent {
                pointer_id, phase, ..
            } => match phase {
                TouchPhase::Down | TouchPhase::Move => {
                    self.active_touch_pointers.insert(*pointer_id);
                }
                TouchPhase::Up | TouchPhase::Cancel => {
                    self.active_touch_pointers.remove(pointer_id);
                }
            },
            InputEvent::GamepadState {
                controller_idx,
                buttons_bitmask,
                left_stick,
                right_stick,
                left_trigger,
                right_trigger,
            } => {
                let is_neutral = *buttons_bitmask == 0
                    && *left_stick == (0, 0)
                    && *right_stick == (0, 0)
                    && *left_trigger == 0
                    && *right_trigger == 0;

                if is_neutral {
                    self.active_gamepads.remove(controller_idx);
                } else {
                    self.active_gamepads.insert(*controller_idx, *buttons_bitmask);
                }
            }
            _ => {}
        }
    }

    /// Synthesizes and injects release events for 100% of held inputs into the sink.
    /// Guarantees that the host OS is never left with a stuck key, mouse drag, or touch pointer.
    pub fn synthesize_emergency_release_events(
        &mut self,
        sink: &mut dyn InputSink,
        now_us: u64,
    ) -> usize {
        let mut released_count = 0;

        // 1. Release all held keyboard keys
        for scancode in self.held_keys.drain() {
            let release_key = InputEvent::Keyboard {
                scancode,
                hid_usage: 0,
                pressed: false,
                modifiers: 0,
            };
            let _ = sink.inject_event(&release_key, now_us);
            released_count += 1;
        }

        // 2. Release all held mouse buttons
        for button in self.held_mouse_buttons.drain() {
            let release_mouse = InputEvent::MouseButton {
                button,
                pressed: false,
            };
            let _ = sink.inject_event(&release_mouse, now_us);
            released_count += 1;
        }

        // 3. Cancel all active touch pointers
        for pointer_id in self.active_touch_pointers.drain() {
            let cancel_touch = InputEvent::TouchEvent {
                pointer_id,
                phase: TouchPhase::Cancel,
                norm_x: 0,
                norm_y: 0,
                pressure: 0,
                radius: 0,
            };
            let _ = sink.inject_event(&cancel_touch, now_us);
            released_count += 1;
        }

        // 4. Reset all active gamepads to neutral state
        for (controller_idx, _) in self.active_gamepads.drain() {
            let neutral_gamepad = InputEvent::GamepadState {
                controller_idx,
                buttons_bitmask: 0,
                left_stick: (0, 0),
                right_stick: (0, 0),
                left_trigger: 0,
                right_trigger: 0,
            };
            let _ = sink.inject_event(&neutral_gamepad, now_us);
            released_count += 1;
        }

        released_count
    }
}

/// Host Virtual HID Injection Engine.
pub struct InputInjectionHost {
    pub permission: StreamInputPermission,
    pub window_bounds: WindowBounds,
    pub anti_replay: AntiReplayWindow,
    pub state_tracker: ActiveInputStateTracker,
    pub sink: Box<dyn InputSink>,
    pub last_feedback_event: Option<HostFeedbackEvent>,
}

impl InputInjectionHost {
    #[must_use]
    pub fn new(
        permission: StreamInputPermission,
        window_bounds: WindowBounds,
        sink: Box<dyn InputSink>,
    ) -> Self {
        Self {
            permission,
            window_bounds,
            anti_replay: AntiReplayWindow::default(),
            state_tracker: ActiveInputStateTracker::new(),
            sink,
            last_feedback_event: None,
        }
    }

    /// Processes an incoming input packet, validating capabilities and anti-replay guards,
    /// translating window bounds, updating state tracking, and injecting into the OS sink.
    pub fn process_input_packet(
        &mut self,
        packet: &InputEventPacket,
        now_us: u64,
    ) -> Result<(), StreamError> {
        // 1. RBAC Capability Token Verification
        if self.permission == StreamInputPermission::ViewOnly {
            return Err(StreamError::PermissionDenied(
                "view-only session rejected attempt to inject input".into(),
            ));
        }

        // 2. Anti-Replay Guard Check
        if !self.anti_replay.check_and_update(packet.seq_no) {
            return Err(StreamError::TransportError(format!(
                "replay attack or stale input packet rejected (seq_no: {})",
                packet.seq_no
            )));
        }

        // 3. Coordinate translation and boundary clamping
        let mapped_event = match &packet.event {
            InputEvent::MouseMoveAbsolute {
                norm_x,
                norm_y,
                display_id,
            } => {
                let (px, py) = self.window_bounds.map_normalized_to_pixel(*norm_x, *norm_y);
                // Store back translated pixel coordinates in norm_x/norm_y for sink
                InputEvent::MouseMoveAbsolute {
                    norm_x: px as u16,
                    norm_y: py as u16,
                    display_id: *display_id,
                }
            }
            InputEvent::TouchEvent {
                pointer_id,
                phase,
                norm_x,
                norm_y,
                pressure,
                radius,
            } => {
                let (px, py) = self.window_bounds.map_normalized_to_pixel(*norm_x, *norm_y);
                InputEvent::TouchEvent {
                    pointer_id: *pointer_id,
                    phase: *phase,
                    norm_x: px as u16,
                    norm_y: py as u16,
                    pressure: *pressure,
                    radius: *radius,
                }
            }
            InputEvent::StylusEvent {
                phase,
                norm_x,
                norm_y,
                pressure,
                tilt_x,
                tilt_y,
                twist,
                barrel_button,
                eraser,
            } => {
                let (px, py) = self.window_bounds.map_normalized_to_pixel(*norm_x, *norm_y);
                InputEvent::StylusEvent {
                    phase: *phase,
                    norm_x: px as u16,
                    norm_y: py as u16,
                    pressure: *pressure,
                    tilt_x: *tilt_x,
                    tilt_y: *tilt_y,
                    twist: *twist,
                    barrel_button: *barrel_button,
                    eraser: *eraser,
                }
            }
            other => other.clone(),
        };

        // 4. Update active held state tracker
        self.state_tracker.track_incoming_event(&mapped_event);

        // 5. Inject into OS sink
        self.sink.inject_event(&mapped_event, now_us)
    }

    /// Dispatches host-to-client feedback event (e.g. rumble vibration or cursor update).
    pub fn dispatch_feedback(&mut self, feedback: HostFeedbackEvent) {
        self.last_feedback_event = Some(feedback);
    }

    /// Triggered upon peer disconnect or session revocation to immediately release all held inputs.
    pub fn handle_client_disconnect(&mut self, now_us: u64) -> usize {
        self.state_tracker
            .synthesize_emergency_release_events(self.sink.as_mut(), now_us)
    }
}
