<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '../auth';
  import type { ActivityLog } from '../types';

  const dispatch = createEventDispatcher();

  let logs: ActivityLog[] = [];
  let loading = true;

  onMount(async () => {
    try {
      const res = await fetch('http://localhost:3000/api/logs', {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error('Failed to fetch logs');
      logs = await res.json();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  });

  function formatAction(action: string) {
    const map: Record<string, string> = {
      task_created: 'タスクを作成',
      task_updated: 'タスクを更新',
      task_deleted: 'タスクを削除',
      report_submitted: '日報を提出',
      report_updated: '日報を更新'
    };
    return map[action] || action;
  }

  function getActionColor(action: string) {
    if (action.includes('created')) return 'text-emerald-600 bg-emerald-50';
    if (action.includes('deleted')) return 'text-red-600 bg-red-50';
    return 'text-blue-600 bg-blue-50';
  }
</script>

<div class="fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-[100] p-4">
  <div class="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[80vh]">
    <div class="p-6 border-b border-slate-100 flex items-center justify-between">
      <h2 class="text-xl font-bold text-slate-800">アクティビティログ</h2>
      <button on:click={() => dispatch('close')} class="text-slate-400 hover:text-slate-600" aria-label="アクティビティログを閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="p-6 flex-1 overflow-y-auto">
      {#if loading}
        <div class="text-center text-slate-400">読み込み中...</div>
      {:else if logs.length === 0}
        <div class="text-center text-slate-400 italic">まだログがありません。</div>
      {:else}
        <div class="space-y-4">
          {#each logs as log}
            <div class="flex gap-4">
              <div class="flex flex-col items-center">
                <div class="w-2 h-2 rounded-full mt-2 {log.action.includes('created') ? 'bg-emerald-500' : 'bg-slate-300'}"></div>
                <div class="w-0.5 flex-1 bg-slate-100"></div>
              </div>
              <div class="flex-1 pb-4">
                <div class="flex items-center gap-2 mb-1">
                  <span class="font-bold text-slate-800 text-sm">{log.user_name}</span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase {getActionColor(log.action)}">
                    {formatAction(log.action)}
                  </span>
                  <span class="text-[10px] text-slate-400 font-mono ml-auto">
                    {new Date(log.created_at).toLocaleString('ja-JP')}
                  </span>
                </div>
                {#if log.details}
                  <p class="text-xs text-slate-500 bg-slate-50 p-2 rounded border border-slate-100 italic">
                    {log.details}
                  </p>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
