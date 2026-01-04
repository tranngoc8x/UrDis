<script>
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { activeConfig, savedServers } from "$lib/stores.js";
  import { resizeWindow } from "$lib/utils.js";

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
        <div class="server-list">
          {#each $savedServers as server, index}
            <div class="server-item">
              <div class="server-info">
                <div class="server-name">{server.name}</div>
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

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    color: #e0e0e0;
    background-color: #1a1a1a;
  }
  .layout {
    display: flex;
    height: calc(100vh - 32px);
    overflow: hidden;
  }
  .sidebar {
    width: 220px;
    background-color: #232323;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow-y: auto;
    flex-shrink: 0;
  }
  .sidebar-logo {
    padding: 1rem;
    display: flex;
    justify-content: center;
  }
  .sidebar-logo img {
    width: 80px;
    height: auto;
  }
  .btn-new-server {
    margin: 0 1rem 2rem;
    padding: 0.6rem 1rem;
    background: linear-gradient(135deg, #4a9eff 0%, #357abd 100%);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    transition:
      transform 0.2s,
      box-shadow 0.2s;
  }
  .btn-new-server:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(74, 158, 255, 0.3);
  }
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1.5rem;
    overflow-y: auto;
  }
  .quick-connect {
    background-color: #232323;
    border-radius: 8px;
    padding: 1rem 1.5rem;
    margin-bottom: 1.5rem;
  }
  .quick-connect h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }
  .input-row {
    display: flex;
    gap: 0.75rem;
    align-items: flex-end;
  }
  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .input-group label {
    font-size: 0.8rem;
    color: #aaa;
  }
  .input-group input {
    padding: 0.5rem 0.75rem;
    background-color: #2d2d2d;
    border: 1px solid #333;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 0.9rem;
  }
  .input-group input:focus {
    outline: none;
    border-color: #4a9eff;
  }
  .input-group.host {
    flex: 2;
  }
  .input-group.port {
    flex: 1;
  }
  .btn-connect {
    padding: 0.5rem 1.25rem;
    background-color: #4a9eff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .btn-connect:hover {
    background-color: #357abd;
  }
  .btn-connect:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .status-message {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .status-message.success {
    background-color: rgba(76, 175, 80, 0.1);
    color: #4caf50;
  }
  .status-message.error {
    background-color: rgba(244, 67, 54, 0.1);
    color: #f44336;
  }
  .server-list-section {
    background-color: #232323;
    border-radius: 8px;
    padding: 1rem 1.5rem;
    flex: 1;
    overflow-y: auto;
  }
  .server-list-section h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }
  .server-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .server-item {
    display: flex;
    align-items: center;
    padding: 0.75rem 1rem;
    background-color: #2d2d2d;
    border-radius: 6px;
    cursor: pointer;
  }
  .server-item:hover {
    background-color: #363636;
  }
  .server-info {
    flex: 1;
  }
  .server-name {
    font-weight: 500;
    margin-bottom: 0.25rem;
  }
  .server-host {
    font-size: 0.8rem;
    color: #aaa;
  }
  .btn-server-connect {
    padding: 0.4rem 0.8rem;
    background-color: transparent;
    color: #4a9eff;
    border: 1px solid #4a9eff;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .btn-server-connect:hover {
    background-color: #4a9eff;
    color: white;
  }
  .btn-server-edit {
    padding: 0.4rem 0.6rem;
    background-color: transparent;
    color: #aaa;
    border: none;
    cursor: pointer;
    margin-left: 0.5rem;
  }
  .btn-server-edit:hover {
    color: #e0e0e0;
  }
  .empty-state {
    color: #aaa;
    font-size: 0.9rem;
    text-align: center;
    padding: 2rem;
  }
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    background-color: #232323;
    border-radius: 12px;
    padding: 1.5rem;
    width: 400px;
    max-width: 90%;
  }
  .modal h2 {
    margin: 0 0 1.5rem;
    font-size: 1.2rem;
  }
  .form-group {
    margin-bottom: 1rem;
  }
  .form-group label {
    display: block;
    font-size: 0.85rem;
    color: #aaa;
    margin-bottom: 0.4rem;
  }
  .form-group input[type="text"],
  .form-group input[type="password"] {
    width: 100%;
    padding: 0.6rem 0.75rem;
    background-color: #2d2d2d;
    border: 1px solid #333;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 0.9rem;
    box-sizing: border-box;
  }
  .form-group input:focus {
    outline: none;
    border-color: #4a9eff;
  }
  .form-group small {
    display: block;
    margin-top: 0.4rem;
    font-size: 0.75rem;
    color: #aaa;
  }
  .form-row {
    display: flex;
    gap: 1rem;
  }
  .flex-1 {
    flex: 1;
  }
  .flex-2 {
    flex: 2;
  }
  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }
  .checkbox-group input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: #4a9eff;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }
  .btn-modal-cancel {
    padding: 0.6rem 1.25rem;
    background-color: transparent;
    color: #aaa;
    border: 1px solid #333;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .btn-modal-cancel:hover {
    background-color: #2d2d2d;
    color: #e0e0e0;
  }
  .btn-modal-create {
    padding: 0.6rem 1.25rem;
    background-color: #4a9eff;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .btn-modal-create:hover {
    background-color: #357abd;
  }
</style>
