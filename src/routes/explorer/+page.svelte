<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig } from "$lib/stores.js";
  import { resizeWindow, formatKeyValue } from "$lib/utils.js";
  import "$src/styles/global.scss";
  import "$src/styles/explorer.scss";

  let keysList = $state([]);
  let selectedKey = $state("");
  let keyValue = $state(null);
  let searchQuery = $state("");

  // Redirect if not connected
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
  <!-- Sidebar -->
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

    <button class="btn-disconnect" onclick={disconnect}> Disconnect </button>
  </aside>

  <!-- Main Content -->
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
