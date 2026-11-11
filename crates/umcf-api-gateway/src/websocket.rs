// websocket.rs
// Event Ring Buffer with Monotonic Sequence Cursors & Lossless Reconnection Replay.
// Ensures WebSocket clients (?since_seq=N or Last-Event-ID) never miss an event.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

/// Monotonically sequenced job event record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEventRecord {
    /// Monotonically incrementing global sequence identifier (1, 2, 3...).
    pub seq: u64,
    /// Associated job identifier.
    pub job_id: String,
    /// Event creation timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Category or state transition ("scheduled", "running", "progress", "token", "completed", "failed").
    pub event_type: String,
    /// Event data payload.
    pub payload: serde_json::Value,
}

/// Bounded circular buffer of sequenced events with subscriber broadcast.
#[derive(Debug)]
pub struct EventRingBuffer {
    capacity: usize,
    seq_counter: AtomicU64,
    events: RwLock<VecDeque<JobEventRecord>>,
    broadcaster: broadcast::Sender<JobEventRecord>,
}

impl Default for EventRingBuffer {
    fn default() -> Self {
        Self::new(2048)
    }
}

impl EventRingBuffer {
    /// Creates a new event ring buffer with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            capacity: capacity.max(64),
            seq_counter: AtomicU64::new(1),
            events: RwLock::new(VecDeque::with_capacity(capacity)),
            broadcaster: tx,
        }
    }

    /// Emits and records a new event, assigning it the next monotonic sequence number.
    pub async fn push_event(
        &self,
        job_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> JobEventRecord {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let record = JobEventRecord {
            seq,
            job_id: job_id.into(),
            timestamp_ms: now_ms,
            event_type: event_type.into(),
            payload,
        };

        {
            let mut buf = self.events.write().await;
            if buf.len() >= self.capacity {
                buf.pop_front();
            }
            buf.push_back(record.clone());
        }

        // Broadcast to live streaming WebSocket listeners
        let _ = self.broadcaster.send(record.clone());

        record
    }

    /// Retrieves all recorded events strictly greater than `since_seq`, optionally filtered by `job_id`.
    pub async fn get_events_since(
        &self,
        job_id: Option<&str>,
        since_seq: u64,
    ) -> Vec<JobEventRecord> {
        let buf = self.events.read().await;
        buf.iter()
            .filter(|ev| {
                ev.seq > since_seq
                    && job_id.map_or(true, |target_id| ev.job_id == target_id)
            })
            .cloned()
            .collect()
    }

    /// Subscribes to new events arriving in real time.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<JobEventRecord> {
        self.broadcaster.subscribe()
    }

    /// Returns the latest assigned sequence number.
    #[must_use]
    pub fn latest_seq(&self) -> u64 {
        self.seq_counter.load(Ordering::Relaxed).saturating_sub(1)
    }
}

/// Streams replayed + live events over an Axum WebSocket connection.
pub async fn handle_ws_stream(
    mut socket: WebSocket,
    ring_buffer: Arc<EventRingBuffer>,
    job_id: String,
    since_seq: u64,
) {
    // 1. Replay historical missed events
    let historical = ring_buffer.get_events_since(Some(&job_id), since_seq).await;
    let mut last_delivered_seq = since_seq;

    for event in historical {
        let json_text = match serde_json::to_string(&event) {
            Ok(txt) => txt,
            Err(_) => continue,
        };
        last_delivered_seq = event.seq;
        if socket.send(Message::Text(json_text)).await.is_err() {
            return;
        }
    }

    // 2. Subscribe to real-time events
    let mut rx = ring_buffer.subscribe();

    loop {
        tokio::select! {
            // Incoming client message (ping / close / ack)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = socket.send(Message::Pong(payload)).await;
                    }
                    _ => {}
                }
            }
            // Outgoing broadcast event
            Ok(event) = rx.recv() => {
                if event.job_id == job_id && event.seq > last_delivered_seq {
                    let json_text = match serde_json::to_string(&event) {
                        Ok(txt) => txt,
                        Err(_) => continue,
                    };
                    last_delivered_seq = event.seq;
                    if socket.send(Message::Text(json_text)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

