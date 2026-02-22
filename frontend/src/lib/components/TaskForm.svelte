<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { formatTime } from '$lib/utils';
  import { auth } from '$lib/auth';
  import type { Task } from '$lib/types';

  /** Selected start time for the new work log. */
  export let start: Date;
  /** Selected end time for the new work log. */
  export let end: Date;
  /** Member ID that owns this work log. */
  export let member_id: number;
  /** Existing task titles used for datalist suggestions. */
  export let existingTasks: string[] = [];

  const dispatch = createEventDispatcher();
  let title = '';
  let tagsInput = '';
  let selectedTaskId: number | null = null;
  let activeTasks: Task[] = [];
  let loadingActiveTasks = false;
  
  let inputElement: HTMLInputElement;
  let formElement: HTMLFormElement;

  /** Loads active tasks for the selected member to allow linking. */
  async function fetchActiveTasks() {
    if (!$auth.token) return;
    loadingActiveTasks = true;
    try {
      const res = await fetch(`http://localhost:3000/api/tasks?member_id=${member_id}&status=todo,doing`, {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (res.ok) {
        activeTasks = await res.json();
      }
    } catch (e) {
      console.error('Failed to fetch active tasks:', e);
    } finally {
      loadingActiveTasks = false;
    }
  }

  /** Prefills form values from an existing active task. */
  function selectTask(task: Task) {
    title = task.title.toString();
    tagsInput = (task.tags || []).join(', ');
    selectedTaskId = task.id;
    // Auto-focus tags after selecting a title if it's already structured
    setTimeout(() => inputElement.focus(), 0);
  }

  /** Emits submit payload or cancels when no usable input exists. */
  function handleSubmit() {
    if (title.trim()) {
      const tags = tagsInput.split(',').map(t => t.trim()).filter(t => t !== '');
      dispatch('submit', { 
        title: selectedTaskId ? null : title, 
        tags: selectedTaskId ? null : tags, 
        task_id: selectedTaskId,
        start, 
        end 
      });
      title = '';
      tagsInput = '';
      selectedTaskId = null;
    } else {
      dispatch('cancel');
    }
  }

  /** Handles Escape to close the modal quickly. */
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      dispatch('cancel');
    }
  }

  /** Cancels only when focus leaves the form and there is no input in progress. */
  function handleFormFocusOut(event: FocusEvent) {
    const nextTarget = event.relatedTarget as Node | null;
    if (nextTarget && formElement?.contains(nextTarget)) return;
    // Don't auto-cancel if we have a title or we are interacting with active tasks
    if (!title.trim() && !tagsInput.trim()) {
      dispatch('cancel');
    }
  }

  onMount(() => {
    inputElement.focus();
    fetchActiveTasks();
  });
</script>

<div class="fixed inset-0 z-[200] flex items-center justify-center p-4">
  <button
    type="button"
    class="absolute inset-0 bg-slate-900/45 backdrop-blur-sm"
    aria-label="タスクフォームを閉じる"
    on:click={() => dispatch('cancel')}
  ></button>

  <div class="relative z-10 w-full max-w-md rounded-2xl border border-slate-200 bg-white p-5 shadow-2xl">
    <div class="flex justify-between items-center mb-4">
      <div class="text-[10px] font-black tracking-widest text-slate-400 uppercase">
        新規作業ログ
      </div>
      <div class="text-[11px] font-mono font-bold text-blue-600 bg-blue-50 px-2 py-0.5 rounded">
        {formatTime(start)} - {formatTime(end)}
      </div>
    </div>

    <form
      bind:this={formElement}
      on:submit|preventDefault={handleSubmit}
      on:focusout={handleFormFocusOut}
      class="w-full space-y-4"
    >
      {#if activeTasks.length > 0}
        <div>
          <p class="block text-[10px] font-bold text-slate-500 uppercase mb-2">進行中のタスクから選ぶ</p>
          <div class="flex flex-wrap gap-1.5 max-h-32 overflow-y-auto pr-1">
            {#each activeTasks as task}
              <button
                type="button"
                on:click={() => selectTask(task)}
                class="px-2.5 py-1.5 rounded-lg border text-[11px] font-bold transition-all {selectedTaskId === task.id ? 'bg-blue-600 border-blue-600 text-white shadow-md shadow-blue-200 scale-95' : 'bg-slate-50 border-slate-200 text-slate-600 hover:border-slate-400 hover:bg-slate-100'}"
              >
                <span class="flex items-center gap-1.5">
                  <span class="w-1.5 h-1.5 rounded-full {task.status === 'doing' ? 'bg-blue-400 animate-pulse' : 'bg-yellow-400'}"></span>
                  {task.title}
                </span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <div>
        <label for="task-form-title" class="block text-[10px] font-bold text-slate-500 uppercase mb-1.5">タスク名</label>
        <div class="relative">
          <input
            id="task-form-title"
            bind:this={inputElement}
            bind:value={title}
            on:input={() => selectedTaskId = null}
            on:keydown={handleKeydown}
            placeholder="新しいタスク名を入力..."
            list="task-suggestions"
            class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-xs font-bold text-slate-800 outline-none transition focus:border-blue-500 focus:ring-4 focus:ring-blue-50"
          />
          <datalist id="task-suggestions">
            {#each existingTasks as taskTitle}
              <option value={taskTitle}></option>
            {/each}
          </datalist>
          {#if selectedTaskId}
            <div class="absolute right-3 top-1/2 -translate-y-1/2">
              <span class="text-[9px] font-black text-blue-500 uppercase tracking-tighter bg-blue-50 px-1.5 py-0.5 rounded border border-blue-100">Linked</span>
            </div>
          {/if}
        </div>
      </div>

      <div>
        <label for="task-form-tags" class="block text-[10px] font-bold text-slate-500 uppercase mb-1.5">タグ</label>
        <input
          id="task-form-tags"
          bind:value={tagsInput}
          on:keydown={handleKeydown}
          placeholder="開発, デザイン, 会議など (カンマ区切り)"
          class="w-full rounded-xl border border-slate-200 px-3 py-2 text-[11px] text-slate-600 outline-none transition focus:border-blue-500 focus:ring-4 focus:ring-blue-50"
        />
      </div>

      <div class="flex items-center justify-end gap-3 pt-2">
        <button
          type="button"
          on:click={() => dispatch('cancel')}
          class="px-4 py-2 text-[11px] font-bold text-slate-400 hover:text-slate-600 transition-colors"
        >
          キャンセル
        </button>
        <button
          type="submit"
          class="min-w-[80px] rounded-xl bg-slate-900 px-5 py-2.5 text-[12px] font-black text-white transition-all hover:bg-slate-800 hover:shadow-lg active:scale-95 disabled:opacity-50"
          disabled={!title.trim()}
        >
          記録を開始
        </button>
      </div>
    </form>
  </div>
</div>
