<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { START_HOUR, END_HOUR } from '$lib/utils';

  export let showLabel = true;
  export let showDot = true;
  export let baseDate: Date;

  let now = new Date();
  let percent = 0;
  let interval: ReturnType<typeof setInterval>;
  let isToday = false;

  function updatePosition() {
    now = new Date();
    
    // Check if baseDate is today
    const d1 = new Date(baseDate);
    const d2 = new Date();
    isToday = d1.getFullYear() === d2.getFullYear() &&
              d1.getMonth() === d2.getMonth() &&
              d1.getDate() === d2.getDate();

    if (!isToday) return;

    const startOfDay = new Date(now);
    startOfDay.setHours(START_HOUR, 0, 0, 0);

    const endOfDay = new Date(now);
    endOfDay.setHours(END_HOUR, 0, 0, 0);

    const diffMs = now.getTime() - startOfDay.getTime();
    const totalMs = endOfDay.getTime() - startOfDay.getTime();

    percent = (diffMs / totalMs) * 100;

    // Clamp between 0 and 100
    if (percent < 0) percent = 0;
    if (percent > 100) percent = 100;
  }

  onMount(() => {
    updatePosition();
    interval = setInterval(updatePosition, 60000); // Update every minute
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
      {now.toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit', hour12: false })}
    </div>
  {/if}
</div>
{/if}
