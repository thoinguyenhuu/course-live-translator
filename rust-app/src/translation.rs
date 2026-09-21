use anyhow::Result;
use serde_json::json;
use std::time::Duration;

const SYSTEM_PROMPT: &str = r#"You are translating an English university lecture into natural spoken Vietnamese.

Translate ONLY the CURRENT segment.

Context from previous segments is provided only to understand terminology.

Rules:
- Do not summarize.
- Do not remove information.
- Preserve technical meaning.
- Keep programming identifiers, API names, library names, product names, commands and code unchanged when appropriate.
- Translate technical terms naturally into Vietnamese.
- If a technical English term is commonly used in Vietnamese, it may be preserved.
- Produce natural spoken Vietnamese suitable for text-to-speech.
- Do not output explanations.
- Do not output markdown.
- Output only the Vietnamese translation."#;

pub struct Translator {
    client: reqwest::Client,
    base_url: String,
}

impl Translator {
    pub fn new() -> Self {
        let base_url = std::env::var("AI_API_BASE_URL")
            .or_else(|_| std::env::var("OPENAI_BASE_URL"))
            .or_else(|_| std::env::var("NINEROUTER_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap(),
            base_url,
        }
    }

    pub async fn translate(&self, text: &str, context: &[String]) -> Result<String> {
        let mut last_err = anyhow::anyhow!("Unknown error");
        let retry_delays = [Duration::from_millis(500), Duration::from_millis(1000)];

        for attempt in 0..=retry_delays.len() {
            if attempt > 0 {
                tokio::time::sleep(retry_delays[attempt - 1]).await;
            }

            match self.do_translate(text, context).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    eprintln!("[Translate] Lần thử {} thất bại: {}", attempt + 1, e);
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    async fn do_translate(&self, text: &str, context: &[String]) -> Result<String> {
        let model = std::env::var("TRANSLATION_MODEL")
            .or_else(|_| std::env::var("MODEL_NAME"))
            .unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let api_key = std::env::var("AI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .or_else(|_| std::env::var("NINEROUTER_API_KEY"))
            .unwrap_or_else(|_| "no-key".to_string());

        let context_block = if !context.is_empty() {
            let list = context
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}. {}", i + 1, c))
                .collect::<Vec<_>>()
                .join("\n");
            format!("Previous context:\n\n{}\n\n", list)
        } else {
            String::new()
        };

        let user_content = format!("{}CURRENT:\n{}", context_block, text);

        let req_body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_content}
            ],
            "temperature": 0.2,
            "stream": false,
            "max_tokens": 150
        });

        let res = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&req_body)
            .send()
            .await?;

        let status = res.status();
        let text_res = res.text().await?;

        let mut final_text = String::new();

        // Thử parse kiểu JSON gộp (không stream)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text_res) {
            if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                final_text = content.to_string();
            }
        }

        // Nếu không có kết quả, thử parse kiểu Stream (SSE - từng cụm chunk)
        if final_text.is_empty() {
            for line in text_res.lines() {
                let line = line.trim();
                if line.starts_with("data: ") {
                    let json_str = &line[6..];
                    if json_str == "[DONE]" {
                        break;
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            final_text.push_str(content);
                        }
                    }
                }
            }
        }

        final_text = final_text.trim().to_string();

        if final_text.is_empty() {
            return Err(anyhow::anyhow!(
                "Lỗi API 9Router (Mã {}): Dữ liệu rỗng hoặc sai định dạng. Raw: {}",
                status,
                text_res
            ));
        }

        Ok(final_text)
    }
}
