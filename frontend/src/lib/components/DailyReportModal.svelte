<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '../auth';
  import type { Task, DailyReport } from '../types';

  export let userTasks: Task[] = [];
  export let existingReport: DailyReport | null = null;
  const dispatch = createEventDispatcher();

  let content = '';
  let reportDate = new Date().toISOString().split('T')[0];
  let loading = false;

  onMount(() => {
    if (existingReport) {
      content = existingReport.content;
      reportDate = existingReport.report_date;
      return;
    }

    // Generate initial template from tasks
    const completedTasks = userTasks.filter(t => t.status === 'done');
    const ongoingTasks = userTasks.filter(t => t.status === 'doing' || t.status === 'todo');

    content = `## 今日の成果\n`;
    if (completedTasks.length > 0) {
      completedTasks.forEach(t => content += `- ${t.title} (完了)\n`);
    } else {
      content += `- 特になし\n`;
    }

    content += `\n## 明日の予定 / 残課題\n`;
    if (ongoingTasks.length > 0) {
      ongoingTasks.forEach(t => content += `- ${t.title} (継続)\n`);
    } else {
      content += `- 特になし\n`;
    }

    content += `\n## ひとこと\n- お疲れ様でした。`;
  });

  async function saveReport() {
    loading = true;
    try {
      const url = existingReport 
        ? `http://localhost:3000/api/reports/${existingReport.id}`
        : 'http://localhost:3000/api/reports';
      
      const method = existingReport ? 'PATCH' : 'POST';

      const res = await fetch(url, {
        method,
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify(existingReport ? { content } : {
          report_date: reportDate,
          content
        })
      });

      if (!res.ok) throw new Error('Failed to save report');
      dispatch('save');
      dispatch('close');
    } catch (e) {
      console.error(e);
      alert('日報の保存に失敗しました。');
    } finally {
      loading = false;
    }
  }
</script>

<div class="fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-[110] p-4">
  <div class="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh]">
    <div class="p-6 border-b border-slate-100 flex items-center justify-between">
      <h2 class="text-xl font-bold text-slate-800">{existingReport ? '日報を編集' : '日報作成'} ({reportDate})</h2>
      <button on:click={() => dispatch('close')} class="text-slate-400 hover:text-slate-600 transition-colors">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="p-6 flex-1 overflow-y-auto">
      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 mb-2">内容 (Markdown形式)</label>
        <textarea 
          bind:value={content} 
          class="w-full h-64 p-4 border border-slate-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none font-mono text-sm resize-none"
        ></textarea>
      </div>

      <div class="bg-slate-50 p-4 rounded-xl border border-slate-100">
        <p class="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2">タスクプレビュー (参考)</p>
        <div class="space-y-1">
          {#each userTasks as task}
            <div class="text-xs text-slate-600 flex items-center gap-2">
              <span class="w-2 h-2 rounded-full {task.status === 'done' ? 'bg-gray-400' : 'bg-blue-500'}"></span>
              {task.title}
            </div>
          {/each}
        </div>
      </div>
    </div>

    <div class="p-6 border-t border-slate-100 flex justify-end gap-3">
      <button on:click={() => dispatch('close')} class="px-6 py-2 text-slate-600 font-bold hover:bg-slate-50 rounded-lg transition-colors">キャンセル</button>
      <button 
        on:click={saveReport} 
        disabled={loading}
        class="px-6 py-2 bg-blue-600 text-white font-bold rounded-lg shadow-lg shadow-blue-200 hover:bg-blue-700 transition-colors disabled:opacity-50"
      >
        {loading ? '保存中...' : '日報を提出'}
      </button>
    </div>
  </div>
</div>
