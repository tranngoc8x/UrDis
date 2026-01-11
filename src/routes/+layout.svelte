<script>
  import { activeConfig } from "$lib/stores.js";
  import "../styles/layout.scss";
</script>

{#if $activeConfig}
  {@const config = $activeConfig}
  <div class="app-header">
    <div class="header-content">
      <div class="connection-pill">
        <span class="status-dot"></span>
        <span class="url-text">{config.host ?? ""}:{config.port ?? ""}</span>
      </div>
    </div>
  </div>
{/if}

<slot />
