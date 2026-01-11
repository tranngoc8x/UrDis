import { writable } from "svelte/store";
import {
  loadServers,
  saveServers,
  migrateFromLocalStorage,
} from "./secureStore.js";

// Cấu hình Redis đang kết nối
export const activeConfig = writable(null);

// Danh sách servers đã lưu
function createSavedServersStore() {
  const { subscribe, set, update } = writable([]);

  // Initialize: Load from Tauri Store và migrate từ localStorage nếu cần
  (async () => {
    try {
      // Migration từ localStorage (chỉ chạy 1 lần)
      await migrateFromLocalStorage();

      // Load servers từ Store
      const servers = await loadServers();
      set(servers);
    } catch (error) {
      console.error("[Store] Failed to load servers:", error);
      set([]);
    }
  })();

  return {
    subscribe,
    set: async (value) => {
      set(value);
      try {
        await saveServers(value);
      } catch (error) {
        console.error("[Store] Failed to save servers:", error);
      }
    },
    update: (fn) => {
      update((current) => {
        const newValue = fn(current);
        // Async save
        saveServers(newValue).catch((error) => {
          console.error("[Store] Failed to update servers:", error);
        });
        return newValue;
      });
    },
    add: (server) => {
      update((servers) => {
        const exists = servers.some(
          (s) => s.host === server.host && s.port === server.port
        );
        if (!exists) {
          const newServers = [...servers, server];
          // Async save
          saveServers(newServers).catch((error) => {
            console.error("[Store] Failed to add server:", error);
          });
          return newServers;
        }
        return servers;
      });
    },
    remove: (index) => {
      update((servers) => {
        const newServers = servers.filter((_, i) => i !== index);
        // Async save
        saveServers(newServers).catch((error) => {
          console.error("[Store] Failed to remove server:", error);
        });
        return newServers;
      });
    },
  };
}

export const savedServers = createSavedServersStore();
