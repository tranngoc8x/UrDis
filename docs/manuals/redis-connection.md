# Hướng dẫn Kết nối Redis - UrDis

Tài liệu này hướng dẫn cách sử dụng tính năng kết nối Redis trong ứng dụng UrDis.

## 1. Bối cảnh
Người dùng cần một giao diện trực quan để kết nối và quản lý các máy chủ Redis. Bước đầu tiên là thiết lập kết nối an toàn và xác thực trạng thái máy chủ.

## 2. Cách sử dụng
1. Mở ứng dụng UrDis.
2. Tìm đến phần **Redis Connection**.
3. Nhập URL của Redis server vào ô input.
   - Định dạng mặc định: `redis://127.0.0.1:6379`
   - Nếu có mật khẩu: `redis://:password@host:port`
4. Nhấn nút **Connect**.

## 3. Trạng thái kết nối
- **Connecting...**: Ứng dụng đang thực hiện lệnh PING tới server.
- **Successfully connected to Redis server!**: Kết nối thành công (nhận được phản hồi PONG).
- **Error**: Hiển thị chi tiết lỗi nếu không thể kết nối (sai URL, server offline, lỗi mạng...).

## 4. Kỹ thuật (Dành cho Dev)
- **Backend**: Sử dụng crate `redis` với tính năng `tokio-comp` để xử lý async.
- **Tauri Command**: `connect_redis` nhận tham số `url` và trả về `Result<String, String>`.
- **Frontend**: SvelteKit gọi backend thông qua hàm `invoke("connect_redis", { url })`.

---
*Tài liệu được tạo tự động bởi Zeno Agent 🧠*
