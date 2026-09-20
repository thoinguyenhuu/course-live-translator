const ws = new window.TranslatorWS('ws://127.0.0.1:3847');
ws.connect();
const adapter = new window.CourseraAdapter(ws);
adapter.init();