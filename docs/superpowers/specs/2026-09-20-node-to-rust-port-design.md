# Coursera Subtitle Translator - Rust Port

## 1. Overview
Port the existing Node.js/TypeScript Coursera subtitle translator backend to a native Rust executable. The Rust app will act as a local server receiving English subtitles via WebSocket from a Chrome extension, stabilizing and segmenting them, translating them via 9Router, synthesizing speech via Edge TTS, and playing the audio locally in the correct order.

## 2. Constraints & Goals
- **Parity:** Maintain exact logical behavior from the Node implementation.
- **Protocol:** Keep `ws://127.0.0.1:3847` and existing JSON message schemas.
- **No Heavy GUI:** Run headless or as a tiny system tray application.
- **Lightweight:** Bounded memory/queues, no DB, no embedded Chromium, no Docker.
- **Latency:** Target 3–8 seconds delay (1-2s prebuffer).
- **Cleanup:** Temporary audio files saved to `%TEMP%/coursera-translator/` and deleted immediately after playback.

## 3. Architecture & Concurrency
The application uses an Actor-like model with `tokio::sync::mpsc` channels to pass messages between distinct processing stages.

### 3.1. Components
1. **WebSocket Server (`tokio-tungstenite`):**
   - Binds to `ws://127.0.0.1:3847`.
   - Parses incoming JSON messages.
   - Forwards `subtitle`, `play`, `pause`, `seek`, and `lesson_change` messages to the Pipeline task.

2. **Pipeline/Subtitle Processor:**
   - **Deduplicator:** Tracks recent hashes (e.g., 50) to ignore duplicate subtitle chunks.
   - **Sentence Buffer:** Performs incremental stabilization and segmentation based on punctuation/timing. Maintains a rolling context (e.g., last 3 segments).
   - Emits completed segments to the Translate task.

3. **Translator (`reqwest`):**
   - Calls the local 9Router/Gemini endpoint (`http://127.0.0.1:20128/v1`).
   - Uses the exact prompt and configuration logic from the Node project.
   - Emits translated text to the TTS task.

4. **TTS Engine (`reqwest` / Edge TTS):**
   - Uses the Edge TTS provider API.
   - Writes generated audio to `%TEMP%/coursera-translator/<uuid>.mp3`.
   - Emits the file path to the Audio Player task.

5. **Audio Player (`rodio`):**
   - Manages a persistent audio output stream.
   - Enqueues files into a single `rodio::Sink` to ensure ordered playback.
   - Deletes the audio file from disk after it finishes playing.

### 3.2. Data Flow & Queues
All channels are bounded (e.g., capacity of 10) to prevent unbounded memory growth.
`WebSocket -> (mpsc) -> Pipeline -> (mpsc) -> Translator -> (mpsc) -> TTS -> (mpsc) -> Audio Player`

## 4. Control Signals
The pipeline must handle lifecycle and playback control messages:
- **Pause:** `rodio::Sink::pause()` is called.
- **Resume:** `rodio::Sink::play()` is called.
- **Seek / Lesson Change:**
  - A `Flush` or `Clear` signal is propagated through all channels.
  - The Pipeline resets its deduplication hashes, sentence buffer, and context.
  - Pending translation and TTS tasks for the old segment are dropped.
  - `rodio::Sink::clear()` stops current audio and clears the playback queue.

## 5. Directory Management
- On startup, the app creates `%TEMP%/coursera-translator/` if it does not exist and clears any leftover files from previous runs.
- On graceful shutdown, it cleans up the directory.

## 6. Migration & Verification
- The Rust implementation will be developed alongside the existing Node code.
- Verification involves running the same test scenario as Node (simulating subtitles) and ensuring 10/10 segments play with 0 errors and a comparable latency (avg ~3.6s, within the 3-8s budget).