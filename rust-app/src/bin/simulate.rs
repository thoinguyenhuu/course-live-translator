use futures_util::SinkExt;
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("[SimClient] Đang kết nối tới ws://127.0.0.1:3847...");
    let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3847").await?;
    println!("[SimClient] Đã kết nối thành công!");

    let fixture_content = std::fs::read_to_string("../fixtures/lecture-sample.json")
        .or_else(|_| std::fs::read_to_string("fixtures/lecture-sample.json"))?;
    let sample: Vec<Value> = serde_json::from_str(&fixture_content)?;

    let mut last_time = 0u64;
    for entry in sample {
        let time = entry["time"].as_u64().unwrap_or(0);
        let text = entry["text"].as_str().unwrap_or("");

        if time > last_time {
            tokio::time::sleep(Duration::from_millis(time - last_time)).await;
            last_time = time;
        }

        println!("[SimClient] Gửi câu tại {}ms: \"{}\"", time, text);
        let payload = serde_json::json!({
            "type": "subtitle",
            "id": format!("seg-{}", time),
            "text": text,
            "timestampMs": chrono_now_ms()
        });

        ws_stream.send(Message::Text(payload.to_string())).await?;
    }

    println!("[SimClient] Đã gửi hết 10 câu mẫu. Đợi 5s hoàn tất...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let _ = ws_stream.close(None).await;
    println!("[SimClient] Hoàn tất mô phỏng!");

    Ok(())
}

fn chrono_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
