<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { type TaskTimeLog } from '$lib/types';
  import { toLocalISOString, formatTime } from '$lib/utils';

  export let timeLog: TaskTimeLog;

  const dispatch = createEventDispatcher();
  
  let start_time = formatTime(new Date(timeLog.start_at));
  let end_time = formatTime(new Date(timeLog.end_at));
  let task_description = timeLog.task_description || '';

  // Simple time input parsing (HH:MM -> Date)
  function parseTime(timeStr: string, baseDate: Date): Date {
    const [h, m] = timeStr.split(':').map(Number);
    // baseDate is 00:00 JST. 
    // We add h hours and m minutes to its absolute time.
    const d = new Date(baseDate.getTime());
    d.setUTCMinutes(d.getUTCMinutes() + (h * 60 + m));
    return d;
  }

  async function handleSave() {
    const baseDate = new Date(timeLog.start_at); // Keep original date, just update time
    const newStart = parseTime(start_time, baseDate);
    const newEnd = parseTime(end_time, baseDate);

    // Ensure end > start
    if (newEnd <= newStart) {
      alert('終了時間は開始時間より後である必要があります。');
      return;
    }

    const updatedTask = {
      ...timeLog,
      start_at: toLocalISOString(newStart),
      end_at: toLocalISOString(newEnd),
      task_description: task_description.trim() || null
    };
    
    dispatch('save', updatedTask);
  }

  function handleDelete() {
    if (confirm('この作業ログを削除しますか？')) {
      dispatch('delete', timeLog.id);
    }
  }

  function handleClose() {
    dispatch('close');
  }

  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="w-[min(92vw,30rem)] rounded-3xl border border-[var(--border-base)] bg-[var(--surface-primary)] p-0 text-[var(--text-primary)] shadow-[var(--shadow-elevated)] open:animate-in open:fade-in open:zoom-in-95 backdrop:bg-slate-950/40 backdrop:backdrop-blur-md backdrop:animate-in backdrop:fade-in"
  on:close={handleClose}
>
  <div class="p-8">
    <div class="mb-6 flex items-center justify-between">
      <h3 class="text-xl font-extrabold tracking-tight text-[var(--text-primary)]">作業ログ編集</h3>
      <button on:click={handleClose} class="rounded-lg p-1 text-[var(--text-muted)] transition-all hover:bg-[var(--surface-secondary)] hover:text-[var(--text-primary)]" aria-label="作業ログ編集を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-5">
      <div>
        <div class="mb-1 block text-[11px] font-bold uppercase tracking-wide text-[var(--text-muted)]">タスク</div>
        <div class="w-full rounded-xl border border-[var(--border-base)] bg-[var(--surface-secondary)] px-3 py-2.5 text-sm font-semibold text-[var(--text-primary)]">
          {timeLog.task_title || `Task #${timeLog.task_id}`}
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label for="task-edit-start" class="mb-1 block text-[11px] font-bold uppercase tracking-wide text-[var(--text-muted)]">開始</label>
          <input 
            id="task-edit-start"
            type="time"
            bind:value={start_time}
            class="form-control px-3 py-2.5 text-sm font-semibold transition-all focus:ring-2"
          />
        </div>
        <div>
          <label for="task-edit-end" class="mb-1 block text-[11px] font-bold uppercase tracking-wide text-[var(--text-muted)]">終了</label>
          <input 
            id="task-edit-end"
            type="time"
            bind:value={end_time}
            class="form-control px-3 py-2.5 text-sm font-semibold transition-all focus:ring-2"
          />
        </div>
      </div>

      <div>
        <label for="task-edit-description" class="mb-1 block text-[11px] font-bold uppercase tracking-wide text-[var(--text-muted)]">詳細</label>
        <textarea
          id="task-edit-description"
          bind:value={task_description}
          rows="3"
          placeholder="タスクの詳細を入力..."
          class="form-control w-full resize-y px-3 py-2.5 text-sm transition-all focus:ring-2"
        ></textarea>
      </div>

      <div class="flex items-end justify-between">
        <div class="text-xs text-[var(--text-muted)]">
          このログの所要時間: <span class="font-bold text-[var(--text-primary)]">{Math.round(timeLog.duration_minutes)}</span> 分
        </div>
        {#if timeLog.total_duration_minutes}
          <div class="rounded-lg border border-[var(--border-base)] bg-[var(--surface-secondary)] px-2 py-1 text-xs text-[var(--text-muted)]">
            タスク合計: <span class="font-bold text-[var(--text-primary)]">
              {Math.floor(timeLog.total_duration_minutes / 60)}時間{timeLog.total_duration_minutes % 60}分
            </span>
          </div>
        {/if}
      </div>
    </div>

    <div class="mt-8 flex items-center justify-between border-t border-[var(--border-base)] pt-4">
      <button 
        on:click={handleDelete}
        class="btn-danger px-3 py-2 text-sm"
      >
        削除
      </button>
      <div class="flex gap-3">
        <button 
          on:click={handleClose}
          class="btn-secondary px-4 py-2 text-sm"
        >
          キャンセル
        </button>
        <button 
          on:click={handleSave}
          class="btn-primary px-5 py-2 text-sm active:scale-95"
        >
          保存
        </button>
      </div>
    </div>
  </div>
</dialog>
