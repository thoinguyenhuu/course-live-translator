use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSegment {
    pub id: String,
    pub text: String,
    #[serde(rename = "startTimeMs")]
    pub start_time_ms: u64,
    #[serde(rename = "endTimeMs")]
    pub end_time_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TranslatedSegment {
    pub id: String,
    pub translated_text: String,
}

#[derive(Debug, Clone)]
pub struct AudioSegment {
    pub id: String,
    pub audio_path: String,
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    Play,
    Pause,
    Resume,
    Seek,
    Stop,
}