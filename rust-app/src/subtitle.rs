use crate::domain::SubtitleSegment;

pub struct Deduplicator {
    last_text: String,
}

impl Deduplicator {
    pub fn new() -> Self { Self { last_text: String::new() } }
    
    pub fn process(&mut self, seg: &SubtitleSegment) -> bool {
        let text = seg.text.trim();
        if text.is_empty() || text == self.last_text {
            return false;
        }
        self.last_text = text.to_string();
        true
    }
    
    pub fn reset(&mut self) { self.last_text.clear(); }
}

pub struct SentenceBuffer {
    buffer: String,
    start_time_ms: u64,
    id: String,
}

impl SentenceBuffer {
    pub fn new() -> Self { 
        Self { 
            buffer: String::new(),
            start_time_ms: 0,
            id: String::new(),
        } 
    }
    
    pub fn push(&mut self, seg: &SubtitleSegment) -> Option<SubtitleSegment> {
        let text = seg.text.trim();
        if text.is_empty() { return None; }

        let is_empty = self.buffer.is_empty();

        // Cập nhật bộ đệm (Incremental update)
        // Nếu câu mới chứa câu cũ (VD: "The" -> "The database"), ghi đè
        if is_empty || text.starts_with(&self.buffer) {
            self.buffer = text.to_string();
            if is_empty {
                self.start_time_ms = seg.start_time_ms;
                self.id = seg.id.clone();
            }
        } else {
            // Câu mới không chứa câu cũ -> câu cũ có thể bị sót dấu chấm,
            // Flush câu cũ và bắt đầu câu mới
            let mut res = None;
            if !self.buffer.is_empty() {
                res = Some(SubtitleSegment {
                    id: self.id.clone(),
                    text: self.buffer.clone(),
                    start_time_ms: self.start_time_ms,
                    end_time_ms: None,
                });
            }
            self.buffer = text.to_string();
            self.start_time_ms = seg.start_time_ms;
            self.id = seg.id.clone();
            return res;
        }

        if self.buffer.ends_with('.') || self.buffer.ends_with('?') || self.buffer.ends_with('!') {
            let res = SubtitleSegment {
                id: self.id.clone(),
                text: self.buffer.clone(),
                start_time_ms: self.start_time_ms,
                end_time_ms: None,
            };
            self.buffer.clear();
            Some(res)
        } else {
            None
        }
    }
    
    pub fn flush(&mut self) -> Option<SubtitleSegment> {
        if self.buffer.is_empty() { return None; }
        let res = SubtitleSegment {
            id: self.id.clone(),
            text: self.buffer.clone(),
            start_time_ms: self.start_time_ms,
            end_time_ms: None,
        };
        self.buffer.clear();
        Some(res)
    }

    pub fn reset(&mut self) { 
        self.buffer.clear(); 
    }
}