import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

/**
 * Resize window and center it on screen
 * @param {number} width
 * @param {number} height
 */
export async function resizeWindow(width, height) {
  try {
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(width, height));
    await win.center();
  } catch (e) {
    console.error("Failed to resize window:", e);
  }
}

/**
 * Format key value for display
 * @param {object} keyValue
 * @returns {string}
 */
export function formatKeyValue(keyValue) {
  if (!keyValue) return "";

  switch (keyValue.type) {
    case "String":
      return keyValue.value;
    case "List":
    case "Set":
    case "ZSet":
    case "Hash":
      return JSON.stringify(keyValue.value, null, 2);
    default:
      return "No data";
  }
}
