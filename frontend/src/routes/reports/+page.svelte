<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/auth';
  import type { DailyReport, User } from '$lib/types';
  import { marked } from 'marked';
  import { goto } from '$app/navigation';

  let reports: DailyReport[] = [];
  let users: User[] = [];
  let loading = true;
  let filterDate = '';
  let filterUserId = '';
  let columns = 1; // 1 to 3

  async function fetchUsers() {
    try {
      const res = await fetch('http://localhost:3000/api/users', {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (res.ok) users = await res.json();
    } catch (e) {
      console.error(e);
    }
  }

  async function fetchReports() {
    loading = true;
    try {
      let url = 'http://localhost:3000/api/reports?';
      if (filterDate) url += `date=${filterDate}&`;
      if (filterUserId) url += `user_id=${filterUserId}&`;

      const res = await fetch(url, {
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

  onMount(async () => {
    if (!$auth.token) {
        goto('/');
        return;
    }
    const savedCols = localStorage.getItem('report_columns');
    if (savedCols) columns = parseInt(savedCols);

    await Promise.all([fetchUsers(), fetchReports()]);
  });

  function setColumns(n: number) {
    columns = n;
    localStorage.setItem('report_columns', n.toString());
  }

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

  function handleFilter() {
    fetchReports();
  }

  function clearFilters() {
    filterDate = '';
    filterUserId = '';
    fetchReports();
  }

  $: gridClass = columns === 1 ? 'grid-cols-1' : columns === 2 ? 'grid-cols-1 md:grid-cols-2' : 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3';
  $: maxWClass = columns === 1 ? 'max-w-3xl' : columns === 2 ? 'max-w-6xl' : 'max-w-[1600px]';
</script>

{#if !$auth.initialized}
  <div class="min-h-screen bg-surface-secondary text-text-muted flex items-center justify-center font-semibold">読み込み中...</div>
{:else}
<div class="min-h-screen bg-surface-secondary text-text-base flex flex-col font-sans">
  <header class="h-12 px-6 flex items-center justify-between bg-surface-primary border-b border-border-base shadow-sm sticky top-0 z-10">
    <div class="flex items-center gap-4">
      <button on:click={() => goto('/')} class="p-1.5 -ml-1.5 text-text-muted hover:text-text-base transition-colors" aria-label="ダッシュボードに戻る">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
      </button>
      <h1 class="text-sm font-black text-text-base tracking-tight uppercase">日報ログ</h1>
    </div>
    
    <div class="flex items-center gap-4">
      <!-- Column Switcher -->
      <div class="flex items-center bg-surface-muted rounded-lg p-0.5 border border-border-base">
        <button 
          on:click={() => setColumns(1)} 
          class="p-1.5 rounded-md transition-all {columns === 1 ? 'bg-surface-primary shadow-sm text-blue-600' : 'text-text-muted hover:text-text-base'}"
          title="1列表示"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><line x1="3" x2="21" y1="9" y2="9"/><line x1="3" x2="21" y1="15" y2="15"/></svg>
        </button>
        <button 
          on:click={() => setColumns(2)} 
          class="p-1.5 rounded-md transition-all {columns === 2 ? 'bg-surface-primary shadow-sm text-blue-600' : 'text-text-muted hover:text-text-base'}"
          title="2列表示"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><line x1="12" x2="12" y1="3" y2="21"/></svg>
        </button>
        <button 
          on:click={() => setColumns(3)} 
          class="p-1.5 rounded-md transition-all {columns === 3 ? 'bg-surface-primary shadow-sm text-blue-600' : 'text-text-muted hover:text-text-base'}"
          title="3列表示"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><line x1="9" x2="9" y1="3" y2="21"/><line x1="15" x2="15" y1="3" y2="21"/></svg>
        </button>
      </div>

      <button 
        on:click={() => goto('/reports/new')}
        class="btn-primary text-[10px] flex items-center gap-2"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14m-7-7v14"/></svg>
        日報を書く
      </button>
    </div>
  </header>

  <main class="{maxWClass} w-full mx-auto p-4 space-y-4 flex-1 transition-all duration-300">
    <!-- Filters -->
    <div class="bg-surface-primary p-3 rounded-xl border border-border-base shadow-sm flex flex-wrap items-end gap-3">
      <div class="flex-1 min-w-[150px]">
        <label for="reports-filter-date" class="block text-[8px] font-black text-text-muted uppercase tracking-widest mb-1">Date</label>
        <input 
          id="reports-filter-date"
          type="date" 
          bind:value={filterDate}
          on:change={handleFilter}
          class="form-control px-2 py-1.5 text-[10px]"
        />
      </div>
      <div class="flex-1 min-w-[150px]">
        <label for="reports-filter-user" class="block text-[8px] font-black text-text-muted uppercase tracking-widest mb-1">Member</label>
        <select 
          id="reports-filter-user"
          bind:value={filterUserId}
          on:change={handleFilter}
          class="form-control px-2 py-1.5 text-[10px]"
        >
          <option value="">全員</option>
          {#each users as user}
            <option value={user.id}>{user.name}</option>
          {/each}
        </select>
      </div>
      <button 
        on:click={clearFilters}
        class="btn-secondary px-3 py-1.5 text-[10px]"
      >
        Reset
      </button>
    </div>

    <!-- Reports List -->
    <div class="grid {gridClass} gap-4">
      {#if loading}
        <div class="col-span-full py-12 flex flex-col items-center justify-center gap-2">
          <div class="w-6 h-6 border-2 border-blue-500/10 border-t-blue-500 rounded-full animate-spin"></div>
          <p class="text-text-muted font-bold text-[10px] uppercase tracking-widest">Loading...</p>
        </div>
      {:else if reports.length === 0}
        <div class="col-span-full py-12 text-center bg-surface-primary rounded-xl border border-dashed border-border-base">
          <p class="text-text-muted text-[10px] font-bold uppercase tracking-widest">No reports found.</p>
        </div>
      {:else}
        {#each reports as report}
          <article class="bg-surface-primary rounded-xl border border-border-base shadow-sm overflow-hidden transition-all hover:border-blue-300/40 flex flex-col">
            <div class="px-4 py-3 bg-surface-secondary border-b border-border-base flex items-center justify-between shrink-0">
              <div class="flex items-center gap-2">
                <div class="w-6 h-6 bg-blue-100 rounded-full flex items-center justify-center text-blue-600 font-black text-[10px] uppercase">
                  {getUserName(report.user_id).charAt(0)}
                </div>
                <div>
                  <h3 class="text-xs font-bold text-text-base leading-none">{getUserName(report.user_id)}</h3>
                  <p class="text-[8px] text-text-muted font-black mt-1 uppercase tracking-tighter">{report.report_date}</p>
                </div>
              </div>
              <div class="flex items-center gap-2">
                {#if canEdit(report)}
                  <button 
                    on:click={() => goto(`/reports/${report.id}`)}
                    class="p-1.5 text-text-muted hover:text-blue-500 hover:bg-blue-500/10 rounded transition-all"
                    title="編集"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>
                  </button>
                {/if}
              </div>
            </div>
            <div class="p-4 prose prose-slate dark:prose-invert prose-sm max-w-none prose-headings:font-black prose-h2:text-xs prose-h2:mb-2 flex-1 overflow-y-auto max-h-[400px]">
              {@html renderMarkdown(report.content)}
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </main>
</div>
{/if}
