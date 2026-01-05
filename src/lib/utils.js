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
/**
 * Build a tree structure from a flat list of keys
 * @param {object[]} keys - Array of { name, key_type }
 * @param {string} separator
 * @returns {{tree: object[], pathsToExpand: Set<string>}}
 */
export function buildTree(keys, separator = ":") {
  const root = [];
  const pathsToExpand = new Set();

  keys.forEach((keyInfo) => {
    const keyName = keyInfo.name;
    const parts = keyName.split(separator);
    let currentLevel = root;
    let currentPath = "";

    parts.forEach((part, index) => {
      const isLast = index === parts.length - 1;
      currentPath = currentPath ? `${currentPath}${separator}${part}` : part;

      if (!isLast) {
        pathsToExpand.add(currentPath);
      }

      let existingNode = currentLevel.find((node) => node.name === part);

      if (!existingNode) {
        existingNode = {
          name: part,
          type: isLast ? "key" : "folder",
          key_type: isLast ? keyInfo.key_type : null,
          children: [],
          fullPath: isLast ? keyName : null,
          keyCount: 0,
        };
        currentLevel.push(existingNode);
        // Sort: folders first, then keys, then alphabetically
        currentLevel.sort((a, b) => {
          if (a.type !== b.type) {
            return a.type === "folder" ? -1 : 1;
          }
          return a.name.localeCompare(b.name);
        });
      }
      currentLevel = existingNode.children;
    });
  });

  // Calculate key counts for folders
  const calculateCounts = (nodes) => {
    let total = 0;
    nodes.forEach((node) => {
      if (node.type === "key") {
        node.keyCount = 1;
      } else {
        node.keyCount = calculateCounts(node.children);
      }
      total += node.keyCount;
    });
    return total;
  };

  calculateCounts(root);

  return { tree: root, pathsToExpand };
}
