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
 * Check if a Uint8Array is valid UTF-8 and contains printable characters
 * @param {Uint8Array} bytes
 * @returns {boolean}
 */
export function isPrintableUtf8(bytes) {
  if (!bytes || bytes.length === 0) return true;
  // Only check a sample of the data to avoid freezing on large strings
  const sample = bytes.length > 16384 ? bytes.slice(0, 16384) : bytes;
  try {
    const decoder = new TextDecoder("utf-8", { fatal: true });
    const text = decoder.decode(sample);
    // Allow null bytes and common printable characters.
    // Only reject if it has too many clearly unprintable control characters.
    // Excluding: \x00 (NULL), \x09 (TAB), \x0A (LF), \x0D (CR)
    // eslint-disable-next-line no-control-regex
    return !/[\x01-\x08\x0B\x0C\x0E-\x1F\x7F]/.test(text);
  } catch (e) {
    return false;
  }
}

/**
 * Convert bytes to hex string
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function bytesToHex(bytes) {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(" ");
}

/**
 * Format raw value based on its type and content
 * @param {string} val
 * @param {string} type - "String" or "Binary"
 * @returns {string}
 */
function formatRawValue(val, type = "String") {
  if (type === "Binary") {
    return `[Binary/Base64]: ${val.slice(0, 100)}${
      val.length > 100 ? "..." : ""
    }`;
  }

  // Truncate extremely large strings for UI stability (1MB limit)
  const MAX_STRING_SIZE = 1024 * 1024;
  if (val && val.length > MAX_STRING_SIZE) {
    return `${val.slice(
      0,
      MAX_STRING_SIZE
    )}\n\n--- [Dữ liệu quá lớn, đã cắt bớt 1MB đầu tiên] ---`;
  }

  return val;
}

/**
 * Format key value for display
 * @param {{type: string, value: any}} keyValue
 * @returns {string}
 */
export function formatKeyValue(keyValue) {
  if (!keyValue || (keyValue.type === "None" && !keyValue.value)) return "";

  const { type, value } = keyValue;

  switch (type) {
    case "String":
    case "Binary":
      return formatRawValue(value, type);
    case "List":
    case "Set": {
      const limitedValue = value.slice(0, 500);
      let json = JSON.stringify(limitedValue, null, 2);
      if (value.length > 500) {
        json += `\n\n--- [Vượt quá 500 phần tử, chỉ hiển thị ${limitedValue.length} phần tử đầu tiên] ---`;
      }
      return json;
    }
    case "ZSet": {
      const limitedValue = value.slice(0, 500);
      let json = JSON.stringify(limitedValue, null, 2);
      if (value.length > 500) {
        json += `\n\n--- [Vượt quá 500 phần tử, chỉ hiển thị ${limitedValue.length} phần tử đầu tiên] ---`;
      }
      return json;
    }
    case "Hash": {
      const entries = Object.entries(value);
      const limitedEntries = entries.slice(0, 500);
      const formattedHash = Object.fromEntries(limitedEntries);
      let json = JSON.stringify(formattedHash, null, 2);
      if (entries.length > 500) {
        json += `\n\n--- [Vượt quá 500 field, chỉ hiển thị ${limitedEntries.length} field đầu tiên] ---`;
      }
      return json;
    }
    default:
      return "No data";
  }
}
/**
 * Build a tree structure from a flat list of keys
 * @param {any[]} keys - Array of { name, key_type }
 * @param {string} separator
 * @returns {{tree: any[], pathsToExpand: Set<string>}}
 */
export function buildTree(keys, separator = ":") {
  /** @type {any[]} */
  const root = [];
  const pathsToExpand = new Set();
  const folderMap = new Map(); // Fast lookup: path -> folder object

  keys.forEach((keyInfo) => {
    const keyName = keyInfo.name;
    const parts = keyName.split(separator);
    let currentLevel = root;
    let currentPath = "";

    parts.forEach((part, index) => {
      const isLast = index === parts.length - 1;
      const partPath = currentPath ? `${currentPath}${separator}${part}` : part;

      if (!isLast) {
        let folder = folderMap.get(partPath);
        if (!folder) {
          folder = {
            name: part,
            type: "folder",
            children: [],
            keyCount: 0,
          };
          currentLevel.push(folder);
          folderMap.set(partPath, folder);
        }
        currentLevel = folder.children;
        currentPath = partPath;
        pathsToExpand.add(partPath);
      } else {
        currentLevel.push({
          name: part,
          fullPath: keyName,
          type: "key",
          key_type: keyInfo.key_type,
        });
      }
    });
  });

  // Sort nodes: folders first, then keys (alphabetically within each type)
  const sortNodes = (nodes) => {
    nodes.sort((a, b) => {
      // Folders first
      if (a.type === "folder" && b.type === "key") return -1;
      if (a.type === "key" && b.type === "folder") return 1;
      // Same type: sort alphabetically by name
      return a.name.localeCompare(b.name);
    });

    // Recursively sort children of folders
    nodes.forEach((node) => {
      if (node.type === "folder" && node.children) {
        sortNodes(node.children);
      }
    });
  };

  sortNodes(root);

  // Calculate key counts for all items
  const calculateCounts = (nodes) => {
    let total = 0;

    for (const node of nodes) {
      if (node.type === "key") {
        total += 1;
      } else {
        const folderCount = calculateCounts(node.children);
        node.keyCount = folderCount;
        total += folderCount;
      }
    }

    return total;
  };

  calculateCounts(root);
  return { tree: root, pathsToExpand };
}
