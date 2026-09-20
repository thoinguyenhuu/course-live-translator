# Node to Rust Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Coursera Node.js backend to a lightweight native Rust executable that receives subtitles, translates them, synthesizes TTS, and plays audio locally.

**Architecture:** A channel-based actor model where tasks (WebSocket, Subtitle Buffer, Translator, TTS, Audio Player) communicate via `tokio::sync::mpsc` channels. A broadcast channel is used for control signals (pause, resume, seek, stop).

**Tech Stack:** Rust, tokio, tokio-tungstenite, reqwest, rodio, serde, tracing

**Spec:** docs/superpowers/specs/2026-09-20-node-to-rust-port-design.md

## Global Constraints

- Keep exact logical behavior from the Node implementation.
- WebSocket must listen on `ws://127.0.0.1:3847`.
- Do not send audio back to the browser.
- Use `%TEMP%/coursera-translator/` for temporary audio files and delete them after playback.
- No heavy GUI framework; headless or simple tray.
- Target 3-8 seconds latency with a 1-2s prebuffer.

## Review Focus

- The extension sends a `seek` message: Pending channels must be cleared, deduplicator/buffer reset, and the rodio audio sink must be cleared so old audio doesn't play.
- An empty text segment arrives: Deduplicator should ignore it or buffer shouldn't crash.
- File deletion timing: Temp files shouldn't be deleted while rodio is still reading them; they must be deleted exactly after playback finishes.
- Reconnection: Extension reconnects via WebSocket, the port 3847 must accept the new connection without dying.
- OOM safety: High volumes of subtitles must not build up infinitely; channels must be bounded.

---

### Task 1: Setup and Domain Types

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/domain.rs`

**Interfaces:**
- Produces: `SubtitleSegment`, `TranslatedSegment`, `AudioSegment`, `ControlMessage` types.

- [ ] **Step 1: Initialize project and add dependencies**

```bash
cargo init
cargo add tokio -F full
cargo add tokio-tungstenite
cargo add serde -F derive
cargo add serde_json
cargo add reqwest -F json,rustls-tls
cargo add rodio
cargo add tracing
cargo add tracing-subscriber
cargo add uuid -F v4
cargo add anyhow
```

- [ ] **Step 2: Define domain types**

Create `src/domain.rs` with the core structs matching the spec:

```rust
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
```

- [ ] **Step 3: Add module to main**

Update `src/main.rs`:

```rust
mod domain;

fn main() {
    println!("Hello, world!");
}
```

- [ ] **Step 4: Build project to verify**

Run: `cargo build`
Expected: Passes compilation

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/domain.rs
git commit -m "chore: setup rust project and domain types"
```

### Task 2: Subtitle Deduplicator and Buffer

**Files:**
- Create: `src/subtitle.rs`
- Create: `tests/subtitle_test.rs`
- Modify: `src/main.rs:1-3`

**Interfaces:**
- Consumes: `domain::SubtitleSegment`, `domain::ControlMessage`
- Produces: `SubtitleProcessor::process(msg)` which emits finalized `domain::SubtitleSegment` out of a channel.

- [ ] **Step 1: Write deduplicator/buffer logic test**

```rust
// tests/subtitle_test.rs
use coursera_learning::subtitle::{Deduplicator, SentenceBuffer};
use coursera_learning::domain::SubtitleSegment;

#[test]
fn test_deduplicator() {
    let mut dedup = Deduplicator::new();
    let seg1 = SubtitleSegment { id: "1".into(), text: "Hello".into(), start_time_ms: 0, end_time_ms: None };
    let seg2 = SubtitleSegment { id: "1".into(), text: "Hello".into(), start_time_ms: 0, end_time_ms: None };
    assert!(dedup.process(&seg1));
    assert!(!dedup.process(&seg2)); // Duplicate should be ignored
}
```
*(Add `name = "coursera_learning"` and `path = "src/main.rs"` in Cargo.toml `[lib]` if needed, or simply test within `src/subtitle.rs` using `#[cfg(test)]`)*

- [ ] **Step 2: Run test to fail**

Run: `cargo test`
Expected: FAIL - `subtitle` module not found

- [ ] **Step 3: Implement Deduplicator & Buffer**

