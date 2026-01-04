# Task: Triển khai tính năng kết nối Redis Server và giao diện người dùng đầu tiên

- **ID:** URDIS-001
- **Trạng thái:** ĐÃ REVIEW & SỬA LỖI ✅ (Review by Serena)
- **Người thực hiện:** Zeno
- **Reviewer:** Serena (Adversarial Review)
- **Ước lượng:** 1 Story Point

## Bối cảnh
Người dùng cần một cách để kết nối ứng dụng UrDis với máy chủ Redis để quản lý dữ liệu. Hiện tại ứng dụng chỉ có giao diện chào hỏi mặc định từ template Tauri.

## Review & Fix Record (by Serena)
Trong quá trình review, tôi đã phát hiện và xử lý các vấn đề sau:
1. **Security (High)**: Ngăn chặn rò rỉ thông tin nhạy cảm bằng cách map lại thông báo lỗi từ backend và thêm validation URL phía frontend.
2. **Automated Tests (High)**: Bổ sung Unit Test cho logic kết nối trong Rust. Đã thêm `tokio` vào `dev-dependencies`.
3. **Performance (Medium)**: Tách logic xử lý để chuẩn bị cho việc sử dụng Connection Pool/State sau này.
4. **UI/UX (Low)**: Tối ưu hóa giao diện Dark Mode (độ tương phản, màu sắc Material Design).

## Phương án thực hiện
1. **Backend (Rust):**
   - Thêm dependency `redis` với feature `tokio-comp`.
   - Tạo lệnh `connect_redis` (async) để thực hiện `PING` tới server.
   - [Bổ sung] Tách hàm `perform_redis_check` để test độc lập.
2. **Frontend (SvelteKit):**
   - Tạo biến trạng thái bằng `$state`.
   - Xây dựng form nhập liệu với URL mặc định.
   - [Bổ sung] Thêm Regex validation cho URL.
3. **Tài liệu:**
   - Tạo hướng dẫn sử dụng tại `docs/manuals/redis-connection.md`.

## Kết quả
- [x] Backend kết nối thành công tới Redis.
- [x] UI hiển thị trực quan, hỗ trợ Dark Mode chuẩn.
- [x] Xử lý lỗi bảo mật và validation URL.
- [x] Tài liệu hướng dẫn sử dụng hoàn tất.
- [x] Unit Test cho backend hoàn thành và Pass.

## Hướng dẫn kiểm thử
1. Chạy một Redis server cục bộ (mặc định port 6379).
2. Chạy test backend: `cd src-tauri && cargo test`.
3. Nhấn nút "Connect" trên ứng dụng UrDis.
4. Kiểm tra thông báo màu xanh "Successfully connected".
