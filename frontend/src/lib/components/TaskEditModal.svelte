<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { type TaskTimeLog } from '$lib/types';
  import { toLocalISOString, formatTime } from '$lib/utils';

  export let timeLog: TaskTimeLog;

  const dispatch = createEventDispatcher();
  
  let start_time = formatTime(new Date(timeLog.start_at));
  let end_time = formatTime(new Date(timeLog.end_at));

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
      end_at: toLocalISOString(newEnd)
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
  class="backdrop:bg-black/50 p-0 rounded-xl shadow-2xl w-[400px] open:animate-in open:fade-in open:zoom-in-95 backdrop:animate-in backdrop:fade-in"
  on:close={handleClose}
>
  <div class="p-6 bg-white">
    <div class="flex justify-between items-center mb-6">
      <h3 class="text-xl font-bold text-slate-800">作業ログ編集</h3>
      <button on:click={handleClose} class="text-slate-400 hover:text-slate-600" aria-label="作業ログ編集を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div>
        <label class="block text-xs font-bold text-slate-500 uppercase mb-1">タスク</label>
        <div class="w-full px-3 py-2 border border-slate-200 rounded-lg bg-slate-50 text-sm font-semibold text-slate-700">
          {timeLog.task_title || `Task #${timeLog.task_id}`}
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs font-bold text-slate-500 uppercase mb-1">開始</label>
          <input 
            type="time"
            bind:value={start_time}
            class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-slate-900"
          />
        </div>
        <div>
          <label class="block text-xs font-bold text-slate-500 uppercase mb-1">終了</label>
          <input 
            type="time"
            bind:value={end_time}
            class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-slate-900"
          />
        </div>
      </div>

      <div class="flex justify-between items-end">
        <div class="text-xs text-slate-500">
          このログの所要時間: <span class="font-bold text-slate-700">{Math.round(timeLog.duration_minutes)}</span> 分
        </div>
        {#if timeLog.total_duration_minutes}
          <div class="text-xs text-slate-500 bg-slate-50 px-2 py-1 rounded border border-slate-100">
            タスク合計: <span class="font-bold text-slate-900">
              {Math.floor(timeLog.total_duration_minutes / 60)}時間{timeLog.total_duration_minutes % 60}分
            </span>
          </div>
        {/if}
      </div>
    </div>

    <div class="mt-8 flex justify-between items-center pt-4 border-t border-slate-100">
      <button 
        on:click={handleDelete}
        class="text-red-500 text-sm font-bold hover:text-red-600 px-3 py-2 rounded hover:bg-red-50 transition-colors"
      >
        削除
      </button>
      <div class="flex gap-3">
        <button 
          on:click={handleClose}
          class="px-4 py-2 text-slate-500 font-bold text-sm hover:text-slate-700"
        >
          キャンセル
        </button>
        <button 
          on:click={handleSave}
          class="px-5 py-2 bg-slate-900 text-white font-bold text-sm rounded-lg hover:bg-slate-800 shadow-sm active:scale-95 transition-all"
        >
          保存
        </button>
      </div>
    </div>
  </div>
</dialog>