```rust
// src/subtitle.rs
use std::collections::VecDeque;
use crate::domain::SubtitleSegment;

pub struct Deduplicator {
    recent_hashes: VecDeque<String>,
}

impl Deduplicator {
    pub fn new() -> Self { Self { recent_hashes: VecDeque::with_capacity(50) } }
    
    pub fn process(&mut self, seg: &SubtitleSegment) -> bool {
        if self.recent_hashes.contains(&seg.text) {
            false
        } else {
            if self.recent_hashes.len() >= 50 { self.recent_hashes.pop_front(); }
            self.recent_hashes.push_back(seg.text.clone());
            true
        }
    }
    
    pub fn reset(&mut self) { self.recent_hashes.clear(); }
}

pub struct SentenceBuffer {
    buffer: String,
}

impl SentenceBuffer {
    pub fn new() -> Self { Self { buffer: String::new() } }
    
    pub fn push(&mut self, seg: &SubtitleSegment) -> Option<SubtitleSegment> {
        self.buffer.push_str(&seg.text);
        self.buffer.push(' ');
        if self.buffer.ends_with(". ") || self.buffer.ends_with("? ") {
            let res = SubtitleSegment {
                id: seg.id.clone(),
                text: self.buffer.trim().to_string(),
                start_time_ms: seg.start_time_ms,
                end_time_ms: seg.end_time_ms,
            };
            self.buffer.clear();
            Some(res)
        } else {
            None
        }
    }
    
    pub fn reset(&mut self) { self.buffer.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_deduplicator() {
        let mut dedup = Deduplicator::new();
        let seg1 = SubtitleSegment { id: "1".into(), text: "Hello".into(), start_time_ms: 0, end_time_ms: None };
        assert!(dedup.process(&seg1));
        assert!(!dedup.process(&seg1));
    }
}
```

Add `mod subtitle;` to `src/main.rs`.

- [ ] **Step 4: Run test to pass**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/subtitle.rs src/main.rs
git commit -m "feat: subtitle deduplicator and sentence buffer"
```

### Task 3: Translation Provider

**Files:**
- Create: `src/translation.rs`
- Modify: `src/main.rs:1-4`

**Interfaces:**
- Consumes: `domain::SubtitleSegment`
- Produces: `domain::TranslatedSegment` via async HTTP call to `127.0.0.1:20128/v1`.

- [ ] **Step 1: Write test stub**

In `src/translation.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Needs integration test or mock; we'll verify it compiles
}
```

- [ ] **Step 2: Implement translation client**

```rust
// src/translation.rs
use crate::domain::{SubtitleSegment, TranslatedSegment};
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

pub struct Translator {
    client: reqwest::Client,
    base_url: String,
}

impl Translator {
    pub fn new() -> Self {
        let base_url = std::env::var("NINEROUTER_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:20128/v1".to_string());
        Self {
            client: reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap(),
            base_url,
        }
    }

