//! Compact Binary Input Protocol & Anti-Replay Guard for Low-Latency Remote Input (UMCF S6.P3.SP3).
//!
//! Enforces sub-3ms serialization, bounded MTU datagram safety, and anti-replay sliding window.

use crate::error::StreamError;

/// Touch interaction lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TouchPhase {
    Down = 0,
    Move = 1,
    Up = 2,
    Cancel = 3,
}

impl TouchPhase {
    #[must_use]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Down),
            1 => Some(Self::Move),
            2 => Some(Self::Up),
            3 => Some(Self::Cancel),
            _ => None,
        }
    }
}

/// Stylus interaction lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StylusPhase {
    Hover = 0,
    Down = 1,
    Move = 2,
    Up = 3,
}

impl StylusPhase {
    #[must_use]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Hover),
            1 => Some(Self::Down),
            2 => Some(Self::Move),
            3 => Some(Self::Up),
            _ => None,
        }
    }
}

/// Cursor shape representation sent from host to client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CursorShapeType {
    DefaultArrow = 0,
    IBeamText = 1,
    Crosshair = 2,
    HandPointer = 3,
    ResizeAll = 4,
    ResizeHorizontal = 5,
    ResizeVertical = 6,
    NotAllowed = 7,
    Hidden = 8,
}

impl CursorShapeType {
    #[must_use]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::DefaultArrow),
            1 => Some(Self::IBeamText),
            2 => Some(Self::Crosshair),
            3 => Some(Self::HandPointer),
            4 => Some(Self::ResizeAll),
            5 => Some(Self::ResizeHorizontal),
            6 => Some(Self::ResizeVertical),
            7 => Some(Self::NotAllowed),
            8 => Some(Self::Hidden),
            _ => None,
        }
    }
}

/// Specific input event payload.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Keyboard {
        scancode: u16,
        hid_usage: u16,
        pressed: bool,
        modifiers: u8,
    },
    MouseMoveRelative {
        dx: i16,
        dy: i16,
        subpixel_dx: i8,
        subpixel_dy: i8,
    },
    MouseMoveAbsolute {
        norm_x: u16,
        norm_y: u16,
        display_id: u8,
    },
    MouseButton {
        button: u8, // 1: Left, 2: Right, 3: Middle, 4: Back, 5: Forward
        pressed: bool,
    },
    MouseScroll {
        wheel_x: i16,
        wheel_y: i16,
    },
    TouchEvent {
        pointer_id: u8,
        phase: TouchPhase,
        norm_x: u16,
        norm_y: u16,
        pressure: u16,
        radius: u16,
    },
    StylusEvent {
        phase: StylusPhase,
        norm_x: u16,
        norm_y: u16,
        pressure: u16,
        tilt_x: i16,
        tilt_y: i16,
        twist: u16,
        barrel_button: bool,
        eraser: bool,
    },
    GamepadState {
        controller_idx: u8,
        buttons_bitmask: u32,
        left_stick: (i16, i16),
        right_stick: (i16, i16),
        left_trigger: u16,
        right_trigger: u16,
    },
}

/// Compact binary envelope for all remote input packets.
#[derive(Debug, Clone, PartialEq)]
pub struct InputEventPacket {
    pub seq_no: u64,
    pub client_timestamp_us: u64,
    pub event: InputEvent,
}

impl InputEventPacket {
    /// Serializes the input packet into a compact binary representation (<= 36 bytes).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(36);
        buf.extend_from_slice(&self.seq_no.to_le_bytes());
        buf.extend_from_slice(&self.client_timestamp_us.to_le_bytes());

