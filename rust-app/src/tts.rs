use crate::domain::AudioSegment;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
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
        Ok(Self {
            temp_dir,
            client: reqwest::Client::builder().timeout(Duration::from_secs(8)).build().unwrap(),
        })
    }

    pub async fn synthesize(&self, id: &str, text: &str) -> Result<AudioSegment> {
        let mut last_err = anyhow::anyhow!("Unknown TTS error");
        let retry_delays = [Duration::from_millis(500), Duration::from_millis(1000)];

        for attempt in 0..=retry_delays.len() {
            if attempt > 0 {
                tokio::time::sleep(retry_delays[attempt - 1]).await;
            }

            match self.do_synthesize(id, text).await {
                Ok(seg) => return Ok(seg),
                Err(e) => {
                    eprintln!("[TTS] Lần thử {} thất bại: {}", attempt + 1, e);
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    async fn do_synthesize(&self, id: &str, text: &str) -> Result<AudioSegment> {
        let file_name = format!("{}.mp3", Uuid::new_v4());
        let path = self.temp_dir.join(&file_name);

        // Google TTS giới hạn 200 ký tự mỗi request
        // Cắt text thành các cụm nhỏ theo từ nếu dài hơn 160 ký tự
        let chunks = split_text_into_chunks(text, 160);
        let mut combined_bytes: Vec<u8> = Vec::new();

        for chunk in chunks {
            let q = urlencoding::encode(&chunk);
            let url = format!("https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl=vi&client=tw-ob", q);
            let response = self.client.get(&url).send().await?.bytes().await?;

            if response.is_empty() {
                return Err(anyhow::anyhow!("Dữ liệu âm thanh TTS trả về rỗng"));
            }

            combined_bytes.extend_from_slice(&response);
        }

        if combined_bytes.is_empty() {
            return Err(anyhow::anyhow!("Không thể tạo âm thanh từ TTS"));
        }

        // Ước lượng thời lượng mp3 ~ 8kbps (8000 bits/s = 1000 bytes/s)
        let duration_ms = ((combined_bytes.len() as u64) / 1000) * 1000;

        fs::write(&path, &combined_bytes).await?;

        Ok(AudioSegment {
            id: id.to_string(),
            audio_path: path.to_string_lossy().to_string(),
            duration_ms: duration_ms.max(1000),
        })
    }
}

fn split_text_into_chunks(text: &str, max_len: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let words = text.split_whitespace().collect::<Vec<_>>();
    let mut current_chunk = String::new();

    for word in words {
        if current_chunk.is_empty() {
            current_chunk.push_str(word);
        } else if current_chunk.len() + 1 + word.len() <= max_len {
            current_chunk.push(' ');
            current_chunk.push_str(word);
        } else {
            chunks.push(current_chunk);
            current_chunk = word.to_string();
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    if chunks.is_empty() {
        chunks.push(text.to_string());
    }

    chunks
}