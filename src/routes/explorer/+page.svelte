<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig } from "$lib/stores.js";
  import { resizeWindow, formatKeyValue, buildTree } from "$lib/utils.js";
  import SimpleBar from "simplebar";
  import "simplebar/dist/simplebar.css";

  let keysList = $state([]);
  let selectedKey = $state("");
  let keyValue = $state({ type: null, value: null });
  let searchInput = $state("");
  let activePattern = $state("");
  let selectedDb = $state(0);
  let dbSizes = $state([]);
  let isDropdownOpen = $state(false);
  let isAddKeyDropdownOpen = $state(false);
  let expandedFolders = $state(new Set());
  const redisKeyTypes = ["String", "Hash", "List", "Set", "ZSet", "Stream"];
  let currentCursor = $state(0);
  let isScanning = $state(true);

  // Svelte action for SimpleBar
  /** @param {HTMLElement} node */
  function simplebar(node, options = { autoHide: false }) {
    console.log("SimpleBar init on node:", node);
    const sb = new SimpleBar(node, options);

    // Auto-recalculate when content changes
    const observer = new MutationObserver(() => sb.recalculate());
    observer.observe(node, {
      childList: true,
      subtree: true,
      characterData: true,
    });

    return {
      destroy() {
        console.log("SimpleBar destroy");
        observer.disconnect();
        sb.unMount();
      },
    };
  }

  // Sidebar resizing
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

  async function fetchKeys(isInitial = true) {
    const config = $activeConfig;
    if (!config) return;
    try {
      isScanning = true;
      if (isInitial) {
        currentCursor = 0;
        keysList = [];
      }

      const [nextCursor, newKeys] = await invoke("list_keys", {
        config,
        db: selectedDb,
        cursor: currentCursor,
        pattern: activePattern,
      });

      keysList = [...keysList, ...newKeys];
      currentCursor = nextCursor;
    } catch (error) {
      console.error("Failed to fetch keys:", error);
    } finally {
      isScanning = false;
    }
  }

  function scanMore() {
    fetchKeys(false);
  }

  function executeSearch() {
    let query = searchInput.trim();
    if (query && !query.includes("*")) {
      query = `*${query}*`;
    }
    searchInput = query;

    // Reset immediately to avoid lag from processing old large list with new pattern
    keysList = [];
    activePattern = query;
    fetchKeys(true);
  }

  function handleSearch(event) {
    if (event.key === "Enter") {
      executeSearch();
    }
  }

  function addKey() {
    isAddKeyDropdownOpen = !isAddKeyDropdownOpen;
  }

  function handleTypeSelect(type) {
    console.log(`Selected key type: ${type}`);
    isAddKeyDropdownOpen = false;
    // Next steps would be to open a modal for key creation
  }

  async function selectKey(key) {
    selectedKey = key;
    keyValue = { type: null, value: null };
    const config = $activeConfig;
    if (!config) return;
    try {
      keyValue = await invoke("get_key_value", { config, key, db: selectedDb });
    } catch (error) {
      console.error("Failed to fetch key value:", error);
    }
  }

  async function disconnect() {
    activeConfig.set(null);
    keysList = [];
    selectedKey = "";
    keyValue = { type: null, value: null };
    await resizeWindow(800, 600);
    goto("/login");
  }

  async function changeDb(db) {
    selectedDb = db;
    isDropdownOpen = false;
    selectedKey = "";
    keyValue = { type: null, value: null };
    expandedFolders = new Set();
    await fetchKeys();
  }

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
  }

  function toggleFolder(folderPath) {
    if (expandedFolders.has(folderPath)) {
      expandedFolders.delete(folderPath);
    } else {
      expandedFolders.add(folderPath);
    }
    expandedFolders = new Set(expandedFolders); // Trigger reactivity
  }

  // Click outside to close dropdowns
  function handleClickOutside(event) {
    const dbDropdown = document.querySelector(".db-dropdown");
    const addDropdown = document.querySelector(".add-key-container");

    if (dbDropdown && !dbDropdown.contains(event.target)) {
      isDropdownOpen = false;
    }
    if (addDropdown && !addDropdown.contains(event.target)) {
      isAddKeyDropdownOpen = false;
    }
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

  let filteredKeys = $derived(keysList);

  let treeResult = $derived(buildTree(filteredKeys, ":"));
  let keyTree = $derived(treeResult.tree);

  $effect(() => {
    if (activePattern.trim() && filteredKeys.length > 0) {
      // Use a timeout to avoid blocking the main thread during heavy rendering
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
          <span class="tree-tag tag-key"
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
      <div class="header-top">
        <h3>Keys ({filteredKeys.length})</h3>
      </div>
      <div class="search-row">
        <div class="search-box">
          <input
            type="text"
            placeholder="Search keys... (Enter to search)"
            bind:value={searchInput}
            onkeydown={handleSearch}
          />
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

  <main class="value-panel" use:simplebar data-simplebar>
    {#if selectedKey}
      <div class="value-header">
        <h2>{selectedKey}</h2>
        {#if keyValue?.type}
          <span class="type-badge">{keyValue.type}</span>
        {/if}
      </div>
      <div class="value-content">
        {#if keyValue?.value !== null}
          <pre>{formatKeyValue(keyValue)}</pre>
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
