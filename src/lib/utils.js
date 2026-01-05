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
 * Format raw bytes for display
 * @param {Uint8Array|any} val
 * @returns {any}
 */
function formatRawValue(val) {
  if (val instanceof Uint8Array || Array.isArray(val)) {
    const bytes = val instanceof Uint8Array ? val : new Uint8Array(val);
    if (isPrintableUtf8(bytes)) {
      // Truncate extremely large strings for UI stability (1MB limit)
      const MAX_STRING_SIZE = 1024 * 1024;
      if (bytes.length > MAX_STRING_SIZE) {
        const truncated = bytes.slice(0, MAX_STRING_SIZE);
        return `${new TextDecoder().decode(
          truncated
        )}\n\n--- [Dữ liệu quá lớn, đã cắt bớt 1MB đầu tiên] ---`;
      }
      return new TextDecoder().decode(bytes);
    }
    return `[Binary] Hex: ${bytesToHex(bytes.slice(0, 100))}${
      bytes.length > 100 ? "..." : ""
    }`;
  }
  return val;
}

/**
 * Format key value for display
 * @param {object} keyValue
 * @returns {string}
 */
export function formatKeyValue(keyValue) {
  if (!keyValue || keyValue.value === null) return "";

  const { type, value } = keyValue;

  switch (type) {
    case "String":
      return formatRawValue(value);
    case "List":
    case "Set": {
      const limitedValue = value.slice(0, 500);
      let json = JSON.stringify(limitedValue.map(formatRawValue), null, 2);
      if (value.length > 500) {
        json += `\n\n--- [Vượt quá 500 phần tử, chỉ hiển thị ${limitedValue.length} phần tử đầu tiên] ---`;
      }
      return json;
    }
    case "ZSet": {
      const limitedValue = value.slice(0, 500);
      let json = JSON.stringify(
        limitedValue.map(([v, s]) => [formatRawValue(v), s]),
        null,
        2
      );
      if (value.length > 500) {
        json += `\n\n--- [Vượt quá 500 phần tử, chỉ hiển thị ${limitedValue.length} phần tử đầu tiên] ---`;
      }
      return json;
    }
    case "Hash": {
      const entries = Object.entries(value);
      const limitedEntries = entries.slice(0, 500);
      const formattedHash = {};
      for (const [k, v] of limitedEntries) {
        formattedHash[k] = formatRawValue(v);
      }
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
 * @param {object[]} keys - Array of { name, key_type }
 * @param {string} separator
 * @returns {{tree: object[], pathsToExpand: Set<string>}}
 */
export function buildTree(keys, separator = ":") {
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

  // Calculate inclusive key counts for folders
  const calculateCounts = (nodes) => {
    let total = 0;
    nodes.forEach((node) => {
      if (node.type === "key") {
        total += 1;
      } else {
        node.keyCount = calculateCounts(node.children);
        total += node.keyCount;
      }
    });
    return total;
  };

  calculateCounts(root);
  return { tree: root, pathsToExpand };
}
