<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig } from "$lib/stores.js";
  import { resizeWindow, formatKeyValue } from "$lib/utils.js";

  let keysList = $state([]);
  let selectedKey = $state("");
  let keyValue = $state(null);
  let searchQuery = $state("");

  onMount(() => {
    if (!$activeConfig) {
      goto("/login");
    } else {
      fetchKeys();
    }
  });

  async function fetchKeys() {
    const config = $activeConfig;
    if (!config) return;
    try {
      keysList = await invoke("list_keys", { config });
    } catch (error) {
      console.error("Failed to fetch keys:", error);
    }
  }

  async function selectKey(key) {
    selectedKey = key;
    keyValue = null;
    const config = $activeConfig;
    if (!config) return;
    try {
      keyValue = await invoke("get_key_value", { config, key });
    } catch (error) {
      console.error("Failed to fetch key value:", error);
    }
  }

  async function disconnect() {
    activeConfig.set(null);
    keysList = [];
    selectedKey = "";
    keyValue = null;
    await resizeWindow(800, 600);
    goto("/login");
  }

  let filteredKeys = $derived(
    keysList.filter((key) =>
      key.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
</script>

<div class="layout">
  <aside class="sidebar sidebar-explorer">
    <div class="sidebar-header">
      <h3>Keys ({filteredKeys.length})</h3>
      <div class="search-box">
        <input
          type="text"
          placeholder="Search keys..."
          bind:value={searchQuery}
        />
      </div>
    </div>

    <div class="keys-list">
      {#each filteredKeys as key}
        <button
          class="key-item"
          class:selected={selectedKey === key}
          onclick={() => selectKey(key)}
        >
          {key}
        </button>
      {/each}
    </div>

    <button class="btn-disconnect" onclick={disconnect}>Disconnect</button>
  </aside>

  <main class="value-panel">
    {#if selectedKey}
      <div class="value-header">
        <h2>{selectedKey}</h2>
        {#if keyValue}
          <span class="type-badge">{keyValue.type}</span>
        {/if}
      </div>
      <div class="value-content">
        {#if keyValue}
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

<style>
  .layout {
    display: flex;
    height: calc(100vh - 32px);
    overflow: hidden;
  }
  .sidebar {
    background-color: #232323;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow-y: auto;
    flex-shrink: 0;
  }
  .sidebar-explorer {
    width: 280px;
  }
  .sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid #333;
  }
  .sidebar-header h3 {
    margin: 0 0 0.75rem;
    font-size: 0.9rem;
    color: #aaa;
  }
  .search-box {
    display: flex;
    align-items: center;
    background-color: #2d2d2d;
    border-radius: 4px;
    padding: 0.4rem 0.75rem;
  }
  .search-box input {
    flex: 1;
    background: none;
    border: none;
    color: #e0e0e0;
    font-size: 0.85rem;
  }
  .search-box input:focus {
    outline: none;
  }
  .search-box input::placeholder {
    color: #aaa;
  }
  .keys-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 0.5rem;
  }
  .key-item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: #aaa;
    padding: 0.6rem 1rem;
    font-size: 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .key-item:hover {
    background-color: #2d2d2d;
    color: #e0e0e0;
  }
  .key-item.selected {
    background-color: #4a9eff;
    color: white;
  }
  .btn-disconnect {
    margin: 1rem;
    padding: 0.5rem 1rem;
    background-color: transparent;
    color: #f44336;
    border: 1px solid #f44336;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
  }
  .btn-disconnect:hover {
    background-color: #f44336;
    color: white;
  }
  .value-panel {
    flex: 1;
    padding: 1.5rem;
    overflow-y: auto;
  }
  .value-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .value-header h2 {
    margin: 0;
    font-size: 1.1rem;
    color: #e0e0e0;
    word-break: break-all;
  }
  .type-badge {
    padding: 0.25rem 0.5rem;
    background-color: #4a9eff;
    color: white;
    border-radius: 4px;
    font-size: 0.75rem;
    text-transform: uppercase;
  }
  .value-content {
    background-color: #232323;
    border-radius: 8px;
    padding: 1rem;
  }
  .value-content pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    color: #e0e0e0;
    font-size: 0.9rem;
    font-family: "Monaco", "Menlo", monospace;
  }
  .placeholder-text {
    color: #aaa;
    font-size: 0.9rem;
    text-align: center;
    padding: 3rem;
  }
</style>
