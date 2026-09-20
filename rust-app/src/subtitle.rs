use std::collections::VecDeque;
use crate::domain::SubtitleSegment;

pub struct Deduplicator {
    recent_hashes: VecDeque<String>,
}

impl Deduplicator {
    pub fn new() -> Self { Self { recent_hashes: VecDeque::with_capacity(50) } }
    
    pub fn process(&mut self, seg: &SubtitleSegment) -> bool {
        if self.recent_hashes.contains(&seg.text) {
            false
        } else {
            if self.recent_hashes.len() >= 50 { self.recent_hashes.pop_front(); }
            self.recent_hashes.push_back(seg.text.clone());
            true
        }
    }
    
    pub fn reset(&mut self) { self.recent_hashes.clear(); }
}

pub struct SentenceBuffer {
    buffer: String,
}

impl SentenceBuffer {
    pub fn new() -> Self { Self { buffer: String::new() } }
    
    pub fn push(&mut self, seg: &SubtitleSegment) -> Option<SubtitleSegment> {
        self.buffer.push_str(&seg.text);
        self.buffer.push(' ');
        if self.buffer.ends_with(". ") || self.buffer.ends_with("? ") {
            let res = SubtitleSegment {
                id: seg.id.clone(),
                text: self.buffer.trim().to_string(),
                start_time_ms: seg.start_time_ms,
                end_time_ms: seg.end_time_ms,
            };
            self.buffer.clear();
            Some(res)
        } else {
            None
        }
    }
    
    pub fn reset(&mut self) { self.buffer.clear(); }
}