        match &self.event {
            InputEvent::Keyboard {
                scancode,
                hid_usage,
                pressed,
                modifiers,
            } => {
                buf.push(1); // Type 1: Keyboard
                buf.extend_from_slice(&scancode.to_le_bytes());
                buf.extend_from_slice(&hid_usage.to_le_bytes());
                buf.push(u8::from(*pressed));
                buf.push(*modifiers);
            }
            InputEvent::MouseMoveRelative {
                dx,
                dy,
                subpixel_dx,
                subpixel_dy,
            } => {
                buf.push(2); // Type 2: MouseMoveRelative
                buf.extend_from_slice(&dx.to_le_bytes());
                buf.extend_from_slice(&dy.to_le_bytes());
                buf.push(*subpixel_dx as u8);
                buf.push(*subpixel_dy as u8);
            }
            InputEvent::MouseMoveAbsolute {
                norm_x,
                norm_y,
                display_id,
            } => {
                buf.push(3); // Type 3: MouseMoveAbsolute
                buf.extend_from_slice(&norm_x.to_le_bytes());
                buf.extend_from_slice(&norm_y.to_le_bytes());
                buf.push(*display_id);
            }
            InputEvent::MouseButton { button, pressed } => {
                buf.push(4); // Type 4: MouseButton
                buf.push(*button);
                buf.push(u8::from(*pressed));
            }
            InputEvent::MouseScroll { wheel_x, wheel_y } => {
                buf.push(5); // Type 5: MouseScroll
                buf.extend_from_slice(&wheel_x.to_le_bytes());
                buf.extend_from_slice(&wheel_y.to_le_bytes());
            }
            InputEvent::TouchEvent {
                pointer_id,
                phase,
                norm_x,
                norm_y,
                pressure,
                radius,
            } => {
                buf.push(6); // Type 6: TouchEvent
                buf.push(*pointer_id);
                buf.push(*phase as u8);
                buf.extend_from_slice(&norm_x.to_le_bytes());
                buf.extend_from_slice(&norm_y.to_le_bytes());
                buf.extend_from_slice(&pressure.to_le_bytes());
                buf.extend_from_slice(&radius.to_le_bytes());
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
                buf.push(7); // Type 7: StylusEvent
                buf.push(*phase as u8);
                buf.extend_from_slice(&norm_x.to_le_bytes());
                buf.extend_from_slice(&norm_y.to_le_bytes());
                buf.extend_from_slice(&pressure.to_le_bytes());
                buf.extend_from_slice(&tilt_x.to_le_bytes());
                buf.extend_from_slice(&tilt_y.to_le_bytes());
                buf.extend_from_slice(&twist.to_le_bytes());
                let flags = (u8::from(*barrel_button)) | (u8::from(*eraser) << 1);
                buf.push(flags);
            }
            InputEvent::GamepadState {
                controller_idx,
                buttons_bitmask,
                left_stick,
                right_stick,
                left_trigger,
                right_trigger,
            } => {
                buf.push(8); // Type 8: GamepadState
                buf.push(*controller_idx);
                buf.extend_from_slice(&buttons_bitmask.to_le_bytes());
                buf.extend_from_slice(&left_stick.0.to_le_bytes());
                buf.extend_from_slice(&left_stick.1.to_le_bytes());
                buf.extend_from_slice(&right_stick.0.to_le_bytes());
                buf.extend_from_slice(&right_stick.1.to_le_bytes());
                buf.extend_from_slice(&left_trigger.to_le_bytes());
                buf.extend_from_slice(&right_trigger.to_le_bytes());
            }
        }

