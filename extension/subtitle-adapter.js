class SubtitleAdapter {
  constructor(ws) {
    this.ws = ws;
    this.video = null;
    this.observer = null;
    this.lastText = '';
  }
  init() {
    setInterval(() => this.findVideo(), 2000);
    this.observeSubtitles();
  }
  findVideo() {
    const v = document.querySelector('video');
    if (v && v !== this.video) {
      this.video = v;
    }
  }
  observeSubtitles() {
    this.observer = new MutationObserver(() => {
      let text = null;

      // Cách 1: HTML5 textTracks API (universal — hoạt động trên mọi nền tảng)
      if (this.video && this.video.textTracks) {
        for (let i = 0; i < this.video.textTracks.length; i++) {
          let track = this.video.textTracks[i];
          if (track.mode === 'showing' && track.activeCues && track.activeCues.length > 0) {
            text = track.activeCues[0].text;
            break;
          }
        }
      }

      // Cách 2: DOM selectors (dự phòng, theo thứ tự ưu tiên)
      if (!text) {
        const selectors = [
          // Coursera
          '.vjs-text-track-display',
          '.rc-VideoCaption',
          // Udemy
          '[data-purpose="captions-cue-text"]',
          '[class*="captions-display--captions-cue-text"]',
          '[class*="captions-display--captions-container"]',
        ];
        for (const sel of selectors) {
          const el = document.querySelector(sel);
          if (el && el.innerText.trim()) {
            text = el.innerText;
            break;
          }
        }
      }

      // Cách 3: Transcript panel (dự phòng cuối)
      if (!text) {
        const transcriptSelectors = [
          // Coursera
          '.rc-Phrase.active',
          '[data-testid="interactive-transcript-cue-active"]',
          // Udemy
          '[data-purpose="transcript-cue-active"]',
          'p.transcript--highlight-cue-text',
        ];
        for (const sel of transcriptSelectors) {
          const el = document.querySelector(sel);
          if (el && el.innerText.trim()) {
            text = el.innerText;
            break;
          }
        }
      }

      if (text && text !== this.lastText && this.video) {
        this.lastText = text;
        this.ws.send({
          type: 'subtitle',
          id: Date.now().toString(),
          text: this.lastText,
          timestampMs: Math.round(this.video.currentTime * 1000)
        });
      }
    });
    this.observer.observe(document.body, { childList: true, subtree: true, characterData: true });
  }
}
window.SubtitleAdapter = SubtitleAdapter;
