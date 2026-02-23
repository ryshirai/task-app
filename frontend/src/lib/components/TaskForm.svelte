<script lang="ts">
  import { apiFetch } from '$lib/api';
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
  let description = '';
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
      const res = await apiFetch(`/api/tasks?member_id=${member_id}&status=todo,doing`, {
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
    description = task.description || '';
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
        description: selectedTaskId ? null : (description.trim() || null),
        tags: selectedTaskId ? null : tags, 
        task_id: selectedTaskId,
        start, 
        end 
      });
      title = '';
      description = '';
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
    if (!title.trim() && !description.trim() && !tagsInput.trim()) {
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
    class="absolute inset-0 bg-slate-950/40 backdrop-blur-md transition-all"
    aria-label="タスクフォームを閉じる"
    on:click={() => dispatch('cancel')}
  ></button>

  <div class="relative z-10 w-full max-w-lg rounded-3xl border border-[var(--border-base)] bg-[var(--surface-primary)] p-8 shadow-[var(--shadow-elevated)]">
    <div class="flex justify-between items-center mb-4">
      <div class="text-[10px] font-black tracking-widest text-[var(--text-muted)] uppercase">
        新規作業ログ
      </div>
      <div class="rounded-md border border-[color:color-mix(in_srgb,#2563eb_26%,var(--border-base))] bg-[color:color-mix(in_srgb,#dbeafe_55%,var(--surface-primary))] px-2.5 py-1 text-[11px] font-mono font-bold text-blue-700">
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
          <p class="mb-2 block text-[10px] font-bold uppercase text-[var(--text-muted)]">進行中のタスクから選ぶ</p>
          <div class="flex flex-wrap gap-1.5 max-h-32 overflow-y-auto pr-1">
            {#each activeTasks as task}
              <button
                type="button"
                on:click={() => selectTask(task)}
                class="rounded-xl border px-2.5 py-1.5 text-[11px] font-bold transition-all {selectedTaskId === task.id ? 'border-blue-600 bg-blue-600 text-white shadow-md shadow-blue-500/25 scale-95' : 'border-[var(--border-base)] bg-[var(--surface-secondary)] text-[var(--text-muted)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]'}"
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
        <label for="task-form-title" class="mb-1.5 block text-[10px] font-bold uppercase text-[var(--text-muted)]">タスク名</label>
        <div class="relative">
          <input
            id="task-form-title"
            bind:this={inputElement}
            bind:value={title}
            on:input={() => selectedTaskId = null}
            on:keydown={handleKeydown}
            placeholder="新しいタスク名を入力..."
            list="task-suggestions"
            class="form-control px-3 py-2.5 text-xs font-bold transition-all focus:ring-2"
          />
          <datalist id="task-suggestions">
            {#each existingTasks as taskTitle}
              <option value={taskTitle}></option>
            {/each}
          </datalist>
          {#if selectedTaskId}
            <div class="absolute right-3 top-1/2 -translate-y-1/2">
              <span class="rounded border border-[color:color-mix(in_srgb,#2563eb_28%,var(--border-base))] bg-[color:color-mix(in_srgb,#dbeafe_60%,var(--surface-primary))] px-1.5 py-0.5 text-[9px] font-black uppercase tracking-tighter text-blue-600">リンク済み</span>
            </div>
          {/if}
        </div>
      </div>

      <div>
        <label for="task-form-tags" class="mb-1.5 block text-[10px] font-bold uppercase text-[var(--text-muted)]">タグ</label>
        <input
          id="task-form-tags"
          bind:value={tagsInput}
          on:keydown={handleKeydown}
          placeholder="開発, デザイン, 会議など (カンマ区切り)"
          class="form-control px-3 py-2 text-[11px] transition-all focus:ring-2"
        />
      </div>

      <div>
        <label for="task-form-description" class="mb-1.5 block text-[10px] font-bold uppercase text-[var(--text-muted)]">詳細</label>
        <textarea
          id="task-form-description"
          bind:value={description}
          on:keydown={handleKeydown}
          rows="3"
          placeholder="詳細"
          class="form-control resize-y px-3 py-2 text-[11px] transition-all focus:ring-2"
        ></textarea>
      </div>

      <div class="flex items-center justify-end gap-3 pt-2">
        <button
          type="button"
          on:click={() => dispatch('cancel')}
          class="btn-secondary px-4 py-2 text-[11px]"
        >
          キャンセル
        </button>
        <button
          type="submit"
          class="btn-primary min-w-[90px] px-5 py-2.5 text-[12px] font-black active:scale-95"
          disabled={!title.trim()}
        >
          記録を開始
        </button>
      </div>
    </form>
  </div>
</div>
