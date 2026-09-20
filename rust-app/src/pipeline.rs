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