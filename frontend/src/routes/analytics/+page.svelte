<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth, logout } from '$lib/auth';
  import type { PersonalAnalyticsResponse } from '$lib/types';

  let loading = true;
  let error: string | null = null;
  let analytics: PersonalAnalyticsResponse | null = null;

  $: maxHeatmapCount = analytics ? Math.max(...analytics.heatmap.map(day => day.count), 0) : 0;
  $: maxStatusCount = analytics ? Math.max(...analytics.task_stats.by_status.map(item => item.count), 0) : 0;

  function heatmapIntensityClass(count: number): string {
    if (count === 0) return 'bg-slate-100 border-slate-200';
    if (maxHeatmapCount <= 1) return 'bg-emerald-300 border-emerald-400';

    const ratio = count / maxHeatmapCount;
    if (ratio >= 0.75) return 'bg-emerald-700 border-emerald-800';
    if (ratio >= 0.5) return 'bg-emerald-500 border-emerald-600';
    if (ratio >= 0.25) return 'bg-emerald-300 border-emerald-400';
    return 'bg-emerald-200 border-emerald-300';
  }

  function formatStatus(status: string): string {
    if (status === 'todo') return 'Todo';
    if (status === 'doing') return 'Doing';
    if (status === 'done') return 'Done';
    return status;
  }

  async function fetchAnalytics() {
    if (!$auth.token) {
      goto('/');
      return;
    }

    loading = true;
    try {
      const res = await fetch('/api/analytics/personal', {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });

      if (!res.ok) {
        if (res.status === 401) {
          logout();
          goto('/');
          return;
        }
        throw new Error(`Failed to fetch analytics: ${res.statusText}`);
      }

      analytics = await res.json();
      error = null;
    } catch (e) {
      console.error('Failed to fetch analytics:', e);
      error = '分析データの取得に失敗しました。';
    } finally {
      loading = false;
    }
  }

  onMount(fetchAnalytics);
</script>

<div class="min-h-screen bg-slate-50">
  <header class="h-12 px-4 sm:px-6 flex items-center justify-between bg-white border-b border-slate-200 shadow-sm sticky top-0 z-10">
    <button on:click={() => goto('/')} class="inline-flex items-center gap-1.5 text-slate-500 hover:text-slate-700 text-sm font-semibold">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"></path></svg>
      Back
    </button>
    <h1 class="text-sm sm:text-base font-black text-slate-800 tracking-tight">Personal Analytics</h1>
    <div class="w-10"></div>
  </header>

  <main class="max-w-5xl mx-auto p-4 sm:p-6">
    {#if loading}
      <div class="rounded-xl border border-slate-200 bg-white p-8 text-center text-slate-500 font-semibold">Loading analytics...</div>
    {:else if error}
      <div class="rounded-xl border border-red-200 bg-red-50 p-8 text-center text-red-600 font-semibold">{error}</div>
    {:else if analytics}
      <section class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <article class="rounded-xl bg-white border border-slate-200 shadow-sm p-4 sm:p-5">
          <p class="text-xs uppercase tracking-wide text-slate-500 font-semibold">Completed Tasks</p>
          <p class="mt-2 text-3xl font-black text-slate-900">{analytics.task_stats.total_completed}</p>
          <div class="mt-3 flex items-center gap-4 text-xs">
            <span class="px-2 py-1 rounded-md bg-emerald-50 text-emerald-700 font-semibold">This Week: {analytics.task_stats.completed_this_week}</span>
            <span class="px-2 py-1 rounded-md bg-slate-100 text-slate-700 font-semibold">Last Week: {analytics.task_stats.completed_last_week}</span>
          </div>
        </article>

        <article class="rounded-xl bg-white border border-slate-200 shadow-sm p-4 sm:p-5">
          <p class="text-xs uppercase tracking-wide text-slate-500 font-semibold">Reports Submitted</p>
          <p class="mt-2 text-3xl font-black text-slate-900">{analytics.report_stats.total_submitted}</p>
          <p class="mt-3 text-xs text-slate-500">Total daily reports created by you.</p>
        </article>
      </section>

      <section class="mt-4 sm:mt-6 rounded-xl bg-white border border-slate-200 shadow-sm p-4 sm:p-5">
        <h2 class="text-sm font-bold text-slate-800">Tasks by Status</h2>
        {#if analytics.task_stats.by_status.length === 0}
          <p class="mt-3 text-sm text-slate-500">No tasks found.</p>
        {:else}
          <ul class="mt-3 space-y-2">
            {#each analytics.task_stats.by_status as item}
              <li class="space-y-1">
                <div class="flex items-center justify-between text-xs">
                  <span class="font-semibold text-slate-700">{formatStatus(item.status)}</span>
                  <span class="font-bold text-slate-500">{item.count}</span>
                </div>
                <div class="h-2 rounded-full bg-slate-100 overflow-hidden">
                  <div
                    class="h-full bg-blue-600 rounded-full"
                    style={`width: ${maxStatusCount === 0 ? 0 : (item.count / maxStatusCount) * 100}%`}
                  ></div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section class="mt-4 sm:mt-6 rounded-xl bg-white border border-slate-200 shadow-sm p-4 sm:p-5">
        <h2 class="text-sm font-bold text-slate-800">Activity Heatmap (Last 30 Days)</h2>
        <ul class="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-2">
          {#each analytics.heatmap as day}
            <li class="flex items-center justify-between rounded-lg border border-slate-200 px-3 py-2 bg-slate-50">
              <span class="text-xs font-semibold text-slate-600">{day.date}</span>
              <div class="flex items-center gap-2">
                <span class={`w-4 h-4 rounded-sm border ${heatmapIntensityClass(day.count)}`}></span>
                <span class="text-xs font-bold text-slate-700">{day.count}</span>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </main>
</div>