    pub async fn translate(&self, seg: SubtitleSegment) -> Result<TranslatedSegment> {
        let model = std::env::var("TRANSLATION_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
        let res = self.client.post(&format!("{}/chat/completions", self.base_url))
            .json(&json!({
                "model": model,
                "messages": [{"role": "user", "content": format!("Translate to Vietnamese: {}", seg.text)}],
                "temperature": 0.3
            }))
            .send().await?
            .json::<serde_json::Value>().await?;
            
        let text = res["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        Ok(TranslatedSegment { id: seg.id, translated_text: text })
    }
}
```

Add `mod translation;` to `src/main.rs`.

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/translation.rs src/main.rs
git commit -m "feat: translation provider client"
```

### Task 4: TTS Provider

**Files:**
- Create: `src/tts.rs`
- Modify: `src/main.rs:1-5`

**Interfaces:**
- Consumes: `domain::TranslatedSegment`
- Produces: `domain::AudioSegment` pointing to an MP3 in `%TEMP%/coursera-translator/`

- [ ] **Step 1: Implement TTS generation**

```rust
// src/tts.rs
use crate::domain::{TranslatedSegment, AudioSegment};
use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;
use tokio::fs;

pub struct TtsProvider {
    temp_dir: PathBuf,
    client: reqwest::Client,
}

impl TtsProvider {
    pub async fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("coursera-translator");
        fs::create_dir_all(&temp_dir).await?;
        Ok(Self { temp_dir, client: reqwest::Client::new() })
    }

    pub async fn synthesize(&self, seg: TranslatedSegment) -> Result<AudioSegment> {
        let file_name = format!("{}.mp3", Uuid::new_v4());
        let path = self.temp_dir.join(&file_name);
        
        // Simple direct call to Google TTS (matching node's google-tts-api host and client parameter)
        // URL encode the text
        let q = urlencoding::encode(&seg.translated_text);
        let url = format!("https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl=vi&client=tw-ob", q);
        let response = self.client.get(&url).send().await?.bytes().await?;
        
        fs::write(&path, &response).await?;
        
        Ok(AudioSegment {
            id: seg.id,
            audio_path: path.to_string_lossy().to_string(),
        })
    }
}
```

Add `mod tts;` to `src/main.rs`.

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/tts.rs src/main.rs
git commit -m "feat: tts provider outputting temp files"
```

### Task 5: Audio Player

**Files:**
- Create: `src/audio.rs`
- Modify: `src/main.rs:1-6`

**Interfaces:**
- Consumes: `domain::AudioSegment` (plays and deletes), `domain::ControlMessage`

- [ ] **Step 1: Write rodio player implementation**

```rust
// src/audio.rs
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
                // Note: file deletion must happen after sink finishes playing.
                // In rodio, this is tricky. We'll spawn a cleanup task tracking sink length.
                let sink_ref = self.sink.clone();
                let path_clone = seg.audio_path.clone();
                tokio::spawn(async move {
                    // Poll until it's finished. A real implementation should use rodio's callbacks or sleep loop.
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
```

Add `mod audio;` to `src/main.rs`.

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/audio.rs src/main.rs
git commit -m "feat: rodio audio player with cleanup"
```

### Task 6: Pipeline Channels & Control Flow

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Wires up the tasks using `tokio::sync::mpsc` channels and spawns Tokio background tasks.

- [ ] **Step 1: Implement the actor loops**

```rust
// src/pipeline.rs
use crate::domain::*;
use crate::subtitle::{Deduplicator, SentenceBuffer};
use crate::translation::Translator;
use crate::tts::TtsProvider;
use crate::audio::AudioPlayer;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::broadcast;

pub fn start_pipeline(
    mut subtitle_rx: mpsc::Receiver<SubtitleSegment>,
    mut control_rx: broadcast::Receiver<ControlMessage>,
) -> anyhow::Result<()> {
    let (tx_trans, mut rx_trans) = mpsc::channel::<SubtitleSegment>(10);
    let (tx_tts, mut rx_tts) = mpsc::channel::<TranslatedSegment>(10);
    let (tx_audio, mut rx_audio) = mpsc::channel::<AudioSegment>(10);
    
    // Subtitle buffer loop
    tokio::spawn(async move {
        let mut dedup = Deduplicator::new();
        let mut buffer = SentenceBuffer::new();
        loop {
            tokio::select! {
                Some(seg) = subtitle_rx.recv() => {
                    if dedup.process(&seg) {
                        if let Some(complete) = buffer.push(&seg) {
                            let _ = tx_trans.send(complete).await;
                        }
                    }
                }
                Ok(msg) = control_rx.recv() => {
                    match msg {
                        ControlMessage::Seek | ControlMessage::Stop => {
                            dedup.reset();
                            buffer.reset();
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    let translator = Arc::new(Translator::new());
    tokio::spawn(async move {
        while let Some(seg) = rx_trans.recv().await {
            if let Ok(translated) = translator.translate(seg).await {
                let _ = tx_tts.send(translated).await;
            }
        }
    });

    // TTS loop
    tokio::spawn(async move {
        if let Ok(tts) = TtsProvider::new().await {
            while let Some(seg) = rx_tts.recv().await {
                if let Ok(audio) = tts.synthesize(seg).await {
                    let _ = tx_audio.send(audio).await;
                }
            }
        }
    });

    // Audio loop
    tokio::spawn(async move {
        if let Ok(player) = AudioPlayer::new() {
            while let Some(audio) = rx_audio.recv().await {
                player.play_segment(audio);
            }
        }
    });

    Ok(())
}
```

Add `cargo add urlencoding` to step 1. Add `mod pipeline;` to main.

- [ ] **Step 2: Commit**

### Task 7: WebSocket Server & Integration

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Starts the `tokio-tungstenite` TCP server on `127.0.0.1:3847`.

- [ ] **Step 1: Implement WS loop and wire it to pipeline**

```rust
// src/main.rs
mod domain;
mod subtitle;
mod translation;
mod tts;
mod audio;
mod pipeline;

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::StreamExt;
use tokio::sync::{mpsc, broadcast};
use serde_json::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let listener = TcpListener::bind("127.0.0.1:3847").await?;
    let (tx_subtitle, rx_subtitle) = mpsc::channel(10);
    let (tx_control, rx_control) = broadcast::channel(10);
    
    pipeline::start_pipeline(rx_subtitle, rx_control)?;

    while let Ok((stream, _)) = listener.accept().await {
        let tx_subtitle = tx_subtitle.clone();
        let tx_control = tx_control.clone();
        tokio::spawn(async move {
            if let Ok(mut ws_stream) = accept_async(stream).await {
                while let Some(Ok(msg)) = ws_stream.next().await {
                    if let Ok(text) = msg.to_text() {
                        if let Ok(json) = serde_json::from_str::<Value>(text) {
                            match json["type"].as_str() {
                                Some("subtitle") => {
                                    if let Ok(seg) = serde_json::from_value(json["payload"].clone()) {
                                        let _ = tx_subtitle.send(seg).await;
                                    }
                                }
                                Some("pause") => { let _ = tx_control.send(domain::ControlMessage::Pause); }
                                Some("resume") => { let _ = tx_control.send(domain::ControlMessage::Resume); }
                                Some("seek") => { let _ = tx_control.send(domain::ControlMessage::Seek); }
                                Some("stop") => { let _ = tx_control.send(domain::ControlMessage::Stop); }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });
    }
    Ok(())
}
```

- [ ] **Step 2: Run application**

Run: `cargo run`
Expected: Application starts and listens on 3847.

- [ ] **Step 3: Commit**

```bash
git commit -am "feat: websocket server and pipeline integration"
```
