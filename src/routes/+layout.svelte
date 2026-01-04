<script>
  import { activeConfig } from "$lib/stores.js";
</script>

<div class="app-header">
  <div class="header-content">
    {#if $activeConfig}
      <div class="connection-pill">
        <span class="status-dot"></span>
        <span class="url-text">{$activeConfig.host}:{$activeConfig.port}</span>
      </div>
    {/if}
  </div>
</div>

<slot />

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
  .app-header {
    height: 32px;
    background-color: #1a1a1a;
    display: flex;
    align-items: center;
    border-bottom: 1px solid #333;
    user-select: none;
    position: relative;
    z-index: 100;
    padding-left: 80px;
  }
  .header-content {
    flex: 1;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    margin-right: 80px;
  }
  .connection-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    background-color: #2d2d2d;
    padding: 2px 12px;
    border-radius: 12px;
    border: 1px solid #444;
  }
  .status-dot {
    width: 6px;
    height: 6px;
    background-color: #4caf50;
    border-radius: 50%;
  }
  .url-text {
    font-size: 0.75rem;
    color: #aaa;
  }
</style>
