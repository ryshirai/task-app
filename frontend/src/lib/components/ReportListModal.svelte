<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '../auth';
  import type { DailyReport, User } from '../types';
  import { marked } from 'marked';
  import DailyReportModal from './DailyReportModal.svelte';

  export let users: User[] = [];
  const dispatch = createEventDispatcher();

  let reports: DailyReport[] = [];
  let loading = true;
  let editingReport: DailyReport | null = null;

  async function fetchReports() {
    loading = true;
    try {
      const res = await fetch('http://localhost:3000/api/reports', {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error('Failed to fetch reports');
      reports = await res.json();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  onMount(fetchReports);

  function getUserName(userId: number) {
    const user = users.find(u => u.id === userId);
    return user ? user.name : '不明なユーザー';
  }

  function renderMarkdown(content: string) {
    return marked.parse(content);
  }

  function canEdit(report: DailyReport) {
    return report.user_id === $auth.user?.id || $auth.user?.role === 'admin';
  }
</script>

<div class="fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-[100] p-4">
  <div class="bg-white rounded-2xl shadow-2xl w-full max-w-3xl flex flex-col max-h-[90vh]">
    <div class="p-6 border-b border-slate-100 flex items-center justify-between">
      <h2 class="text-xl font-bold text-slate-800">日報一覧</h2>
      <button on:click={() => dispatch('close')} class="text-slate-400 hover:text-slate-600" aria-label="日報一覧を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="p-6 flex-1 overflow-y-auto space-y-6">
      {#if loading}
        <div class="text-center text-slate-400">読み込み中...</div>
      {:else if reports.length === 0}
        <div class="text-center text-slate-400 italic">まだ日報がありません。</div>
      {:else}
        {#each reports as report}
          <div class="bg-slate-50 rounded-xl border border-slate-100 overflow-hidden shadow-sm">
            <div class="px-4 py-3 bg-slate-100/80 border-b border-slate-200 flex items-center justify-between">
              <span class="font-bold text-slate-800 flex items-center gap-2">
                <div class="w-2 h-2 bg-blue-500 rounded-full"></div>
                {getUserName(report.user_id)}
              </span>
              <div class="flex items-center gap-3">
                {#if canEdit(report)}
                  <button 
                    on:click={() => editingReport = report}
                    class="text-xs text-blue-600 hover:text-blue-800 font-bold px-2 py-1 hover:bg-blue-50 rounded transition-colors"
                  >
                    編集
                  </button>
                {/if}
                <span class="text-xs text-slate-500 font-mono font-bold bg-white px-2 py-1 rounded border border-slate-200">{report.report_date}</span>
              </div>
            </div>
            <div class="p-6 text-sm text-slate-800 leading-relaxed prose prose-slate max-w-none prose-headings:text-slate-900 prose-headings:font-bold prose-h2:text-base prose-h2:mt-0 prose-li:my-0">
              {@html renderMarkdown(report.content)}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  {#if editingReport}
    <DailyReportModal 
      existingReport={editingReport} 
      on:close={() => editingReport = null} 
      on:save={fetchReports}
    />
  {/if}
</div>
