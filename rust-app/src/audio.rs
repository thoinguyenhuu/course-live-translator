use crate::domain::AudioSegment;
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioPlayer {
    pub fn new() -> anyhow::Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let default_speed: f32 = std::env::var("AUDIO_SPEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.15);
        sink.set_speed(default_speed);
        Ok(Self { _stream, sink })
    }

    pub fn set_speed(&self, speed: f32) {
        self.sink.set_speed(speed.max(0.5).min(3.0));
    }

    pub fn play_segment(&self, seg: AudioSegment) -> (usize, bool) {
        let empty_before = self.sink.empty();
        let cursor = Cursor::new(seg.audio_bytes);
        match Decoder::new(cursor) {
            Ok(source) => {
                self.sink.append(source);
            }
            Err(e) => {
                eprintln!("[Audio] Lỗi decode MP3 in-memory: {}", e);
            }
        }
        (self.sink.len(), empty_before)
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    #[allow(dead_code)]
    pub fn queue_len(&self) -> usize {
        self.sink.len()
    }

    pub fn pause(&self) {
        self.sink.pause();
    }
    pub fn resume(&self) {
        self.sink.play();
    }
    pub fn clear(&self) {
        self.sink.clear();
    }
}
