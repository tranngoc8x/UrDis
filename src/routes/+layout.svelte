<script>
  import { activeConfig } from "$lib/stores.js";
</script>

{#if $activeConfig}
  <div class="app-header">
    <div class="header-content">
      <div class="connection-pill">
        <span class="status-dot"></span>
        <span class="url-text">{$activeConfig.host}:{$activeConfig.port}</span>
      </div>
    </div>
  </div>
{/if}

<slot />

<style lang="scss">
  @import "./layout.scss";
</style>
