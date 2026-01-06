<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig } from "$lib/stores.js";
  import { resizeWindow, formatKeyValue, buildTree } from "$lib/utils.js";
  import SimpleBar from "simplebar";
  import "simplebar/dist/simplebar.css";

  // State Management
  /** @type {any[]} */
  let keysList = $state([]);
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

  let scanBuffer = [];

  async function fetchKeys(isInitial = true) {
    const config = $activeConfig;
    if (!config) return;
    try {
      isScanning = true;
      if (isInitial) {
        currentCursor = 0;
        keysList = [];
        scanBuffer = [];
      }

      const results = await invoke("list_keys", {
        config,
        db: selectedDb,
        cursor: currentCursor,
        pattern: activePattern,
      });

      const nextCursor = results[0];
      const newKeys = results[1];

      if (newKeys && newKeys.length > 0) {
        scanBuffer.push(...newKeys);
      }

      currentCursor = nextCursor;

      if (nextCursor !== 0) {
        // Update UI occasionally if buffer is large enough
        if (scanBuffer.length >= 20000) {
          keysList = [...keysList, ...scanBuffer];
          scanBuffer = [];
        }
        // Yield to the main thread before continuing
        setTimeout(() => fetchKeys(false), 50);
      } else {
        // Final update
        keysList = [...keysList, ...scanBuffer];
        scanBuffer = [];
        isScanning = false;
      }
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
    if (query && !query.includes("*")) {
      query = `*${query}*`;
    }
    searchInput = query;
    keysList = [];
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

  /** @param {string} key */
  async function selectKey(key) {
    if (selectedKey === key) return;

    // Performance: Clear editor immediately before any async work
    if (editorNode) editorNode.innerHTML = "";
    originalContent = "";
    editableContent = "";

    selectedKey = key;
    const cacheKey = `${selectedDb}:${key}`;

    if (valueCache.has(cacheKey)) {
      keyValue = valueCache.get(cacheKey);
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

      try {
        const config = $activeConfig;
        if (!config) return;

        const result = await invoke("get_key_value", {
          config,
          key,
          db: selectedDb,
        });
        valueCache.set(cacheKey, result);
        // Only update if the selected key hasn't changed during the async operation
        if (selectedKey === key) {
          keyValue = result;
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

          // Guard: If value is too large, it can crash the browser tab in contenteditable
          const MAX_EDITABLE_SIZE = 100 * 1024; // 100KB
          if (formatted.length > MAX_EDITABLE_SIZE) {
            editorNode.setAttribute("contenteditable", "false");
            editorNode.innerHTML =
              `<div class="warning-text">// Nội dung quá lớn (${formatBytes(formatted.length)}), chế độ chỉnh sửa đã bị tắt để đảm bảo hiệu năng.</div>` +
              formatted
                .slice(0, MAX_EDITABLE_SIZE)
                .split("\n")
                .map((line) => `<div>${line || "<br>"}</div>`)
                .join("") +
              `<div class="warning-text">... [Đã cắt bớt nội dung]</div>`;
          } else {
            editorNode.setAttribute("contenteditable", "true");
            editorNode.innerHTML = formatted
              .split("\n")
              .map((line) => `<div>${line || "<br>"}</div>`)
              .join("");
          }
        });
      }
    }
  });

  /** @param {any} event */
  function handleInput(event) {
    // Extract plain text while maintaining logical lines (divs)
    // textContent is faster and safer than innerText but doesn't preserve line breaks from divs well.
    // However, innerText is the one that forces reflow.
    // For large buffers, we might need a more optimized way.
    editableContent = event.target.innerText.replace(/\n$/, "");
  }

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

  // Formatting helpers for metadata
  /** @param {number} ttl */
  function formatTTL(ttl) {
    if (ttl === -1) return "Persistent";
    if (ttl === -2) return "Expired";
    if (ttl < 60) return `${ttl}s`;
    if (ttl < 3600) return `${Math.floor(ttl / 60)}m ${ttl % 60}s`;
    return `${Math.floor(ttl / 3600)}h ${Math.floor((ttl % 3600) / 60)}m`;
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
    if (expandedFolders.has(folderPath)) {
      expandedFolders.delete(folderPath);
    } else {
      expandedFolders.add(folderPath);
    }
    expandedFolders = new Set(expandedFolders);
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
        expandedFolders = new Set([
          ...expandedFolders,
          ...treeResult.pathsToExpand,
        ]);
      }, 100);
      return () => clearTimeout(timeout);
    }
  });
</script>

{#snippet renderTree(nodes, depth = 0, parentPath = "")}
  {@const visibleNodes = nodes.slice(0, 200)}
  {#each visibleNodes as node}
    {@const currentPath = parentPath ? `${parentPath}:${node.name}` : node.name}
    <div class="tree-node" style="padding-left: {depth * 12}px">
      {#if node.type === "folder"}
        <button
          class="tree-item folder"
          onclick={() => toggleFolder(currentPath)}
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
          class:selected={selectedKey === node.fullPath}
          onclick={() => selectKey(node.fullPath)}
        >
          <span class="tree-tag tag-{(node.key_type || 'key').toLowerCase()}"
            >{(node.key_type || "KEY").toUpperCase()}</span
          >
          <span class="name">{node.name}</span>
        </button>
      {/if}
    </div>
  {/each}
  {#if nodes.length > 200}
    <div class="limit-info" style="padding-left: {depth * 12 + 20}px">
      Showing 200 of {nodes.length} items...
    </div>
  {/if}
{/snippet}

<div class="layout">
  <aside class="sidebar sidebar-explorer" style="width: {sidebarWidth}px">
    <div class="sidebar-header">
      <!-- <div class="header-top">
        <h3>Keys ({filteredKeys.length})</h3>
      </div> -->
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
                executeSearch();
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

    <div class="keys-list" class:scanning={isScanning} use:simplebar>
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
          <div class="placeholder-text">Loading...</div>
        {/if}
      </div>
    {:else}
      <div class="placeholder-text">
        Select a key from the sidebar to view its value
      </div>
    {/if}
  </main>
</div>

<style lang="scss">
  @import "./explorer.scss";
</style>
