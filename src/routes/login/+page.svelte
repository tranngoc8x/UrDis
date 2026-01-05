<script>
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { activeConfig, savedServers } from "$lib/stores.js";
  import { resizeWindow } from "$lib/utils.js";
  import { simplebar } from "$lib/actions.js";

  let redisHost = $state("");
  let redisPort = $state("");
  let redisStatus = $state("");
  let isConnecting = $state(false);
  let isError = $state(false);
  let connectingServer = $state(null);

  let showNewServerModal = $state(false);
  let editingIndex = $state(null);
  let newServer = $state({
    name: "",
    host: "localhost",
    port: "6379",
    password: "",
    username: "",
    enableSSL: false,
  });

  function openNewServerModal(index = null) {
    if (index !== null) {
      editingIndex = index;
      const servers = $savedServers;
      const s = servers[index];
      newServer = {
        name: s.name || "",
        host: s.host || "localhost",
        port: s.port || "6379",
        password: s.password || "",
        username: s.username || "",
        enableSSL: s.enableSSL || false,
      };
    } else {
      editingIndex = null;
      newServer = {
        name: "",
        host: "localhost",
        port: "6379",
        password: "",
        username: "",
        enableSSL: false,
      };
    }
    showNewServerModal = true;
  }

  function closeNewServerModal() {
    showNewServerModal = false;
    editingIndex = null;
  }

  function createServer() {
    const host = newServer.host.trim();
    if (!host) return;

    const serverData = {
      name: newServer.name.trim() || `${host}:${newServer.port}`,
      host: host,
      port: (newServer.port || "6379").trim(),
      password: newServer.password,
      username: newServer.username,
      enableSSL: newServer.enableSSL,
    };

    if (editingIndex !== null) {
      savedServers.update((servers) => {
        servers[editingIndex] = serverData;
        return [...servers];
      });
    } else {
      savedServers.add(serverData);
    }
    closeNewServerModal();
  }

  async function quickConnect(event) {
    event?.preventDefault();
    const host = redisHost.trim();
    if (!host) return;
    const port = redisPort.trim() || "6379";
    const config = { host, port, username: "", password: "", enableSSL: false };
    await connectToServer(config, false);
  }

  async function connectToServer(config, addToList = false) {
    isConnecting = true;
    connectingServer = `${config.host}:${config.port}`;
    redisStatus = "";
    isError = false;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      redisStatus = await invoke("connect_redis", { config });
      if (redisStatus.startsWith("Successfully")) {
        isError = false;
        activeConfig.set(config);
        await resizeWindow(1200, 700);
        if (addToList) savedServers.add(config);
        goto("/explorer");
      }
    } catch (error) {
      redisStatus = `Connection failed: ${error}`;
      isError = true;
      redisHost = config.host;
      redisPort = config.port;
    } finally {
      isConnecting = false;
      connectingServer = null;
    }
  }
</script>

<div class="layout">
  <aside class="sidebar">
    <div class="sidebar-logo">
      <img src="/icon.png" alt="UrDis Logo" />
      <span class="app-name">UrDis</span>
    </div>
    <button class="btn-new-server" onclick={() => openNewServerModal()}
      >+ New Redis Server</button
    >
  </aside>

  <main class="main-content">
    <section class="quick-connect">
      <h2>Quick Connect</h2>
      <form onsubmit={quickConnect}>
        <div class="input-row">
          <div class="input-group host">
            <label for="quick-host">Host</label>
            <input
              id="quick-host"
              type="text"
              bind:value={redisHost}
              placeholder="localhost"
            />
          </div>
          <div class="input-group port">
            <label for="quick-port">Port</label>
            <input
              id="quick-port"
              type="text"
              bind:value={redisPort}
              placeholder="6379"
            />
          </div>
          <button class="btn-connect" type="submit" disabled={isConnecting}>
            {isConnecting ? "Connecting..." : "Connect"}
          </button>
        </div>
      </form>
      {#if redisStatus}
        <div
          class="status-message"
          class:success={!isError}
          class:error={isError}
        >
          {redisStatus}
        </div>
      {/if}
    </section>

    <section class="server-list-section">
      <h2>Saved Servers</h2>
      {#if $savedServers.length === 0}
        <div class="empty-state">
          No saved servers. Click "+ New Redis Server" to add one.
        </div>
      {:else}
        <div class="server-list" use:simplebar>
          {#each $savedServers as server, index}
            <div class="server-item">
              <div class="server-info">
                <div class="server-name">
                  {server.name}
                  {#if server.password || server.username}
                    <span class="icon-lock" title="Has authentication"
                      >&#128274;</span
                    >
                  {/if}
                  {#if server.enableSSL}
                    <span class="icon-ssl" title="SSL enabled">SSL</span>
                  {/if}
                </div>
                <div class="server-host">{server.host}:{server.port}</div>
              </div>
              <button
                class="btn-server-connect"
                onclick={() => connectToServer(server, false)}
                disabled={isConnecting &&
                  connectingServer === `${server.host}:${server.port}`}
              >
                {isConnecting &&
                connectingServer === `${server.host}:${server.port}`
                  ? "..."
                  : "Connect"}
              </button>
              <button
                class="btn-server-edit"
                onclick={() => openNewServerModal(index)}>Edit</button
              >
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </main>
</div>

{#if showNewServerModal}
  <div
    class="modal-overlay"
    onclick={closeNewServerModal}
    onkeydown={(e) => e.key === "Escape" && closeNewServerModal()}
    role="button"
    tabindex="0"
    aria-label="Close modal"
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
    >
      <h2>
        {editingIndex !== null ? "Edit Redis Server" : "New Redis Server"}
      </h2>
      <div class="form-group">
        <label for="server-name">Name</label>
        <input
          id="server-name"
          type="text"
          bind:value={newServer.name}
          placeholder="localhost"
        />
      </div>
      <div class="form-row">
        <div class="form-group flex-2">
          <label for="server-host">Host</label>
          <input
            id="server-host"
            type="text"
            bind:value={newServer.host}
            placeholder="localhost"
          />
        </div>
        <div class="form-group flex-1">
          <label for="server-port">Port</label>
          <input
            id="server-port"
            type="text"
            bind:value={newServer.port}
            placeholder="6379"
          />
        </div>
      </div>
      <div class="form-group">
        <label for="server-username">Username</label>
        <input
          id="server-username"
          type="text"
          bind:value={newServer.username}
          placeholder="Optional"
        />
        <small>Username is supported after Redis 6 with Redis ACL.</small>
      </div>
      <div class="form-group">
        <label for="server-password">Password</label>
        <input
          id="server-password"
          type="password"
          bind:value={newServer.password}
          placeholder=""
        />
      </div>
      <div class="form-group checkbox-group">
        <label for="server-ssl">
          <input
            id="server-ssl"
            type="checkbox"
            bind:checked={newServer.enableSSL}
          />
          Enable SSL
        </label>
      </div>
      <div class="modal-actions">
        <button class="btn-modal-cancel" onclick={closeNewServerModal}
          >Cancel</button
        >
        <button class="btn-modal-create" onclick={createServer}
          >{editingIndex !== null ? "Save" : "Create"}</button
        >
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  @import "./login.scss";
</style>
