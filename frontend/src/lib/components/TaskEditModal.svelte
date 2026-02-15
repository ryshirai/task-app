<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { type Task } from '$lib/types';
  import { formatTime, toLocalISOString } from '$lib/utils';

  export let task: Task;

  const dispatch = createEventDispatcher();
  
  let title = task.title;
  let status = task.status;
  let progress_rate = task.progress_rate;
  let tagsInput = (task.tags || []).join(', ');
  let start_time = new Date(task.start_at).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit', hour12: false });
  let end_time = new Date(task.end_at).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit', hour12: false });

  // Simple time input parsing (HH:MM -> Date)
  function parseTime(timeStr: string, baseDate: Date): Date {
    const [h, m] = timeStr.split(':').map(Number);
    const d = new Date(baseDate);
    d.setHours(h, m, 0, 0);
    return d;
  }

  async function handleSave() {
    // Validate inputs
    if (!title.trim()) return;

    const baseDate = new Date(task.start_at); // Keep original date, just update time
    const newStart = parseTime(start_time, baseDate);
    const newEnd = parseTime(end_time, baseDate);

    // Ensure end > start
    if (newEnd <= newStart) {
      alert('終了時間は開始時間より後である必要があります。');
      return;
    }

    const updatedTask = {
      ...task,
      title,
      status,
      progress_rate: Number(progress_rate),
      tags: tagsInput.split(',').map(t => t.trim()).filter(t => t !== ''),
      start_at: toLocalISOString(newStart),
      end_at: toLocalISOString(newEnd)
    };
    
    dispatch('save', updatedTask);
  }

  function handleDelete() {
    if (confirm('このタスクを削除しますか？')) {
      dispatch('delete', task.id);
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
      <h3 class="text-xl font-bold text-slate-800">タスク編集</h3>
      <button on:click={handleClose} class="text-slate-400 hover:text-slate-600" aria-label="タスク編集を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div>
        <label class="block text-xs font-bold text-slate-500 uppercase mb-1">タイトル</label>
        <input 
          bind:value={title} 
          class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-slate-900 font-medium"
          placeholder="タスク名"
        />
      </div>

      <div>
        <label class="block text-xs font-bold text-slate-500 uppercase mb-1">タグ (カンマ区切り)</label>
        <input 
          bind:value={tagsInput} 
          class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-slate-900 text-sm"
          placeholder="Project A, 重要, 会議"
        />
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

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs font-bold text-slate-500 uppercase mb-1">ステータス</label>
          <select 
            bind:value={status}
            class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-slate-900 bg-white"
          >
            <option value="todo">未着手</option>
            <option value="doing">進行中</option>
            <option value="done">完了</option>
          </select>
        </div>
        <div>
          <label class="block text-xs font-bold text-slate-500 uppercase mb-1">進捗 ({progress_rate}%)</label>
          <input 
            type="range" 
            min="0" 
            max="100" 
            step="10" 
            bind:value={progress_rate}
            class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-slate-900 mt-3"
          />
        </div>
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
