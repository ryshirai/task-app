<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime, getPercentage } from '$lib/utils';

  export let start: Date;
  export let end: Date;

  const dispatch = createEventDispatcher();
  let title = '';
  let tagsInput = '';
  let inputElement: HTMLInputElement;

  $: left = getPercentage(start);
  $: width = getPercentage(end) - left;

  function handleSubmit() {
    if (title.trim()) {
      const tags = tagsInput.split(',').map(t => t.trim()).filter(t => t !== '');
      dispatch('submit', { title, tags, start, end });
      title = '';
      tagsInput = '';
    } else {
      dispatch('cancel');
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      dispatch('cancel');
    }
  }

  import { onMount } from 'svelte';
  onMount(() => {
    inputElement.focus();
  });
</script>

<div
  class="absolute top-0 bottom-0 z-50 bg-white border-2 border-blue-500 rounded shadow-lg px-2 flex items-center min-w-[150px]"
  style="left: {left}%; width: {Math.max(width, 15)}%;"
>
  <div class="flex flex-col w-full">
    <div class="text-[10px] text-blue-600 font-bold">
      {formatTime(start)} - {formatTime(end)}
    </div>
    <form on:submit|preventDefault={handleSubmit} class="w-full space-y-1">
      <input
        bind:this={inputElement}
        bind:value={title}
        on:keydown={handleKeydown}
        on:blur={() => !title && !tagsInput && dispatch('cancel')}
        placeholder="タスク名..."
        class="w-full text-xs font-bold outline-none bg-transparent border-b border-blue-100"
      />
      <input
        bind:value={tagsInput}
        placeholder="タグ (カンマ区切り)"
        class="w-full text-[9px] outline-none bg-transparent text-slate-400"
      />
    </form>
  </div>
</div>
