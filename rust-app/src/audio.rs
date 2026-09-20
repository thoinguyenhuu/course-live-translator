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
                let sink_ref = self.sink.clone();
                let path_clone = seg.audio_path.clone();
                tokio::spawn(async move {
                    while !sink_ref.empty() {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                    let _ = tokio::fs::remove_file(path_clone).await;
                });
            }
        }
    }
    
    pub fn pause(&self) { self.sink.pause(); }
    pub fn resume(&self) { self.sink.play(); }
    pub fn clear(&self) { self.sink.clear(); }
}