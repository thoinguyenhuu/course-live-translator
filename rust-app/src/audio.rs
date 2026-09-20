use crate::domain::AudioSegment;
use rodio::{OutputStream, Sink, Decoder};
use std::fs::File;
use std::io::BufReader;

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioPlayer {
    pub fn new() -> anyhow::Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Self { _stream, sink })
    }

    pub fn play_segment(&self, seg: AudioSegment) {
        if let Ok(file) = File::open(&seg.audio_path) {
            let buf = BufReader::new(file);
            if let Ok(source) = Decoder::new(buf) {
                self.sink.append(source);
                let path_clone = seg.audio_path.clone();
                let dur = seg.duration_ms;
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(dur + 5000));
                    let _ = std::fs::remove_file(&path_clone);
                });
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    pub fn queue_len(&self) -> usize {
        self.sink.len()
    }

    pub fn pause(&self) { self.sink.pause(); }
    pub fn resume(&self) { self.sink.play(); }
    pub fn clear(&self) { self.sink.clear(); }
}