<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPercentage, formatTime, isSameJSTDate } from '$lib/utils';

  export let showLabel = true;
  export let showDot = true;
  export let baseDate: Date; // 00:00 JST

  let now = new Date();
  let interval: ReturnType<typeof setInterval>;

  $: percent = getPercentage(now);
  $: isToday = isSameJSTDate(now, baseDate);

  onMount(() => {
    interval = setInterval(() => {
      now = new Date();
    }, 60000); // Update every minute
  });

  onDestroy(() => {
    clearInterval(interval);
  });
</script>

{#if isToday}
<div 
  class="absolute top-0 bottom-0 border-l-2 border-red-500 z-40 pointer-events-none transition-all duration-500 ease-linear"
  style="left: {percent}%"
>
  {#if showDot}
    <div class="absolute -top-1 -left-[5px] w-2 h-2 bg-red-500 rounded-full animate-ping"></div>
    <div class="absolute -top-1 -left-[5px] w-2 h-2 bg-red-500 rounded-full"></div>
  {/if}
  
  {#if showLabel}
    <div class="absolute top-1 -left-6 bg-red-500 text-white text-[10px] font-bold px-1.5 py-0.5 rounded shadow-sm whitespace-nowrap z-30">
      {formatTime(now)}
    </div>
  {/if}
</div>
{/if}
