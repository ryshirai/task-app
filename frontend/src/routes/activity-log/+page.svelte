<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth } from '$lib/auth';
  import type { ActivityLog } from '$lib/types';

  let logs: ActivityLog[] = [];
  let loading = true;
  let currentPage = 1;
  let totalPages = 1;

  async function fetchLogs(page: number) {
    loading = true;
    try {
      const res = await fetch(`http://localhost:3000/api/logs?page=${page}`, {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error('Failed to fetch logs');
      const data = await res.json();
      logs = data.items;
      totalPages = data.total_pages;
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function changePage(page: number) {
    if (page < 1 || page > totalPages || page === currentPage) return;
    currentPage = page;
    await fetchLogs(currentPage);
  }

  onMount(async () => {
    if (!$auth.token) {
      goto('/');
      return;
    }
    await fetchLogs(currentPage);
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

<div class="min-h-screen bg-slate-50 flex flex-col font-sans">
  <header class="h-12 px-6 flex items-center gap-4 bg-white border-b border-slate-200 shadow-sm sticky top-0 z-10">
    <button on:click={() => goto('/')} class="p-1.5 -ml-1.5 text-slate-400 hover:text-slate-600 transition-colors" aria-label="ホームへ戻る">
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
    </button>
    <h1 class="text-sm font-black text-slate-800 tracking-tight uppercase">アクティビティログ</h1>
  </header>

  <main class="max-w-4xl w-full mx-auto p-4 md:p-6 flex-1">
    <div class="bg-white rounded-2xl shadow-sm border border-slate-200 p-4 md:p-5">
      {#if loading}
        <div class="text-center text-slate-400">読み込み中...</div>
      {:else if logs.length === 0}
        <div class="text-center text-slate-400 italic">まだログがありません。</div>
      {:else}
        <div class="overflow-x-auto">
          <table class="table-auto w-full border-collapse text-xs">
            <thead>
              <tr class="border-b border-slate-200 bg-slate-50">
                <th class="px-3 py-1.5 text-left font-bold text-slate-600">日時</th>
                <th class="px-3 py-1.5 text-left font-bold text-slate-600">ユーザー</th>
                <th class="px-3 py-1.5 text-left font-bold text-slate-600">操作</th>
                <th class="px-3 py-1.5 text-left font-bold text-slate-600">詳細</th>
              </tr>
            </thead>
            <tbody>
              {#each logs as log}
                <tr class="border-b border-slate-100 align-top">
                  <td class="px-3 py-1.5 text-slate-500 font-mono whitespace-nowrap">
                    {new Date(log.created_at).toLocaleString('ja-JP')}
                  </td>
                  <td class="px-3 py-1.5 text-slate-800 font-semibold whitespace-nowrap">
                    {log.user_name}
                  </td>
                  <td class="px-3 py-1.5">
                    <span class="inline-block px-2 py-0.5 rounded text-[10px] font-bold uppercase {getActionColor(log.action)}">
                      {formatAction(log.action)}
                    </span>
                  </td>
                  <td class="px-3 py-1.5 text-slate-600">
                    {#if log.details}
                      {log.details}
                    {:else}
                      <span class="text-slate-400 italic">-</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <div class="mt-4 flex items-center justify-end gap-3 text-xs">
        <button
          class="px-3 py-1.5 rounded border border-slate-300 text-slate-700 disabled:opacity-40 disabled:cursor-not-allowed"
          on:click={() => changePage(currentPage - 1)}
          disabled={loading || currentPage <= 1}
        >
          Previous
        </button>
        <span class="text-slate-500 font-mono">page {currentPage} / {totalPages}</span>
        <button
          class="px-3 py-1.5 rounded border border-slate-300 text-slate-700 disabled:opacity-40 disabled:cursor-not-allowed"
          on:click={() => changePage(currentPage + 1)}
          disabled={loading || currentPage >= totalPages}
        >
          Next
        </button>
      </div>
    </div>
  </main>
</div>
