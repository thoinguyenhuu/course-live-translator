class CourseraAdapter {
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
      // Đã gỡ bỏ toàn bộ event điều khiển (play/pause/seek) theo yêu cầu.
      // Extension giờ chỉ là ống dẫn dữ liệu thuần túy (chỉ gửi subtitle).
    }
  }
  observeSubtitles() {
    this.observer = new MutationObserver(() => {
      let text = null;

      // Cách 1: Đọc trực tiếp từ HTML5 Video gốc (Chuẩn nhất, chống đổi giao diện)
      if (this.video && this.video.textTracks) {
        for (let i = 0; i < this.video.textTracks.length; i++) {
          let track = this.video.textTracks[i];
          if (track.mode === 'showing' && track.activeCues && track.activeCues.length > 0) {
            text = track.activeCues[0].text;
            break;
          }
        }
      }

      // Cách 2: Tìm trên màn hình (dự phòng)
      if (!text) {
        let display = document.querySelector('.vjs-text-track-display, .rc-VideoCaption');
        if (display && display.innerText.trim()) {
          text = display.innerText;
        }
      }

      // Cách 3: Tìm trong bảng Transcript bên dưới
      if (!text) {
        let altCue = document.querySelector('.rc-Phrase.active, [data-testid="interactive-transcript-cue-active"]');
        text = altCue ? altCue.innerText : null;
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
window.CourseraAdapter = CourseraAdapter;