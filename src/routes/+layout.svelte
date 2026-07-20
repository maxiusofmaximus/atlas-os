<script lang="ts">
  // OpenCode OS — root layout shell. Keeps the HUD mounted at all routes.
  // Design tokens (CSS file) loaded at +page level until deps installed.
  const { children } = $props();
</script>

{@render children()}
