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