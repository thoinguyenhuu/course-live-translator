class TranslatorWS {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectDelay = 1000;
  }
  connect() {
    this.ws = new WebSocket(this.url);
    this.ws.onopen = () => { console.log('WS Connected'); this.reconnectDelay = 1000; };
    this.ws.onclose = () => {
      setTimeout(() => this.connect(), this.reconnectDelay);
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, 10000);
    };
    this.ws.onerror = () => this.ws.close();
  }
  send(data) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }
}
window.TranslatorWS = TranslatorWS;