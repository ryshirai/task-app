<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { initializeTheme, theme } from '$lib/theme';

  let { children } = $props();

  onMount(() => {
    initializeTheme();
  });
</script>

<svelte:body class:dark={$theme === 'dark'} />

<div class="min-h-screen text-[var(--color-text)]">
  {@render children()}
</div>
