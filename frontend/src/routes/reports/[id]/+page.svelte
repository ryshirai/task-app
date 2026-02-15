<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/auth';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import type { DailyReport } from '$lib/types';
  import ReportPreview from '$lib/components/ReportPreview.svelte';

  let id = $page.params.id;
  let content = '';
  let reportDate = '';
  let loading = true;
  let saving = false;

  async function fetchReport() {
    try {
      const res = await fetch(`http://localhost:3000/api/reports/${id}`, {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error('Failed to fetch report');
      const report: DailyReport = await res.json();
      content = report.content;
      reportDate = report.report_date;
      
      if (report.user_id !== $auth.user?.id && $auth.user?.role !== 'admin') {
          alert('この日報を編集する権限がありません。');
          goto('/reports');
      }
    } catch (e) {
      console.error(e);
      alert('日報の取得に失敗しました。');
      goto('/reports');
    } finally {
      loading = false;
    }
  }

  async function updateReport() {
    if (!content.trim()) return;
    saving = true;
    try {
      const res = await fetch(`http://localhost:3000/api/reports/${id}`, {
        method: 'PATCH',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({ content })
      });

      if (!res.ok) throw new Error('Failed to update report');
      goto('/reports');
    } catch (e) {
      console.error(e);
      alert('日報の更新に失敗しました。');
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    if (!$auth.token) {
        goto('/');
        return;
    }
    fetchReport();
  });
</script>

<div class="min-h-screen bg-slate-50 flex flex-col font-sans">
  <header class="h-12 px-6 flex items-center justify-between bg-white border-b border-slate-200 shadow-sm sticky top-0 z-10">
    <div class="flex items-center gap-4">
      <button on:click={() => goto('/reports')} class="p-1.5 -ml-1.5 text-slate-400 hover:text-slate-600 transition-colors">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
      </button>
      <h1 class="text-sm font-black text-slate-800 tracking-tight uppercase">日報編集</h1>
    </div>
    <div class="flex items-center gap-3">
        <button 
            on:click={updateReport} 
            disabled={saving || !content.trim()}
            class="px-4 py-1.5 bg-slate-900 text-white text-xs font-bold rounded-lg shadow-sm hover:bg-slate-800 transition-all disabled:opacity-50 active:scale-95"
        >
            {saving ? '保存中...' : '変更を保存'}
        </button>
    </div>
  </header>

  <main class="max-w-[1200px] w-full mx-auto p-4 flex-1 flex flex-col md:flex-row gap-4 overflow-hidden">
    {#if loading}
        <div class="flex-1 flex flex-col items-center justify-center gap-3">
            <div class="w-6 h-6 border-2 border-slate-200 border-t-slate-800 rounded-full animate-spin"></div>
            <p class="text-slate-400 font-bold text-[10px] uppercase tracking-widest">Loading Report...</p>
        </div>
    {:else}
        <!-- Editor Side -->
        <div class="flex-1 flex flex-col gap-4 min-w-0">
            <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-sm flex flex-col flex-1 min-h-[400px]">
                <div class="flex items-center justify-between mb-3 pb-2 border-b border-slate-50">
                    <div class="flex items-center gap-2">
                        <span class="text-[10px] font-black text-slate-400 uppercase tracking-widest">対象日:</span>
                        <span class="text-xs font-bold text-slate-700">{reportDate}</span>
                    </div>
                    <span class="text-[8px] font-black text-blue-500 bg-blue-50 px-2 py-0.5 rounded-full uppercase tracking-widest">Edit Mode</span>
                </div>

                <textarea 
                    bind:value={content} 
                    class="flex-1 w-full p-4 border border-slate-100 rounded-xl focus:ring-2 focus:ring-blue-500/10 focus:border-blue-500 outline-none font-mono text-xs resize-none bg-slate-50/20 leading-relaxed"
                ></textarea>
            </div>
        </div>

        <!-- Preview Side -->
        <div class="w-full md:w-[400px] shrink-0 h-[400px] md:h-auto">
            <ReportPreview {content} title="表示プレビュー" />
        </div>
    {/if}
  </main>
</div>
