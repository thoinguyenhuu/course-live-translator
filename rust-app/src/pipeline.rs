use crate::domain::*;
use crate::subtitle::{Deduplicator, SentenceBuffer};
use crate::translation::Translator;
use crate::tts::TtsProvider;
use crate::audio::AudioPlayer;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{mpsc, broadcast, Semaphore};

pub fn start_pipeline(
    mut subtitle_rx: mpsc::Receiver<SubtitleSegment>,
    mut control_rx: broadcast::Receiver<ControlMessage>,
) -> anyhow::Result<()> {
    let current_generation = Arc::new(AtomicU64::new(1));
    
    // Bounded channels
    let (tx_work, mut rx_work) = mpsc::channel::<WorkItem>(10);
    let (tx_reorder, mut rx_reorder) = mpsc::channel::<AudioResult>(10);
    let (tx_audio, mut rx_audio) = mpsc::channel::<AudioSegment>(10);

    let mut control_rx_reorder = control_rx.resubscribe();
    let mut control_rx_audio = control_rx.resubscribe();

    // -------------------------------------------------------------
    // STAGE 1: Subtitle Processor (Dedup + SentenceBuffer + Assign Seq)
    // -------------------------------------------------------------
    let gen_stage1 = current_generation.clone();
    tokio::spawn(async move {
        let mut dedup = Deduplicator::new();
        let mut buffer = SentenceBuffer::new();
        let mut context_window: VecDeque<String> = VecDeque::with_capacity(3);
        let mut seq_counter: u64 = 0;
        let mut sleep = Box::pin(tokio::time::sleep(tokio::time::Duration::from_secs(3600)));

        loop {
            tokio::select! {
                Some(seg) = subtitle_rx.recv() => {
                    sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_millis(1200));

                    if dedup.process(&seg) {
                        if let Some(stable_seg) = buffer.push(&seg) {
                            println!("[Subtitle] Ổn định câu: \"{}\"", stable_seg.text);
                            let gen = gen_stage1.load(Ordering::SeqCst);
                            let seq = seq_counter;
                            seq_counter += 1;

                            let context = context_window.iter().cloned().collect::<Vec<_>>();
                            context_window.push_back(stable_seg.text.clone());
                            if context_window.len() > 3 {
                                context_window.pop_front();
                            }

                            let work = WorkItem {
                                generation: gen,
                                seq,
                                start_time_ms: stable_seg.start_time_ms,
                                text: stable_seg.text,
                                context,
                            };
                            let _ = tx_work.send(work).await;
                        }
                    }
                }
                () = &mut sleep => {
                    if let Some(stable_seg) = buffer.flush() {
                        println!("[Subtitle] Tự động chốt câu sau 1.2s im lặng: \"{}\"", stable_seg.text);
                        let gen = gen_stage1.load(Ordering::SeqCst);
                        let seq = seq_counter;
                        seq_counter += 1;

                        let context = context_window.iter().cloned().collect::<Vec<_>>();
                        context_window.push_back(stable_seg.text.clone());
                        if context_window.len() > 3 {
                            context_window.pop_front();
                        }

                        let work = WorkItem {
                            generation: gen,
                            seq,
                            start_time_ms: stable_seg.start_time_ms,
                            text: stable_seg.text,
                            context,
                        };
                        let _ = tx_work.send(work).await;
                    }
                    sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3600));
                }
                Ok(msg) = control_rx.recv() => {
                    match msg {
                        ControlMessage::Seek | ControlMessage::Stop => {
                            println!("[Subtitle] Nhận Seek/Stop -> Tăng generation, reset buffer");
                            gen_stage1.fetch_add(1, Ordering::SeqCst);
                            dedup.reset();
                            buffer.reset();
                            context_window.clear();
                            seq_counter = 0;
                            sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3600));
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // -------------------------------------------------------------
    // STAGE 2: Bounded Concurrent Workers (Translate -> TTS)
    // -------------------------------------------------------------
    let translator = Arc::new(Translator::new());
    let tts_result = TtsProvider::new();
    let gen_stage2 = current_generation.clone();

    tokio::spawn(async move {
        let tts = match tts_result.await {
            Ok(t) => Arc::new(t),
            Err(e) => {
                eprintln!("[TTS] Lỗi khởi tạo TTS Provider: {}", e);
                return;
            }
        };

        // Giới hạn số lượng tác vụ dịch & TTS chạy đồng thời (tối đa 3 in-flight)
        let semaphore = Arc::new(Semaphore::new(3));

        while let Some(item) = rx_work.recv().await {
            let cur_gen = gen_stage2.load(Ordering::SeqCst);
            if item.generation != cur_gen {
                println!("[Worker] Bỏ qua seq {} vì generation cũ ({} != {})", item.seq, item.generation, cur_gen);
                continue;
            }

            let sem = semaphore.clone();
            let trans = translator.clone();
            let tts_engine = tts.clone();
            let tx_out = tx_reorder.clone();
            let gen_check = gen_stage2.clone();

            tokio::spawn(async move {
                let _permit = sem.acquire_owned().await.expect("Semaphore closed");

                if item.generation != gen_check.load(Ordering::SeqCst) {
                    return;
                }

                println!("[Worker] Bắt đầu xử lý song song: seq {} -> \"{}\"", item.seq, item.text);

                // 1. Dịch thuật với retry
                let trans_res = trans.translate(&item.text, &item.context).await;

                if item.generation != gen_check.load(Ordering::SeqCst) {
                    println!("[Worker] Hủy seq {} do generation đổi sau khi dịch", item.seq);
                    return;
                }

                // 2. Tạo giọng nói với retry
                let audio_segment = match trans_res {
                    Ok(vi_text) => {
                        println!("[Worker] Dịch xong seq {}: \"{}\" -> Gọi TTS", item.seq, vi_text);
                        match tts_engine.synthesize(&item.seq.to_string(), &vi_text).await {
                            Ok(audio) => Some(audio),
                            Err(e) => {
                                eprintln!("[Worker] TTS thất bại cho seq {}: {}", item.seq, e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Worker] Dịch thất bại cho seq {}: {}", item.seq, e);
                        None
                    }
                };

                if item.generation != gen_check.load(Ordering::SeqCst) {
                    if let Some(audio) = audio_segment {
                        let _ = std::fs::remove_file(&audio.audio_path);
                    }
                    return;
                }

                // Gửi về Reorder Buffer
                let _ = tx_out.send(AudioResult {
                    generation: item.generation,
                    seq: item.seq,
                    start_time_ms: item.start_time_ms,
                    segment: audio_segment,
                }).await;
            });
        }
    });

    // -------------------------------------------------------------
    // STAGE 3: Reorder Buffer + Prebuffer + Backpressure
    // -------------------------------------------------------------
    let gen_stage3 = current_generation.clone();
    tokio::spawn(async move {
        let mut buffer: BTreeMap<u64, AudioResult> = BTreeMap::new();
        let mut next_expected_seq: u64 = 0;
        let mut is_playing = false;
        let mut first_ready_time: Option<Instant> = None;

        let mut check_interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            tokio::select! {
                Some(res) = rx_reorder.recv() => {
                    let cur_gen = gen_stage3.load(Ordering::SeqCst);
                    if res.generation != cur_gen {
                        if let Some(a) = res.segment {
                            let _ = std::fs::remove_file(&a.audio_path);
                        }
                        continue;
                    }

                    println!("[Reorder] Nhận kết quả: seq {} (đang đợi seq {})", res.seq, next_expected_seq);
                    buffer.insert(res.seq, res);
                }
                _ = check_interval.tick() => {
                    let cur_gen = gen_stage3.load(Ordering::SeqCst);

                    // Backpressure: nếu buffer tích tụ quá lớn (> 10) hoặc quá cũ (> 15s), skip seq đang kẹt
                    if let Some(first_seq) = buffer.keys().next().cloned() {
                        if first_seq > next_expected_seq && (first_seq - next_expected_seq > 5 || buffer.len() > 8) {
                            println!("[Reorder] Backpressure: Nhảy qua các seq bị trễ/lỗi từ {} lên {}", next_expected_seq, first_seq);
                            next_expected_seq = first_seq;
                        }
                    }

                    // Prebuffer check:
                    // Khi chưa bắt đầu phát, đợi 1 segment tiếp theo có sẵn (1 segment ahead) HOẶC tối đa 600ms
                    if !is_playing {
                        if buffer.contains_key(&next_expected_seq) {
                            let has_ahead = buffer.contains_key(&(next_expected_seq + 1));
                            let time_elapsed = first_ready_time.map(|t| t.elapsed().as_millis() > 600).unwrap_or(false);

                            if has_ahead || time_elapsed {
                                is_playing = true;
                                first_ready_time = None;
                            } else if first_ready_time.is_none() {
                                first_ready_time = Some(Instant::now());
                            }
                        }
                    }

                    // Nhả audio theo đúng thứ tự tuần tự
                    if is_playing {
                        while let Some(res) = buffer.remove(&next_expected_seq) {
                            next_expected_seq += 1;

                            if res.generation != cur_gen {
                                if let Some(a) = res.segment {
                                    let _ = std::fs::remove_file(&a.audio_path);
                                }
                                continue;
                            }

                            if let Some(audio) = res.segment {
                                println!("[Reorder] Nhả audio đúng thứ tự: seq {} -> Loa", res.seq);
                                let _ = tx_audio.send(audio).await;
                            } else {
                                println!("[Reorder] Bỏ qua seq {} bị lỗi để không chặn bài giảng", res.seq);
                            }
                        }

                        // Nếu buffer rỗng, reset trạng thái prebuffer để đón đợt tiếp theo mượt mà
                        if buffer.is_empty() {
                            is_playing = false;
                        }
                    }
                }
                Ok(msg) = control_rx_reorder.recv() => {
                    match msg {
                        ControlMessage::Seek | ControlMessage::Stop => {
                            println!("[Reorder] Xóa sạch Reorder Buffer do Seek/Stop");
                            let drained = std::mem::take(&mut buffer);
                            for (_, res) in drained {
                                if let Some(a) = res.segment {
                                    let _ = std::fs::remove_file(&a.audio_path);
                                }
                            }
                            next_expected_seq = 0;
                            is_playing = false;
                            first_ready_time = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // -------------------------------------------------------------
    // STAGE 4: Persistent Audio Player (Dedicated OS Thread)
    // -------------------------------------------------------------
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Ok(player) = AudioPlayer::new() {
            println!("[Audio] Đã khởi tạo Persistent Audio Player");
            loop {
                let (audio_opt, msg_opt) = rt.block_on(async {
                    tokio::select! {
                        Some(a) = rx_audio.recv() => (Some(a), None),
                        Ok(m) = control_rx_audio.recv() => (None, Some(m)),
                        else => (None, None)
                    }
                });

                if let Some(audio) = audio_opt {
                    println!("[Audio] Đang phát audio: {}", audio.audio_path);
                    player.play_segment(audio);
                } else if let Some(msg) = msg_opt {
                    match msg {
                        ControlMessage::Pause => { println!("[Audio] Tạm dừng loa"); player.pause(); },
                        ControlMessage::Resume => { println!("[Audio] Tiếp tục loa"); player.resume(); },
                        ControlMessage::Seek | ControlMessage::Stop => { println!("[Audio] Xóa hàng đợi âm thanh cũ"); player.clear(); },
                        _ => {}
                    }
                }
            }
        } else {
            eprintln!("[Audio] Lỗi: Không thể mở thiết bị âm thanh hệ thống");
        }
    });

    Ok(())
}