import { Store } from "@tauri-apps/plugin-store";

let store = null;

/**
 * Khởi tạo Tauri Store
 * Store file: redis-connections.json
 */
async function initStore() {
  if (!store) {
    store = await Store.load("redis-connections.json");
  }
  return store;
}

/**
 * Load danh sách servers từ Store
 * @returns {Promise<Array>} Danh sách servers
 */
export async function loadServers() {
  const s = await initStore();
  const servers = await s.get("servers");
  return servers || [];
}

/**
 * Lưu danh sách servers vào Store
 * @param {Array} servers - Danh sách servers
 */
export async function saveServers(servers) {
  const s = await initStore();
  await s.set("servers", servers);
  await s.save();
}

/**
 * Migration từ localStorage sang Tauri Store
 * Chỉ chạy 1 lần khi phát hiện có data cũ
 */
export async function migrateFromLocalStorage() {
  // Kiểm tra xem đã migrate chưa
  const s = await initStore();
  const migrated = await s.get("migrated");

  if (migrated) {
    return; // Đã migrate rồi, skip
  }

  // Kiểm tra localStorage cũ
  if (typeof localStorage !== "undefined") {
    const oldData = localStorage.getItem("redis_servers");

    if (oldData) {
      try {
        const servers = JSON.parse(oldData);
        console.log(
          "[Migration] Found old localStorage data, migrating to Store..."
        );

        // Lưu vào Store
        await saveServers(servers);

        // Xóa localStorage cũ
        localStorage.removeItem("redis_servers");

        console.log("[Migration] Migration completed successfully!");
      } catch (error) {
        console.error("[Migration] Failed to migrate:", error);
      }
    }

    // Đánh dấu đã migrate
    await s.set("migrated", true);
    await s.save();
  }
}
