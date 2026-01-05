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

<style lang="scss">
  @import "./layout.scss";
</style>
