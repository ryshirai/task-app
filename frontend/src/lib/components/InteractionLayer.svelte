<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { xToTime, snapTo15Min, getPercentage } from '$lib/utils';

  const dispatch = createEventDispatcher();
  export let baseDate: Date;

  let isDragging = false;
  let startX = 0;
  let currentX = 0;
  let containerWidth = 0;
  let container: HTMLDivElement;

  let selectionStart: Date | null = null;
  let selectionEnd: Date | null = null;

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return; // Only left click
    isDragging = true;
    const rect = container.getBoundingClientRect();
    startX = e.clientX - rect.left;
    currentX = startX;
    containerWidth = rect.width;
    
    selectionStart = snapTo15Min(xToTime(startX, containerWidth, baseDate));
    selectionEnd = selectionStart;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    const rect = container.getBoundingClientRect();
    currentX = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    
    const timeAtCursor = xToTime(currentX, containerWidth, baseDate);
    selectionEnd = snapTo15Min(timeAtCursor);
  }

  function handleMouseUp() {
    if (!isDragging) return;
    isDragging = false;
    
    if (selectionStart && selectionEnd && selectionStart.getTime() !== selectionEnd.getTime()) {
      // Ensure start is before end
      const start = selectionStart < selectionEnd ? selectionStart : selectionEnd;
      const end = selectionStart < selectionEnd ? selectionEnd : selectionStart;
      
      dispatch('select', { start, end });
    }
    
    selectionStart = null;
    selectionEnd = null;
  }

  $: previewStyle = (() => {
    if (!selectionStart || !selectionEnd) return '';
    const startP = getPercentage(selectionStart);
    const endP = getPercentage(selectionEnd);
    const left = Math.min(startP, endP);
    const width = Math.abs(endP - startP);
    return `left: ${left}%; width: ${width}%;`;
  })();
</script>

<div
  bind:this={container}
  class="absolute inset-0 z-10 cursor-crosshair"
  on:mousedown={handleMouseDown}
  role="presentation"
>
  {#if isDragging && selectionStart && selectionEnd}
    <div
      class="absolute top-0 bottom-0 bg-blue-400/50 border-x border-blue-600 pointer-events-none"
      style={previewStyle}
    ></div>
  {/if}
</div>

<svelte:window on:mousemove={handleMouseMove} on:mouseup={handleMouseUp} />
