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