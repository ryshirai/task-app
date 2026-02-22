<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { auth, logout } from '$lib/auth';
  import type { PersonalAnalyticsResponse } from '$lib/types';

  let loading = true;
  let error: string | null = null;
  let analytics: PersonalAnalyticsResponse | null = null;
  let observedAnalyticsUserId: string | null | undefined = undefined;
  let fetchInProgress = false;
  let fetchQueued = false;

  $: maxHeatmapCount = analytics ? Math.max(...analytics.heatmap.map(day => day.count), 0) : 0;
  $: maxStatusCount = analytics ? Math.max(...analytics.task_stats.by_status.map(item => item.count), 0) : 0;

  function heatmapIntensityClass(count: number): string {
    if (count === 0) return 'bg-surface-muted border-border-base';
    if (maxHeatmapCount <= 1) return 'bg-emerald-300 border-emerald-400';

    const ratio = count / maxHeatmapCount;
    if (ratio >= 0.75) return 'bg-emerald-700 border-emerald-800';
    if (ratio >= 0.5) return 'bg-emerald-500 border-emerald-600';
    if (ratio >= 0.25) return 'bg-emerald-300 border-emerald-400';
    return 'bg-emerald-200 border-emerald-300';
  }

  function formatStatus(status: string): string {
    if (status === 'todo') return '未着手';
    if (status === 'doing') return '進行中';
    if (status === 'done') return '完了';
    return status;
  }

  async function fetchAnalytics() {
    if (!$auth.token) {
      loading = false;
      return;
    }

    if (fetchInProgress) {
      fetchQueued = true;
      return;
    }

    fetchInProgress = true;
    const userId = $page.url.searchParams.get('user_id');
    const endpoint = userId
      ? `http://localhost:3000/api/analytics/users/${userId}`
      : 'http://localhost:3000/api/analytics/personal';

    loading = true;
    try {
      const res = await fetch(endpoint, {
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
      fetchInProgress = false;
      loading = false;
      if (fetchQueued) {
        fetchQueued = false;
        fetchAnalytics();
      }
    }
  }

  $: analyticsUserId = $page.url.searchParams.get('user_id');
  $: if ($auth.initialized && !$auth.token) {
    goto('/');
  }
  $: if ($auth.initialized && $auth.token && analyticsUserId !== observedAnalyticsUserId) {
    observedAnalyticsUserId = analyticsUserId;
    fetchAnalytics();
  }
</script>

{#if !$auth.initialized}
  <div class="min-h-screen bg-surface-muted text-text-muted flex items-center justify-center font-semibold">読み込み中...</div>
{:else}
<div class="min-h-screen bg-surface-muted text-text-base">
  <header class="sticky top-0 z-10 flex h-12 items-center justify-between border-b border-border-base bg-surface-primary px-4 shadow-sm sm:px-6">
    <button on:click={() => goto('/')} class="inline-flex items-center gap-1.5 text-sm font-semibold text-text-muted hover:text-text-base">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"></path></svg>
      戻る
    </button>
    <h1 class="text-sm font-black tracking-tight text-text-base sm:text-base">
      {analytics ? `${analytics.user_name}のパフォーマンス分析` : '個人パフォーマンス分析'}
    </h1>
    <div class="w-10"></div>
  </header>

  <main class="max-w-5xl mx-auto p-4 sm:p-6">
    {#if loading}
      <div class="rounded-xl border border-border-base bg-surface-primary p-8 text-center font-semibold text-text-muted">分析データを読み込み中...</div>
    {:else if error}
      <div class="rounded-xl border border-red-200 bg-red-50 p-8 text-center text-red-600 font-semibold">{error}</div>
    {:else if analytics}
      <section class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <article class="rounded-xl border border-border-base bg-surface-primary p-4 shadow-sm sm:p-5">
          <p class="text-xs font-semibold uppercase tracking-wide text-text-muted">完了済みタスク</p>
          <p class="mt-2 text-3xl font-black text-text-base">{analytics.task_stats.total_completed}</p>
          <div class="mt-3 flex items-center gap-4 text-xs">
            <span class="px-2 py-1 rounded-md bg-emerald-50 text-emerald-700 font-semibold">今週: {analytics.task_stats.completed_this_week}</span>
            <span class="rounded-md bg-surface-muted px-2 py-1 font-semibold text-text-base">先週: {analytics.task_stats.completed_last_week}</span>
          </div>
        </article>

        <article class="rounded-xl border border-border-base bg-surface-primary p-4 shadow-sm sm:p-5">
          <p class="text-xs font-semibold uppercase tracking-wide text-text-muted">提出済み日報</p>
          <p class="mt-2 text-3xl font-black text-text-base">{analytics.report_stats.total_submitted}</p>
          <p class="mt-3 text-xs text-text-muted">これまでに作成した日報の総数です。</p>
        </article>
      </section>

      <section class="mt-4 rounded-xl border border-border-base bg-surface-primary p-4 shadow-sm sm:mt-6 sm:p-5">
        <h2 class="text-sm font-bold text-text-base">ステータス別タスク</h2>
        {#if analytics.task_stats.by_status.length === 0}
          <p class="mt-3 text-sm text-text-muted">タスクが見つかりませんでした。</p>
        {:else}
          <ul class="mt-3 space-y-2">
            {#each analytics.task_stats.by_status as item}
              <li class="space-y-1">
                <div class="flex items-center justify-between text-xs">
                  <span class="font-semibold text-text-base">{formatStatus(item.status)}</span>
                  <span class="font-bold text-text-muted">{item.count}</span>
                </div>
                <div class="h-2 overflow-hidden rounded-full bg-surface-muted">
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

      <section class="mt-4 rounded-xl border border-border-base bg-surface-primary p-4 shadow-sm sm:mt-6 sm:p-5">
        <h2 class="text-sm font-bold text-text-base">最近30日のアクティビティ</h2>
        <ul class="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-2">
          {#each [...analytics.heatmap].reverse() as day}
            <li class="flex items-center justify-between rounded-lg border border-border-base bg-surface-secondary px-3 py-2">
              <span class="text-xs font-semibold text-text-muted">{day.date}</span>
              <div class="flex items-center gap-2">
                <span class={`w-4 h-4 rounded-sm border ${heatmapIntensityClass(day.count)}`}></span>
                <span class="text-xs font-bold text-text-base">{day.count}</span>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </main>
</div>
{/if}
