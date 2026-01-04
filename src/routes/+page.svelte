<script>
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => {
    goto("/login");
  });
</script>

<p>Redirecting...</p>
