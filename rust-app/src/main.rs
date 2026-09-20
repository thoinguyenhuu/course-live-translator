#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod domain;
mod pipeline;
mod subtitle;
mod translation;
mod tts;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::accept_async;
use tray_item::{IconSource, TrayItem};

#[cfg(windows)]
extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}
#[cfg(windows)]
const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;

fn load_env() {
    let env_paths = ["../.env", ".env", "../../.env"];
    for path in env_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    std::env::set_var(key.trim(), value.trim());
                }
            }
            break;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    load_env();
    tracing_subscriber::fmt::init();
    let listener = TcpListener::bind("127.0.0.1:3847").await?;
    let (tx_subtitle, rx_subtitle) = mpsc::channel(10);
    let (tx_control, _rx_control) = broadcast::channel(10);

    pipeline::start_pipeline(rx_subtitle, tx_control.subscribe())?;

    let mut _tray = match TrayItem::new("Coursera Translator", IconSource::Resource("#1")) {
        Ok(mut tray) => {
            let _ = tray.add_label("Coursera Translator");
            let _ = tray.add_label("Cổng: 3847 (Đang chạy)");
            let _ = tray.inner_mut().add_separator();
            let _ = tray.add_menu_item("Mở thư mục Extension", || {
                let _ = std::process::Command::new("explorer")
                    .arg("..\\extension")
                    .spawn();
            });
            let _ = tray.inner_mut().add_separator();
            let _ = tray.add_menu_item("Thoát", || {
                println!("[SystemTray] Người dùng chọn Thoát");
                std::process::exit(0);
            });
            println!("[SystemTray] Đã khởi tạo icon trên khay hệ thống");
            Some(tray)
        }
        Err(e) => {
            eprintln!(
                "[SystemTray] Cảnh báo: Không thể tạo icon khay hệ thống: {:?}",
                e
            );
            None
        }
    };

    println!("Coursera Rust Translator is running!");
    println!("Listening on ws://127.0.0.1:3847");
    println!("Keep this window open while studying.");

    while let Ok((stream, _)) = listener.accept().await {
        println!("Extension connected!");
        let tx_subtitle = tx_subtitle.clone();
        let tx_control = tx_control.clone();
        tokio::spawn(async move {
            if let Ok(mut ws_stream) = accept_async(stream).await {
                while let Some(Ok(msg)) = ws_stream.next().await {
                    if let Ok(text) = msg.to_text() {
                        if let Ok(json) = serde_json::from_str::<Value>(text) {
                            match json["type"].as_str() {
                                Some("subtitle") => {
                                    match serde_json::from_value::<domain::SubtitleSegment>(
                                        json.clone(),
                                    ) {
                                        Ok(seg) => {
                                            let _ = tx_subtitle.send(seg).await;
                                        }
                                        Err(e) => {
                                            println!(
                                                "[Lỗi] Parse JSON phụ đề thất bại: {}. Data: {}",
                                                e, json
                                            );
                                        }
                                    }
                                }
                                Some("pause") => {
                                    let _ = tx_control.send(domain::ControlMessage::Pause);
                                }
                                Some("resume") => {
                                    let _ = tx_control.send(domain::ControlMessage::Resume);
                                }
                                Some("seek") => {
                                    let _ = tx_control.send(domain::ControlMessage::Seek);
                                }
                                Some("stop") => {
                                    let _ = tx_control.send(domain::ControlMessage::Stop);
                                }
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