        buf
    }

    /// Deserializes an input packet from a binary slice with bounds validation.
    pub fn from_bytes(slice: &[u8]) -> Result<Self, StreamError> {
        if slice.len() < 17 {
            return Err(StreamError::TransportError("input packet too short".into()));
        }

        let seq_no = u64::from_le_bytes(slice[0..8].try_into().map_err(|_| {
            StreamError::TransportError("slice bounds mismatch".into())
        })?);
        let client_timestamp_us = u64::from_le_bytes(slice[8..16].try_into().map_err(|_| {
            StreamError::TransportError("slice bounds mismatch".into())
        })?);
        let event_type = slice[16];
        let payload = &slice[17..];

        let event = match event_type {
            1 => {
                if payload.len() < 6 {
                    return Err(StreamError::TransportError("invalid keyboard payload".into()));
                }
                let scancode = u16::from_le_bytes(payload[0..2].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let hid_usage = u16::from_le_bytes(payload[2..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let pressed = payload[4] != 0;
                let modifiers = payload[5];
                InputEvent::Keyboard {
                    scancode,
                    hid_usage,
                    pressed,
                    modifiers,
                }
            }
            2 => {
                if payload.len() < 6 {
                    return Err(StreamError::TransportError("invalid mouse rel payload".into()));
                }
                let dx = i16::from_le_bytes(payload[0..2].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let dy = i16::from_le_bytes(payload[2..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let subpixel_dx = payload[4] as i8;
                let subpixel_dy = payload[5] as i8;
                InputEvent::MouseMoveRelative {
                    dx,
                    dy,
                    subpixel_dx,
                    subpixel_dy,
                }
            }
            3 => {
                if payload.len() < 5 {
                    return Err(StreamError::TransportError("invalid mouse abs payload".into()));
                }
                let norm_x = u16::from_le_bytes(payload[0..2].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let norm_y = u16::from_le_bytes(payload[2..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let display_id = payload[4];
                InputEvent::MouseMoveAbsolute {
                    norm_x,
                    norm_y,
                    display_id,
                }
            }
            4 => {
                if payload.len() < 2 {
                    return Err(StreamError::TransportError("invalid mouse button payload".into()));
                }
                let button = payload[0];
                let pressed = payload[1] != 0;
                InputEvent::MouseButton { button, pressed }
            }
            5 => {
                if payload.len() < 4 {
                    return Err(StreamError::TransportError("invalid mouse scroll payload".into()));
                }
                let wheel_x = i16::from_le_bytes(payload[0..2].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let wheel_y = i16::from_le_bytes(payload[2..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                InputEvent::MouseScroll { wheel_x, wheel_y }
            }
            6 => {
                if payload.len() < 10 {
                    return Err(StreamError::TransportError("invalid touch payload".into()));
                }
                let pointer_id = payload[0];
                let phase = TouchPhase::from_u8(payload[1])
                    .ok_or_else(|| StreamError::TransportError("invalid touch phase".into()))?;
                let norm_x = u16::from_le_bytes(payload[2..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let norm_y = u16::from_le_bytes(payload[4..6].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let pressure = u16::from_le_bytes(payload[6..8].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let radius = u16::from_le_bytes(payload[8..10].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                InputEvent::TouchEvent {
                    pointer_id,
                    phase,
                    norm_x,
                    norm_y,
                    pressure,
                    radius,
                }
            }
            7 => {
                if payload.len() < 14 {
                    return Err(StreamError::TransportError("invalid stylus payload".into()));
                }
                let phase = StylusPhase::from_u8(payload[0])
                    .ok_or_else(|| StreamError::TransportError("invalid stylus phase".into()))?;
                let norm_x = u16::from_le_bytes(payload[1..3].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let norm_y = u16::from_le_bytes(payload[3..5].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let pressure = u16::from_le_bytes(payload[5..7].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let tilt_x = i16::from_le_bytes(payload[7..9].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let tilt_y = i16::from_le_bytes(payload[9..11].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let twist = u16::from_le_bytes(payload[11..13].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let flags = payload[13];
                let barrel_button = (flags & 0x01) != 0;
                let eraser = (flags & 0x02) != 0;
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
                }
            }
            8 => {
                if payload.len() < 17 {
                    return Err(StreamError::TransportError("invalid gamepad payload".into()));
                }
                let controller_idx = payload[0];
                let buttons_bitmask = u32::from_le_bytes(payload[1..5].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let stick_lx = i16::from_le_bytes(payload[5..7].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let stick_ly = i16::from_le_bytes(payload[7..9].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let stick_rx = i16::from_le_bytes(payload[9..11].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let stick_ry = i16::from_le_bytes(payload[11..13].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let left_trigger = u16::from_le_bytes(payload[13..15].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let right_trigger = u16::from_le_bytes(payload[15..17].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                InputEvent::GamepadState {
                    controller_idx,
                    buttons_bitmask,
                    left_stick: (stick_lx, stick_ly),
                    right_stick: (stick_rx, stick_ry),
                    left_trigger,
                    right_trigger,
                }
            }
            other => {
                return Err(StreamError::TransportError(format!(
                    "unknown input event type {other}"
                )));
            }
        };

        Ok(Self {
            seq_no,
            client_timestamp_us,
            event,
        })
    }
}

/// Host-to-Client feedback events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFeedbackEvent {
    GamepadRumble {
        controller_idx: u8,
        low_freq_motor: u16,
        high_freq_motor: u16,
        left_trigger_motor: u16,
        right_trigger_motor: u16,
        duration_ms: u16,
    },
    CursorShapeUpdate {
        shape: CursorShapeType,
        hotspot: (u16, u16),
    },
    ClipboardSync {
        mime_type: String,
        payload: Vec<u8>,
    },
}

impl HostFeedbackEvent {
    /// Serializes feedback event to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Self::GamepadRumble {
                controller_idx,
                low_freq_motor,
                high_freq_motor,
                left_trigger_motor,
                right_trigger_motor,
                duration_ms,
            } => {
                buf.push(1);
                buf.push(*controller_idx);
                buf.extend_from_slice(&low_freq_motor.to_le_bytes());
                buf.extend_from_slice(&high_freq_motor.to_le_bytes());
                buf.extend_from_slice(&left_trigger_motor.to_le_bytes());
                buf.extend_from_slice(&right_trigger_motor.to_le_bytes());
                buf.extend_from_slice(&duration_ms.to_le_bytes());
            }
            Self::CursorShapeUpdate { shape, hotspot } => {
                buf.push(2);
                buf.push(*shape as u8);
                buf.extend_from_slice(&hotspot.0.to_le_bytes());
                buf.extend_from_slice(&hotspot.1.to_le_bytes());
            }
            Self::ClipboardSync { mime_type, payload } => {
                buf.push(3);
                let mime_bytes = mime_type.as_bytes();
                buf.extend_from_slice(&(mime_bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(mime_bytes);
                buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                buf.extend_from_slice(payload);
            }
        }
        buf
    }

    /// Deserializes feedback event from bytes.
    pub fn from_bytes(slice: &[u8]) -> Result<Self, StreamError> {
        if slice.is_empty() {
            return Err(StreamError::TransportError("empty feedback slice".into()));
        }
        let feedback_type = slice[0];
        let payload = &slice[1..];
        match feedback_type {
            1 => {
                if payload.len() < 11 {
                    return Err(StreamError::TransportError("invalid rumble payload".into()));
                }
                let controller_idx = payload[0];
                let low_freq_motor = u16::from_le_bytes(payload[1..3].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let high_freq_motor = u16::from_le_bytes(payload[3..5].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let left_trigger_motor = u16::from_le_bytes(payload[5..7].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let right_trigger_motor = u16::from_le_bytes(payload[7..9].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let duration_ms = u16::from_le_bytes(payload[9..11].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                Ok(Self::GamepadRumble {
                    controller_idx,
                    low_freq_motor,
                    high_freq_motor,
                    left_trigger_motor,
                    right_trigger_motor,
                    duration_ms,
                })
            }
            2 => {
                if payload.len() < 5 {
                    return Err(StreamError::TransportError("invalid cursor payload".into()));
                }
                let shape = CursorShapeType::from_u8(payload[0])
                    .ok_or_else(|| StreamError::TransportError("invalid cursor shape".into()))?;
                let hotspot_x = u16::from_le_bytes(payload[1..3].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                let hotspot_y = u16::from_le_bytes(payload[3..5].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?);
                Ok(Self::CursorShapeUpdate {
                    shape,
                    hotspot: (hotspot_x, hotspot_y),
                })
            }
            3 => {
                if payload.len() < 6 {
                    return Err(StreamError::TransportError("invalid clipboard payload".into()));
                }
                let mime_len = u16::from_le_bytes(payload[0..2].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?) as usize;
                if payload.len() < 2 + mime_len + 4 {
                    return Err(StreamError::TransportError("truncated clipboard mime".into()));
                }
                let mime_type = std::str::from_utf8(&payload[2..2 + mime_len])
                    .map_err(|e| StreamError::TransportError(e.to_string()))?
                    .to_string();
                let rest = &payload[2 + mime_len..];
                let payload_len = u32::from_le_bytes(rest[0..4].try_into().map_err(|_| {
                    StreamError::TransportError("slice bounds mismatch".into())
                })?) as usize;
                if rest.len() < 4 + payload_len {
                    return Err(StreamError::TransportError("truncated clipboard data".into()));
                }
                let clip_data = rest[4..4 + payload_len].to_vec();
                Ok(Self::ClipboardSync {
                    mime_type,
                    payload: clip_data,
                })
            }
            other => Err(StreamError::TransportError(format!(
                "unknown feedback event type {other}"
            ))),
        }
    }
}

/// Anti-replay sliding window tracking received sequence numbers.
#[derive(Debug, Clone)]
pub struct AntiReplayWindow {
    max_seq: u64,
    bitmap: u64, // 64-packet history sliding window
    window_size: u64,
}

impl Default for AntiReplayWindow {
    fn default() -> Self {
        Self::new(64)
    }
}

impl AntiReplayWindow {
    #[must_use]
    pub fn new(window_size: u64) -> Self {
        Self {
            max_seq: 0,
            bitmap: 0,
            window_size: window_size.clamp(1, 64),
        }
    }

    /// Verifies if a sequence number is valid (not replayed or stale) and marks it as seen.
    pub fn check_and_update(&mut self, seq_no: u64) -> bool {
        if seq_no == 0 {
            return false;
        }

        if seq_no > self.max_seq {
            let diff = seq_no - self.max_seq;
            if diff < 64 {
                self.bitmap <<= diff;
                self.bitmap |= 1;
            } else {
                self.bitmap = 1;
            }
            self.max_seq = seq_no;
            return true;
        }

        let diff = self.max_seq - seq_no;
        if diff >= self.window_size {
            return false; // Too old, fallen off window
        }

        let mask = 1u64 << diff;
        if (self.bitmap & mask) != 0 {
            return false; // Already seen, replay attack!
        }

        self.bitmap |= mask;
        true
    }
}
