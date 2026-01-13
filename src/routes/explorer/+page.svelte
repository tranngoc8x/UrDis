<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig } from "$lib/stores.js";
  import { resizeWindow, formatKeyValue, buildTree } from "$lib/utils.js";
  import SimpleBar from "simplebar";
  import HashEditor from "./HashEditor.svelte";
  import "simplebar/dist/simplebar.css";

  // State Management
  /** @type {any[]} */
  let keysList = $state([]);
  /** @type {any[]} */
  let allKeys = $state([]); // Store all keys when fully loaded
  let selectedKey = $state("");
  let keyValue = $state({
    key_type: null,
    value: null,
    ttl: -1,
    memory: 0,
    encoding: "",
  });
  let valueCache = new Map(); // Cache for near-instant display

  /** @type {string} */
  let searchInput = $state("");
  let activePattern = $state("");
  let selectedDb = $state(0);
  /** @type {number[]} */
  let dbSizes = $state([]);
  let isDropdownOpen = $state(false);
  let isAddKeyDropdownOpen = $state(false);
  let expandedFolders = $state(new Set());
  const redisKeyTypes = ["String", "Hash", "List", "Set", "ZSet", "Stream"];
  let currentCursor = $state(0);
  let isScanning = $state(true);

  // Multi-select & Context Menu
  /** @type {string[]} */
  let selectedKeys = $state([]); // Array of strings
  let lastSelectedKey = $state("");
  let showContextMenu = $state(false);
  let contextMenuKey = $state(""); // The key right-clicked on
  let contextMenuPosition = $state({ x: 0, y: 0 });

  // Confirmation Dialog
  let showConfirmDialog = $state(false);
  let confirmDialogMessage = $state("");
  let confirmDialogCallback = $state(() => {});

  // TTL Dialog
  let showTTLDialog = $state(false);
  let ttlValue = $state(0);
  let ttlUnit = $state("seconds"); // seconds, minutes, hours, days, weeks, months, years
  let ttlTargetKey = $state("");

  let editableContent = $state("");
  let originalContent = $state("");
  let isModified = $derived(editableContent !== originalContent);
  /** @type {HTMLElement|null} */
  let editorNode = $state(null);

  /** @type {any} */
  let valueLoadingTimeout;
  let isLoadingValue = $state(false);

  let selectedKeyType = $derived(
    keysList.find((k) => k.name === selectedKey)?.key_type || ""
  );

  // Svelte action for SimpleBar
  /** @param {HTMLElement} node */
  function simplebar(node, options = { autoHide: false }) {
    const sb = new SimpleBar(node, options);
    // Optimization: Throttle recalculates to avoid forced reflow storms
    /** @type {number} */
    let rafId;
    const observer = new MutationObserver(() => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => sb.recalculate());
    });
    observer.observe(node, {
      childList: true,
      subtree: true,
      characterData: true,
    });

    return {
      destroy() {
        observer.disconnect();
        sb.unMount();
      },
    };
  }

  // Sidebar resizing logic
  let sidebarWidth = $state(400);
  let isResizing = $state(false);

  function startResizing(event) {
    isResizing = true;
    event.preventDefault();
  }

  function stopResizing() {
    isResizing = false;
  }

  function handleMouseMove(event) {
    if (!isResizing) return;
    const maxWidth = window.innerWidth * (2 / 3);
    const minWidth = 200;
    const newWidth = event.clientX;
    if (newWidth >= minWidth && newWidth <= maxWidth) {
      sidebarWidth = newWidth;
    }
  }

  // Lifecycle & Data Fetching
  const dbOptions = Array.from({ length: 16 }, (_, i) => i);

  onMount(() => {
    console.log("[UrDis] Component mounted. isScanning:", isScanning);

    // Global click diagnostic
    window.addEventListener(
      "mousedown",
      (e) => {
        console.log("[UrDis] Global Mousedown:", e.target);
      },
      true
    );

    if (!$activeConfig) {
      goto("/login");
    } else {
      fetchDbSizes();
      fetchKeys();
    }
  });

  async function fetchDbSizes() {
    const config = $activeConfig;
    if (!config) return;
    try {
      dbSizes = await invoke("get_db_sizes", { config });
    } catch (error) {
      console.error("Failed to fetch DB sizes:", error);
    }
  }

  async function fetchKeys(isInitial = true) {
    const config = $activeConfig;
    if (!config) return;
    try {
      isScanning = true;
      if (isInitial) {
        currentCursor = 0;
        keysList = [];
      }

      console.log(
        `[SCAN] Before: cursor=${currentCursor}, keysList.length=${keysList.length}`
      );

      console.log(
        `[DEBUG] Calling list_keys with currentCount=${keysList.length}`
      );

      const results = await invoke("list_keys", {
        config,
        db: selectedDb,
        cursor: currentCursor,
        pattern: activePattern,
        currentCount: keysList.length,
      });

      const nextCursor = results[0];
      const newKeys = results[1];

      console.log(
        `[SCAN] After: nextCursor=${nextCursor}, newKeys.length=${newKeys?.length || 0}`
      );

      if (newKeys && newKeys.length > 0) {
        keysList = [...keysList, ...newKeys];
      }

      currentCursor = nextCursor;

      // Save all keys when fully loaded (no pattern)
      if (nextCursor === 0 && !activePattern) {
        allKeys = [...keysList];
        console.log(`[SCAN] All keys loaded: ${allKeys.length}`);
      }

      console.log(
        `[SCAN] Updated: currentCursor=${currentCursor}, keysList.length=${keysList.length}`
      );

      // Auto-fetch when pattern search and not enough keys yet
      const hasPattern = activePattern && activePattern !== "*";
      if (hasPattern && nextCursor !== 0 && keysList.length < 500) {
        console.log(
          `[SCAN] Pattern search: auto-fetching more (${keysList.length}/1000)`
        );
        setTimeout(() => fetchKeys(false), 50);
        return; // Don't set isScanning = false yet
      }

      isScanning = false;
    } catch (error) {
      console.error("Failed to fetch keys:", error);
      isScanning = false;
    }
  }

  function scanMore() {
    fetchKeys(false);
  }

  // Search & Filters
  function executeSearch() {
    let query = searchInput.trim();

    // If all keys loaded, filter on frontend
    if (allKeys.length > 0 && query) {
      console.log(`[SEARCH] Filtering ${allKeys.length} keys on frontend`);

      // Normalize pattern for JS regex
      let pattern = query;
      if (!pattern.includes("*") && !pattern.includes("?")) {
        pattern = `${pattern}*`;
      }

      // Convert Redis pattern to regex
      const regexPattern = pattern.replace(/\*/g, ".*").replace(/\?/g, ".");
      const regex = new RegExp(`^${regexPattern}$`, "i");

      // Filter allKeys
      keysList = allKeys.filter((key) => regex.test(key.name));
      activePattern = query;
      searchInput = pattern;

      console.log(`[SEARCH] Found ${keysList.length} matching keys`);
      return;
    }

    // Otherwise, send pattern to backend for scan
    // Chỉ thêm * cuối nếu chưa có wildcard
    if (query && !query.includes("*") && !query.includes("?")) {
      query = `${query}*`;
    }
    searchInput = query;
    keysList = [];
    allKeys = []; // Clear allKeys when starting new backend scan
    activePattern = query;
    fetchKeys(true);
  }

  /** @param {KeyboardEvent} event */
  function handleSearch(event) {
    if (event.key === "Enter") {
      executeSearch();
    }
  }

  /** @param {any} event */
  function handleSearchInput(event) {
    searchInput = event.target.value;
  }

  // Key Value Handling (with Caching & Prefetching)
  /** @param {string} type */
  function handleTypeSelect(type) {
    console.log(`Selected key type: ${type}`);
    isAddKeyDropdownOpen = false;
  }

  // Flatten tree to get a list of paths in display order for Shift-selection
  function getFlattenedNodes(nodes, parentPath = "") {
    let flat = [];
    nodes.forEach((node) => {
      const currentPath = parentPath ? `${parentPath}:${node.name}` : node.name;

      if (node.type === "key") {
        flat.push(node.fullPath);
      } else if (node.type === "folder") {
        // Even if folder isn't "selectable" as a key, it's a row in the tree
        // But for Redis keys selection, we usually only care about keys.
        if (expandedFolders.has(currentPath) && node.children) {
          flat = [...flat, ...getFlattenedNodes(node.children, currentPath)];
        }
      }
    });
    return flat;
  }

  /**
   * @param {string} key
   * @param {MouseEvent|null} event
   */
  async function selectKey(key, event = null) {
    console.log(
      `[selectKey] >>> CLICK DETECTED for key: ${key}, button: ${event?.button}, ctrl: ${event?.ctrlKey}, meta: ${event?.metaKey}, shift: ${event?.shiftKey}`
    );

    // Handle Right Click (button 2)
    if (event && event.button === 2) {
      const alreadySelected = selectedKeys.some((k) => k === key);
      if (alreadySelected) {
        console.log(
          `[selectKey] Right-click on ALREADY SELECTED item: ${key}. Keeping selection.`
        );
        lastSelectedKey = key;
      } else {
        console.log(
          `[selectKey] Right-click on UNSELECTED item: ${key}. Selecting single.`
        );
        selectedKeys = [key];
        lastSelectedKey = key;
      }
    } else {
      // Handle Multi-select (Left Click)
      if (event && (event.ctrlKey || event.metaKey)) {
        if (selectedKeys.includes(key)) {
          selectedKeys = selectedKeys.filter((k) => k !== key);
          console.log(
            `[selectKey] Removed key: ${key}. New selectedKeys: ${selectedKeys.length}`
          );
        } else {
          selectedKeys = [...selectedKeys, key];
          console.log(
            `[selectKey] Added key: ${key}. New selectedKeys: ${selectedKeys.length}`
          );
        }
      } else if (event && event.shiftKey && lastSelectedKey) {
        // Real Range selection
        console.log(
          `[selectKey] Shift-click detected. Last selected: ${lastSelectedKey}`
        );
        const flatKeys = getFlattenedNodes(keyTree);
        const startIdx = flatKeys.indexOf(lastSelectedKey);
        const endIdx = flatKeys.indexOf(key);

        if (startIdx !== -1 && endIdx !== -1) {
          const [min, max] =
            startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
          const range = flatKeys.slice(min, max + 1);

          selectedKeys = range;
          console.log(
            `[selectKey] Range selected. New selectedKeys: ${selectedKeys.length}`
          );
        } else {
          selectedKeys = [key];
          console.log(
            `[selectKey] Shift-click, but range invalid. Selected single key: ${key}`
          );
        }
      } else {
        selectedKeys = [key];
        console.log(`[selectKey] Single key selected: ${key}`);
      }
      lastSelectedKey = key;
    }

    // Load value if exactly one key is selected
    if (selectedKeys.length === 1) {
      const targetKey = selectedKeys[0];
      if (selectedKey === targetKey) return;
      selectedKey = targetKey;
      console.log(
        `[selectKey] Loading value for single selected key: ${selectedKey}`
      );
    } else {
      selectedKey = "";
      // Reset keyValue to avoid showing old data
      keyValue = {
        key_type: null,
        value: null,
        ttl: -1,
        memory: 0,
        encoding: "",
      };
      console.log(
        `[selectKey] Multiple keys selected or no key. Resetting value view.`
      );
      return;
    }

    // Performance: Clear editor immediately before any async work
    if (editorNode) editorNode.innerHTML = "";
    originalContent = "";
    editableContent = "";

    selectedKey = key;
    const cacheKey = `${selectedDb}:${key}`;

    if (valueCache.has(cacheKey)) {
      const entry = valueCache.get(cacheKey);
      const now = Date.now();
      const elapsed = Math.floor((now - entry.fetchedAt) / 1000);

      keyValue = {
        ...entry,
        ttl: entry.ttl > 0 ? Math.max(0, entry.ttl - elapsed) : entry.ttl,
      };
      console.log(`[selectKey] Value loaded from cache for ${key}`);
    } else {
      // Clear current value to show loading state
      keyValue = {
        key_type: null,
        value: null,
        ttl: -1,
        memory: 0,
        encoding: "",
      };

      // Only show loading if it takes more than 100ms
      clearTimeout(valueLoadingTimeout);
      valueLoadingTimeout = setTimeout(() => {
        isLoadingValue = true;
      }, 100);
      console.log(`[selectKey] Fetching value for ${key}...`);

      try {
        const config = $activeConfig;
        if (!config) return;

        const result = await invoke("get_key_value", {
          config,
          key,
          db: selectedDb,
        });

        const cacheEntry = { ...result, fetchedAt: Date.now() };
        valueCache.set(cacheKey, cacheEntry);

        // Only update if the selected key hasn't changed during the async operation
        if (selectedKey === key) {
          keyValue = {
            ...result,
            ttl: result.ttl,
          };
          console.log(`[selectKey] Value fetched and updated for ${key}`);
        } else {
          console.log(
            `[selectKey] Value fetched for ${key}, but selectedKey changed to ${selectedKey}. Discarding.`
          );
        }
      } catch (error) {
        console.error("Failed to get key value:", error);
      } finally {
        if (selectedKey === key) {
          clearTimeout(valueLoadingTimeout);
          isLoadingValue = false;
        }
      }
    }
  }

  // UI Actions

  $effect(() => {
    if (keyValue.value !== null) {
      const formatted = formatKeyValue(keyValue.value);
      originalContent = formatted;
      editableContent = formatted;

      // Update editor DOM manually to maintain line wrappers for CSS counters
      if (editorNode) {
        // Optimized: Only update if content actually changed (avoid re-renders)
        // Guard: Use requestAnimationFrame to prevent blocking the UI thread on large content
        requestAnimationFrame(() => {
          if (!editorNode) return;

          editorNode.setAttribute("contenteditable", "true");
          editorNode.innerHTML = formatted
            .split("\n")
            .map((line) => `<div>${line || "<br>"}</div>`)
            .join("");
        });
      }
    }
  });

  $effect(() => {
    let interval;
    if (keyValue.ttl > 0) {
      interval = setInterval(() => {
        if (keyValue.ttl > 0) {
          keyValue.ttl -= 1;
        } else {
          clearInterval(interval);
        }
      }, 1000);
    }
    return () => clearInterval(interval);
  });

  async function saveValue() {
    if (!isModified) return;

    try {
      const config = $activeConfig;
      if (!config) return;

      isLoadingValue = true;
      await invoke("set_key_value", {
        config,
        key: selectedKey,
        db: selectedDb,
        value: editableContent,
      });

      // Update cache and original state
      if (keyValue.value) {
        const updatedValue = {
          ...keyValue,
          value: { ...keyValue.value, value: editableContent },
        };
        keyValue = updatedValue;
        originalContent = editableContent;
        valueCache.set(`${selectedDb}:${selectedKey}`, updatedValue);
      }

      console.log("Value saved successfully");
    } catch (error) {
      console.error("Failed to save value:", error);
      alert("Failed to save value: " + error);
    } finally {
      isLoadingValue = false;
    }
  }

  // Context Menu Actions
  function showKeyContextMenu(event, key) {
    event.preventDefault();
    // If key not in selection, select only this key
    const alreadySelected = selectedKeys.some((k) => k === key);
    if (!alreadySelected) {
      console.log(
        `[showKeyContextMenu] Key ${key} not in selection. Resetting to single.`
      );
      selectedKeys = [key];
      selectedKey = key;
    }
    contextMenuKey = key;
    contextMenuPosition = { x: event.clientX, y: event.clientY };
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
  }

  async function copySelectedKeyNames(single = false) {
    let names = "";
    if (single && contextMenuKey) {
      names = contextMenuKey;
    } else {
      names = Array.from(selectedKeys).join("\n");
    }

    if (names) {
      try {
        await navigator.clipboard.writeText(names);
        console.log(
          `[ContextMenu] Copied ${single ? "single" : selectedKeys.length} key(s)`
        );
      } catch (err) {
        console.error("Clipboard error:", err);
      }
    }
    closeContextMenu();
  }

  async function copyKeyAsJSON() {
    const key = contextMenuKey || selectedKey;
    if (!key) {
      closeContextMenu();
      return;
    }

    try {
      const config = $activeConfig;
      if (!config) return;

      // Get key value
      const result = await invoke("get_key_value", {
        config,
        db: selectedDb,
        key,
      });

      // Build JSON object
      const jsonObject = { [key]: result.value };
      const jsonString = JSON.stringify(jsonObject, null, 2);

      await navigator.clipboard.writeText(jsonString);
      console.log(
        "[ContextMenu] Copied key as JSON:",
        jsonString.substring(0, 200)
      );
    } catch (err) {
      console.error("Failed to copy as JSON:", err);
      alert("Failed to copy as JSON: " + err);
    }
    closeContextMenu();
  }

  function confirmDeleteSelectedKeys() {
    const count = selectedKeys.length;
    const message =
      count === 1
        ? `Delete key "${selectedKeys[0]}"?`
        : `Delete ${count} selected keys?`;
    showConfirm(message, async () => {
      try {
        isLoadingValue = true;
        const keysToDelete = [...selectedKeys]; // Use spread to ensure a new array for the Set conversion
        const config = $activeConfig;
        if (!config) return;

        await invoke("delete_keys", {
          config,
          db: selectedDb,
          keys: keysToDelete,
        });

        // Update UI
        const toDeleteSet = new Set(keysToDelete);
        keysList = keysList.filter((k) => !toDeleteSet.has(k.name));
        allKeys = allKeys.filter((k) => !toDeleteSet.has(k.name));
        selectedKeys = [];
        selectedKey = "";

        // Trigger tree rebuild
        keysList = [...keysList];
      } catch (error) {
        console.error("Failed to delete keys:", error);
        alert("Failed to delete keys: " + error);
      } finally {
        isLoadingValue = false;
      }
    });
    closeContextMenu();
  }

  // Confirm dialog helpers (mirrored from HashEditor)
  function showConfirm(message, callback) {
    confirmDialogMessage = message;
    confirmDialogCallback = callback;
    showConfirmDialog = true;
  }

  function handleConfirmYes() {
    showConfirmDialog = false;
    if (confirmDialogCallback) confirmDialogCallback();
  }

  function handleConfirmNo() {
    showConfirmDialog = false;
  }

  // TTL Dialog helpers
  function openTTLDialog(key) {
    ttlTargetKey = key;
    ttlValue = 0;
    ttlUnit = "seconds";
    showTTLDialog = true;
    closeContextMenu();
  }

  function closeTTLDialog() {
    showTTLDialog = false;
    ttlTargetKey = "";
  }

  async function saveTTL() {
    if (!ttlTargetKey || ttlValue <= 0) {
      closeTTLDialog();
      return;
    }

    // Convert to seconds
    let seconds = ttlValue;
    switch (ttlUnit) {
      case "minutes":
        seconds = ttlValue * 60;
        break;
      case "hours":
        seconds = ttlValue * 3600;
        break;
      case "days":
        seconds = ttlValue * 86400;
        break;
      case "weeks":
        seconds = ttlValue * 604800;
        break;
      case "months":
        seconds = ttlValue * 2592000;
        break; // 30 days
      case "years":
        seconds = ttlValue * 31536000;
        break; // 365 days
    }

    try {
      const config = $activeConfig;
      if (!config) return;

      await invoke("set_key_ttl", {
        config,
        db: selectedDb,
        key: ttlTargetKey,
        ttl: seconds,
      });

      console.log(`[TTL] Set TTL for ${ttlTargetKey} to ${seconds} seconds`);

      // Refresh key value if it's the selected key
      if (ttlTargetKey === selectedKey) {
        await selectKey(selectedKey);
      }
    } catch (error) {
      console.error("Failed to set TTL:", error);
      alert("Failed to set TTL: " + error);
    } finally {
      closeTTLDialog();
    }
  }

  /** @param {any} event */
  function handleInput(event) {
    // Extract plain text while maintaining logical lines (divs)
    // textContent is faster and safer than innerText but doesn't preserve line breaks from divs well.
    // However, innerText is the one that forces reflow.
    // For large buffers, we might need a more optimized way.
    editableContent = event.target.innerText.replace(/\n$/, "");
  }

  // Formatting helpers for metadata
  /** @param {number} ttl */
  function formatTTL(ttl) {
    if (ttl === -1) return "Persistent";
    if (ttl === -2 || ttl <= 0) return "Expired";

    const days = Math.floor(ttl / 86400);
    const hours = Math.floor((ttl % 86400) / 3600);
    const minutes = Math.floor((ttl % 3600) / 60);
    const seconds = ttl % 60;

    if (ttl >= 86400) {
      return `${days}d ${hours}h ${seconds}s`;
    }
    if (ttl >= 3600) {
      return `${hours}h ${minutes}m ${seconds}s`;
    }
    return `${minutes}m ${seconds}s`;
  }

  /** @param {number} bytes */
  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }

  async function disconnect() {
    activeConfig.set(null);
    keysList = [];
    selectedKey = "";
    keyValue = {
      key_type: null,
      value: null,
      ttl: -1,
      memory: 0,
      encoding: "",
    };
    valueCache.clear();
    clearTimeout(valueLoadingTimeout);
    isLoadingValue = false;
    await resizeWindow(800, 600);
    goto("/login");
  }

  /** @param {number} db */
  async function changeDb(db) {
    if (selectedDb === db) return;
    selectedDb = db;
    isDropdownOpen = false;
    keysList = [];
    allKeys = []; // Reset allKeys to avoid stale data
    selectedKey = "";
    keyValue = {
      key_type: null,
      value: null,
      ttl: -1,
      memory: 0,
      encoding: "",
    };
    valueCache.clear();
    clearTimeout(valueLoadingTimeout);
    isLoadingValue = false;
    await fetchKeys();
  }

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
  }

  function addKey() {
    isAddKeyDropdownOpen = !isAddKeyDropdownOpen;
  }

  /** @param {string} folderPath */
  function toggleFolder(folderPath) {
    console.log(`[toggleFolder] >>> FOLDER CLICK: ${folderPath}`);
    if (expandedFolders.has(folderPath)) {
      expandedFolders.delete(folderPath);
    } else {
      expandedFolders.add(folderPath);
    }
    expandedFolders = new Set(expandedFolders);
    console.log(
      `[toggleFolder] New state for ${folderPath}: ${expandedFolders.has(folderPath)}`
    );
  }

  /** @param {Event} event */
  function preventPropagation(event) {
    event.stopPropagation();
  }

  /** @param {MouseEvent} event */
  function handleClickOutside(event) {
    const dbDropdown = document.querySelector(".db-dropdown");
    const addDropdown = document.querySelector(".add-key-container");
    if (dbDropdown && !dbDropdown.contains(event.target))
      isDropdownOpen = false;
    if (addDropdown && !addDropdown.contains(event.target))
      isAddKeyDropdownOpen = false;

    // Close context menu on click outside
    showContextMenu = false;
  }

  onMount(() => {
    window.addEventListener("click", handleClickOutside);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", stopResizing);
    return () => {
      window.removeEventListener("click", handleClickOutside);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", stopResizing);
    };
  });

  // Derived State & Effects
  let filteredKeys = $derived(keysList);
  let treeResult = $derived(buildTree(filteredKeys, ":"));
  let keyTree = $derived(treeResult.tree);
  let contentLines = $derived(
    keyValue.value !== null ? formatKeyValue(keyValue.value).split("\n") : []
  );

  $effect(() => {
    if (activePattern.trim() && filteredKeys.length > 0) {
      const timeout = setTimeout(() => {
        treeResult.pathsToExpand.forEach((path) => {
          expandedFolders.add(path);
        });
        expandedFolders = new Set(expandedFolders);
      }, 100);
      return () => clearTimeout(timeout);
    }
  });
