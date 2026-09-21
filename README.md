# Course Live Translator 🎓🔊

Ứng dụng desktop nhẹ (Native Rust) kết hợp Chrome Extension để dịch và lồng tiếng bài giảng video theo thời gian thực (hỗ trợ Coursera, Udemy và các nền tảng học trực tuyến).

---

## 🌟 Tính năng nổi bật

- **Siêu nhẹ RAM:** Chỉ tốn ~17MB (idle) và ~22-26MB (khi đang dịch/phát audio). Giảm hơn 5 lần so với Node.js.
- **Hệ thống khay (System Tray):** Chạy ngầm góc màn hình, không mở cửa sổ console đen thừa.
- **Tùy chỉnh tốc độ đọc (1.0x - 1.5x):** Chỉnh trực tiếp qua menu chuột phải trên icon khay hệ thống hoặc qua `.env`.
- **Độ trễ thấp:** Pipeline đồng thời có Reorder Buffer, độ trễ trung bình chỉ ~2.8s - 3.5s.
- **Không ghi đĩa (Zero Disk I/O):** Giải mã âm thanh MP3 trực tiếp trên RAM, không tạo file tạm rác.
- **Khử trùng lặp & chốt câu thông minh:** Tự động gom câu hoàn chỉnh theo dấu chấm câu hoặc sau 1.2s im lặng.

---

## 📂 Cấu trúc dự án

```text
├── assets/          # Logo ứng dụng & icon khay hệ thống (.ico, .png)
├── extension/       # Chrome Extension Manifest V3 (bắt phụ đề video)
├── fixtures/        # Dữ liệu phụ đề mẫu phục vụ benchmark & kiểm thử
├── rust-app/        # Mã nguồn ứng dụng native Rust (System Tray, Pipeline, Audio)
└── tests/           # Kịch bản PowerShell đo đạc benchmark & RAM
```

---

## 🚀 Hướng dẫn cài đặt & sử dụng

### 1. Yêu cầu hệ thống
- Hệ điều hành: Windows 10/11
- Trình duyệt: Google Chrome, Edge hoặc Brave
- Rust toolchain (để build từ nguồn): [rustup.rs](https://rustup.rs/)

### 2. Cấu hình
Sao chép `.env.example` thành `.env` tại thư mục gốc:
```bash
cp .env.example .env
```
Điền thông tin API của bạn (Hỗ trợ mọi nhà cung cấp chuẩn OpenAI: OpenAI, Groq, OpenRouter, Ollama, vLLM...):
```env
PORT=3847
AI_API_BASE_URL=https://api.openai.com/v1
AI_API_KEY=your_api_key_here
MODEL_NAME=gpt-4o-mini
AUDIO_SPEED=1.15
```

### 3. Build & Chạy ứng dụng
Vào thư mục `rust-app`:
```bash
cd rust-app
cargo build --release
```
Chạy file thực thi:
```bash
target/release/coursera-rust-translator.exe
```
Ứng dụng sẽ xuất hiện icon mũ cử nhân xanh-cam trên **Khay hệ thống (System Tray)** góc dưới phải màn hình.

### 4. Cài đặt Chrome Extension
1. Mở Chrome và truy cập: `chrome://extensions/`
2. Bật công tắc **Developer mode** (Chế độ dành cho nhà phát triển).
3. Bấm **Load unpacked** (Tải tiện ích đã giải nén) và chọn thư mục `extension/` của dự án.
4. Mở bài giảng trên Coursera (hoặc nền tảng hỗ trợ), ứng dụng sẽ tự động kết nối và dịch giọng nói trực tiếp.

---

## ⚙️ Tùy chọn trên System Tray
Bấm chuột phải vào icon ở khay hệ thống để:
- Xem trạng thái kết nối WebSocket (`ws://127.0.0.1:3847`).
- Chỉnh tốc độ phát giọng nói: `1.0x`, `1.15x` (Khuyên dùng), `1.25x`, `1.5x`.
- Mở nhanh thư mục Extension.
- Thoát ứng dụng.

---

## 📄 License
MIT License
