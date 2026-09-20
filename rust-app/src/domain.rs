use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSegment {
    #[serde(default)]
    pub id: String,
    pub text: String,
    #[serde(rename = "timestampMs", alias = "startTimeMs", default)]
    pub start_time_ms: u64,
    #[serde(rename = "endTimeMs")]
    pub end_time_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WorkItem {
    pub generation: u64,
    pub seq: u64,
    pub start_time_ms: u64,
    pub text: String,
    pub context: Vec<String>,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioResult {
    pub generation: u64,
    pub seq: u64,
    pub start_time_ms: u64,
    pub segment: Option<AudioSegment>, // None nếu thất bại sau retry
    pub trans_ms: u64,
    pub tts_ms: u64,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioSegment {
    pub id: String,
    pub audio_bytes: Vec<u8>,
    pub seq: u64,
    pub trans_ms: u64,
    pub tts_ms: u64,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ControlMessage {
    Play,
    Pause,
    Resume,
    Seek,
    Stop,
    SetSpeed(f32),
}
