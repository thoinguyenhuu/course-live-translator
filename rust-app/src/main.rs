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