</script>

{#snippet renderTree(nodes, depth = 0, parentPath = "")}
  {#each nodes as node}
    {@const currentPath = parentPath ? `${parentPath}:${node.name}` : node.name}
    <div class="tree-node" style="padding-left: {depth * 12}px">
      {#if node.type === "folder"}
        <button
          class="tree-item folder"
          onmousedown={(e) => {
            console.log("[snippet] Folder mousedown:", currentPath);
            toggleFolder(currentPath);
          }}
        >
          <span class="tree-tag tag-dir">DIR</span>
          <span class="name">{node.name}</span>
          <span class="count-badge">
            ({node.keyCount}{currentCursor !== 0 ? "+" : ""})
          </span>
        </button>
        {#if expandedFolders.has(currentPath)}
          {@render renderTree(node.children, depth + 1, currentPath)}
        {/if}
      {:else}
        <button
          class="tree-item key"
          class:selected={selectedKeys.includes(node.fullPath)}
          onmousedown={(e) => {
            if (e.button === 1) return; // Ignore middle click
            console.log(
              "[snippet] Item mousedown:",
              node.fullPath,
              "button:",
              e.button
            );
            e.stopPropagation();
            selectKey(node.fullPath, e);
          }}
          oncontextmenu={(e) => {
            console.log("[snippet] ContextMenu triggered:", node.fullPath);
            e.stopPropagation();
            showKeyContextMenu(e, node.fullPath);
          }}
        >
          <span class="tree-tag tag-{(node.key_type || 'key').toLowerCase()}"
            >{(node.key_type || "KEY").toUpperCase()}</span
          >
          <span class="name">{node.name}</span>
        </button>
      {/if}
    </div>
  {/each}
{/snippet}

<div class="layout">
  <aside class="sidebar sidebar-explorer" style="width: {sidebarWidth}px">
    <div class="sidebar-header">
      <!-- <div class="header-top">
        <h3>Keys ({filteredKeys.length})</h3>
      </div> -->
      <div class="footer-info">
        <span>{filteredKeys.length} keys</span>
        {#if selectedKeys.length > 0}
          <span class="selection-count">| {selectedKeys.length} selected</span>
        {/if}
      </div>
      <div class="search-row">
        <div class="search-box">
          <input
            type="text"
            placeholder="Search keys... (Enter to search)"
            bind:value={searchInput}
            onkeydown={handleSearch}
          />
          {#if searchInput}
            <button
              class="btn-clear-search"
              onclick={() => {
                searchInput = "";
                // Restore all keys if they were loaded
                if (allKeys.length > 0) {
                  keysList = [...allKeys];
                  activePattern = "";
                } else {
                  executeSearch();
                }
              }}
              title="Clear search"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          {/if}
        </div>
        <button
          class="btn-action refresh"
          onclick={executeSearch}
          title="Refresh/Search"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.0"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
            <polyline points="21 3 21 8 16 8" />
          </svg>
        </button>
        <div class="add-key-container">
          <button
            class="btn-action add"
            onclick={addKey}
            title="Add New Key"
            class:active={isAddKeyDropdownOpen}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
          {#if isAddKeyDropdownOpen}
            <div class="add-key-dropdown">
              {#each redisKeyTypes as type}
                <button
                  class="dropdown-item"
                  onclick={() => handleTypeSelect(type.toLowerCase())}
                >
                  <span class="type-icon tag-{type.toLowerCase()}">
                    {type[0]}
                  </span>
                  <span class="type-label">{type}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <div
      class="keys-list"
      class:scanning={isScanning}
      style="overflow-y: auto;"
    >
      {@render renderTree(keyTree)}
    </div>

    <div class="sidebar-footer">
      {#if currentCursor !== 0}
        <button class="btn-scan-more" onclick={scanMore} disabled={isScanning}>
          {#if isScanning}
            <span class="spinner"></span>
          {/if}
          {isScanning ? "Scanning..." : "Scan More"}
        </button>
      {/if}

      <div class="footer-bottom-row">
        <button class="btn-disconnect" onclick={disconnect} title="Disconnect">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
            <polyline points="16 17 21 12 16 7" />
            <line x1="21" y1="12" x2="9" y2="12" />
          </svg>
        </button>

        <div class="db-dropdown">
          <button class="dropdown-trigger" onclick={toggleDropdown}>
            DB {selectedDb} ({dbSizes[selectedDb] ?? 0})
            <span class="arrow" class:open={isDropdownOpen}>▼</span>
          </button>
          {#if isDropdownOpen}
            <div class="dropdown-menu">
              {#each dbOptions as db}
                <button
                  class="dropdown-item"
                  class:active={selectedDb === db}
                  onclick={() => changeDb(db)}
                >
                  <span class="db-label">DB {db}</span>
                  <span class="db-count">{dbSizes[db] ?? 0}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </aside>

  <button
    class="resizer"
    class:active={isResizing}
    onmousedown={startResizing}
    type="button"
    aria-label="Resize Sidebar"
  ></button>

  <main class="value-panel">
    {#if selectedKey}
      <div class="value-header">
        {#if selectedKeyType}
          <span class="type-badge tag-{selectedKeyType.toLowerCase()}"
            >{selectedKeyType.toUpperCase()}</span
          >
        {/if}
        <h2>{selectedKey}</h2>
        {#if isModified}
          <button
            class="save-action"
            onclick={saveValue}
            disabled={isLoadingValue}
          >
            <i class="codicon codicon-save"></i>
            {isLoadingValue ? "Saving..." : "Save Changes"}
          </button>
        {/if}
      </div>
      <div class="metadata-bar">
        <div class="meta-item">
          <span class="label">TTL:</span>
          <span class="value" class:persistent={keyValue.ttl === -1}
            >{formatTTL(keyValue.ttl)}</span
          >
        </div>
        <div class="meta-item">
          <span class="label">Memory:</span>
          <span class="value">{formatBytes(keyValue.memory)}</span>
        </div>
        <div class="meta-item">
          <span class="label">Encoding:</span>
          <span class="value">{keyValue.encoding}</span>
        </div>
      </div>
      {#if keyValue.key_type === "hash"}
        <HashEditor
          {selectedKey}
          {selectedDb}
          on:refresh={() => selectKey(selectedKey)}
        />
      {:else}
        <div class="value-content" use:simplebar data-simplebar>
          {#if keyValue?.value !== null}
            <div
              bind:this={editorNode}
              class="editor-container"
              contenteditable="true"
              oninput={handleInput}
              spellcheck="false"
            ></div>
          {:else}
            <div class="placeholder-text"></div>
          {/if}
        </div>
      {/if}
    {:else}
      <div class="placeholder-text">
        {selectedKeys.length > 1
          ? `${selectedKeys.length} keys selected. Right-click for actions.`
          : "Select a key from the sidebar to view its value"}
      </div>
    {/if}
  </main>
</div>

<!-- Context Menu: Single Key -->
{#if showContextMenu && selectedKeys.length === 1}
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="context-menu"
      style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="context-menu-item" onclick={() => copySelectedKeyNames(true)}>
        <i class="codicon codicon-copy"></i>
        <span>Copy Key Name</span>
      </div>
      <div class="context-menu-item" onclick={copyKeyAsJSON}>
        <i class="codicon codicon-json"></i>
        <span>Copy as JSON</span>
      </div>
      <div
        class="context-menu-item"
        onclick={() => openTTLDialog(contextMenuKey)}
      >
        <i class="codicon codicon-clock"></i>
        <span>Set TTL</span>
      </div>
      <div class="context-menu-separator"></div>
      <div
        class="context-menu-item danger"
        onclick={() => {
          confirmDeleteSelectedKeys();
        }}
      >
        <i class="codicon codicon-trash"></i>
        <span>Delete Key</span>
      </div>
    </div>
  </div>
{/if}

<!-- Context Menu: Multiple Keys -->
{#if showContextMenu && selectedKeys.length > 1}
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="context-menu"
      style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="context-menu-group-label">
        {selectedKeys.length} Keys Selected
      </div>
      <div
        class="context-menu-item"
        onclick={() => copySelectedKeyNames(false)}
      >
        <i class="codicon codicon-copy"></i>
        <span>Copy All Names</span>
      </div>
      <div class="context-menu-separator"></div>
      <div class="context-menu-item danger" onclick={confirmDeleteSelectedKeys}>
        <i class="codicon codicon-trash"></i>
        <span>Delete All Keys</span>
      </div>
    </div>
  </div>
{/if}

<!-- Confirm Dialog -->
{#if showConfirmDialog}
  <div class="confirm-dialog-overlay" onclick={handleConfirmNo}>
    <div class="confirm-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="confirm-dialog-message">{confirmDialogMessage}</div>
      <div class="confirm-dialog-buttons">
        <button class="btn-confirm-no" onclick={handleConfirmNo}>Cancel</button>
        <button class="btn-confirm-yes" onclick={handleConfirmYes}
          >Delete</button
        >
      </div>
    </div>
  </div>
{/if}

<!-- TTL Dialog -->
{#if showTTLDialog}
  <div class="ttl-dialog-overlay" onclick={closeTTLDialog}>
    <div class="ttl-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="ttl-dialog-header">
        <h3>Set TTL</h3>
        <span class="ttl-dialog-key">{ttlTargetKey}</span>
      </div>
      <div class="ttl-dialog-body">
        <div class="ttl-input-group">
          <input
            type="number"
            min="1"
            bind:value={ttlValue}
            placeholder="Enter value"
            class="ttl-input"
          />
          <select bind:value={ttlUnit} class="ttl-select">
            <option value="seconds">Seconds</option>
            <option value="minutes">Minutes</option>
            <option value="hours">Hours</option>
            <option value="days">Days</option>
            <option value="weeks">Weeks</option>
            <option value="months">Months</option>
            <option value="years">Years</option>
          </select>
        </div>
      </div>
      <div class="ttl-dialog-footer">
        <button class="btn-ttl-cancel" onclick={closeTTLDialog}>Cancel</button>
        <button class="btn-ttl-save" onclick={saveTTL}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  @import "../../styles/explorer.scss";

  /* Context Menu Styles */
  .context-menu-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 10000;
    background: transparent;
  }

  .context-menu {
    position: fixed;
    background: #252526;
    border: 1px solid #454545;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    border-radius: 4px;
    padding: 4px 0;
    min-width: 180px;
    z-index: 10001;
    animation: contextMenuFade 0.1s ease-out;
  }

  .context-menu-item {
    padding: 8px 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    font-size: 13px;
    color: #cccccc;

    &:hover {
      background: #094771;
      color: #ffffff;
    }

    &.danger {
      color: #ff5555;
      &:hover {
        background: #902727;
      }
    }
  }

  .context-menu-group-label {
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 700;
    color: #666666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-menu-separator {
    height: 1px;
    background: #454545;
    margin: 4px 0;
  }

  /* Confirm Dialog Styles */
  .confirm-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 20000;
    backdrop-filter: blur(2px);
  }

  .confirm-dialog {
    background: #252526;
    border: 1px solid #454545;
    border-radius: 8px;
    padding: 1.5rem;
    min-width: 320px;
    max-width: 450px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    animation: dialogScale 0.2s cubic-bezier(0.17, 0.67, 0.83, 0.67);
  }

  .confirm-dialog-message {
    color: #cccccc;
    font-size: 0.95rem;
    margin-bottom: 2rem;
    line-height: 1.5;
    text-align: center;
    word-break: break-all;
  }

  .confirm-dialog-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .btn-confirm-no,
  .btn-confirm-yes {
    padding: 0.5rem 1.25rem;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-confirm-no {
    background: #3c3c3c;
    color: #cccccc;
    border: 1px solid #454545;

    &:hover {
      background: #4c4c4c;
    }
  }

  .btn-confirm-yes {
    background: #e53935;
    color: white;
    border: 1px solid #b71c1c;

    &:hover {
      background: #f44336;
    }
  }

  @keyframes contextMenuFade {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes dialogScale {
    from {
      opacity: 0;
      transform: scale(0.9);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  /* TTL Dialog Styles */
  .ttl-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10002;
  }

  .ttl-dialog {
    background: #252526;
    border: 1px solid #454545;
    border-radius: 8px;
    min-width: 320px;
    max-width: 400px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    animation: dialogScale 0.2s ease-out;
  }

  .ttl-dialog-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #333;

    h3 {
      margin: 0 0 0.25rem 0;
      font-size: 1rem;
      font-weight: 600;
      color: #ffffff;
    }

    .ttl-dialog-key {
      font-size: 0.8rem;
      color: #888;
      font-family: monospace;
      word-break: break-all;
    }
  }

  .ttl-dialog-body {
    padding: 1.25rem;
  }

  .ttl-input-group {
    display: flex;
    gap: 0.5rem;
  }

  .ttl-input {
    flex: 1;
    padding: 0.6rem 0.75rem;
    border: 1px solid #454545;
    border-radius: 4px;
    background: #1e1e1e;
    color: #cccccc;
    font-size: 0.9rem;

    &:focus {
      outline: none;
      border-color: #007acc;
    }
  }

  .ttl-select {
    padding: 0.6rem 0.75rem;
    border: 1px solid #454545;
    border-radius: 4px;
    background: #1e1e1e;
    color: #cccccc;
    font-size: 0.9rem;
    cursor: pointer;

    &:focus {
      outline: none;
      border-color: #007acc;
    }
  }

  .ttl-dialog-footer {
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #333;
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .btn-ttl-cancel,
  .btn-ttl-save {
    padding: 0.5rem 1.25rem;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ttl-cancel {
    background: #3c3c3c;
    color: #cccccc;
    border: 1px solid #454545;

    &:hover {
      background: #4c4c4c;
    }
  }

  .btn-ttl-save {
    background: #0e639c;
    color: white;
    border: 1px solid #0d5689;

    &:hover {
      background: #1177bb;
    }
  }
</style>
