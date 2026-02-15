<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '$lib/utils';

  export let start: Date;
  export let end: Date;

  const dispatch = createEventDispatcher();
  let title = '';
  let tagsInput = '';
  let inputElement: HTMLInputElement;
  let formElement: HTMLFormElement;

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

  function handleFormFocusOut(event: FocusEvent) {
    const nextTarget = event.relatedTarget as Node | null;
    if (nextTarget && formElement?.contains(nextTarget)) return;
    if (!title.trim() && !tagsInput.trim()) {
      dispatch('cancel');
    }
  }

  import { onMount } from 'svelte';
  onMount(() => {
    inputElement.focus();
  });
</script>

<div class="fixed inset-0 z-[200] flex items-center justify-center p-4">
  <button
    type="button"
    class="absolute inset-0 bg-slate-900/45"
    aria-label="Close task form"
    on:click={() => dispatch('cancel')}
  ></button>

  <div class="relative z-10 w-full max-w-md rounded-lg border border-slate-300 bg-white p-3 shadow-2xl">
    <div class="mb-2 text-[10px] font-bold tracking-wide text-slate-500">
      {formatTime(start)} - {formatTime(end)}
    </div>
    <form
      bind:this={formElement}
      on:submit|preventDefault={handleSubmit}
      on:focusout={handleFormFocusOut}
      class="w-full space-y-2"
    >
      <input
        bind:this={inputElement}
        bind:value={title}
        on:keydown={handleKeydown}
        placeholder="タスク名..."
        class="w-full rounded-md border border-slate-300 px-2 py-1.5 text-xs font-semibold text-slate-800 outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
      />
      <input
        bind:value={tagsInput}
        on:keydown={handleKeydown}
        placeholder="タグ (カンマ区切り)"
        class="w-full rounded-md border border-slate-200 px-2 py-1.5 text-[11px] text-slate-600 outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
      />
      <div class="flex items-center justify-end gap-2">
        <button
          type="submit"
          class="rounded-md bg-blue-600 px-3 py-1.5 text-[11px] font-semibold text-white transition hover:bg-blue-700"
        >
          追加
        </button>
        <button
          type="button"
          on:click={() => dispatch('cancel')}
          class="rounded-md border border-slate-300 px-3 py-1.5 text-[11px] font-semibold text-slate-700 transition hover:bg-slate-50"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
</div>
