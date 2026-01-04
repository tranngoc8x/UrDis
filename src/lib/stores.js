import { writable } from "svelte/store";

// Cấu hình Redis đang kết nối
export const activeConfig = writable(null);

// Danh sách servers đã lưu
function createSavedServersStore() {
  const { subscribe, set, update } = writable([]);

  // Load from localStorage
  if (typeof localStorage !== "undefined") {
    const stored = localStorage.getItem("redis_servers");
    if (stored) {
      set(JSON.parse(stored));
    }
  }

  return {
    subscribe,
    set: (value) => {
      set(value);
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("redis_servers", JSON.stringify(value));
      }
    },
    update: (fn) => {
      update((current) => {
        const newValue = fn(current);
        if (typeof localStorage !== "undefined") {
          localStorage.setItem("redis_servers", JSON.stringify(newValue));
        }
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
          if (typeof localStorage !== "undefined") {
            localStorage.setItem("redis_servers", JSON.stringify(newServers));
          }
          return newServers;
        }
        return servers;
      });
    },
    remove: (index) => {
      update((servers) => {
        const newServers = servers.filter((_, i) => i !== index);
        if (typeof localStorage !== "undefined") {
          localStorage.setItem("redis_servers", JSON.stringify(newServers));
        }
        return newServers;
      });
    },
  };
}

export const savedServers = createSavedServersStore();
