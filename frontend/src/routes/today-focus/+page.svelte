<script lang="ts">
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { page } from '$app/stores';
  import TodayFocusPanel from '$lib/components/TodayFocusPanel.svelte';
  import Login from '$lib/components/Login.svelte';
  import { auth, logout } from '$lib/auth';
  import type { DisplayGroup, Task } from '$lib/types';
  import { getTodayJSTString } from '$lib/utils';

  let selectedDate = getTodayJSTString();
  let selectedGroupId: number | null = null;
  let filterText = '';
  let selectedTimelineMemberId: number | null = null;
  let displayGroups: DisplayGroup[] = [];
  let groupsLoaded = false;
  let initializedFromUrl = false;

  function parseDateParam(value: string | null): string | null {
    if (!value) return null;
    return /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : null;
  }

  function parseGroupParam(value: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
  }

  $: currentDateParam = parseDateParam($page.url.searchParams.get('date'));
  $: currentGroupParam = parseGroupParam($page.url.searchParams.get('group_id'));

  $: if (browser && !initializedFromUrl) {
    selectedDate = currentDateParam || localStorage.getItem('glanceflow_selected_date') || getTodayJSTString();
    selectedGroupId = currentGroupParam;
    initializedFromUrl = true;
  }

  $: if (browser && selectedDate) {
    localStorage.setItem('glanceflow_selected_date', selectedDate);
  }

  $: if (browser && initializedFromUrl) {
    const params = new URLSearchParams($page.url.searchParams);
    if (selectedDate) params.set('date', selectedDate);
    else params.delete('date');

    if (selectedGroupId !== null) params.set('group_id', String(selectedGroupId));
    else params.delete('group_id');

    const nextQuery = params.toString();
    const currentQuery = $page.url.searchParams.toString();
    if (nextQuery !== currentQuery) {
      goto(`/today-focus${nextQuery ? `?${nextQuery}` : ''}`, {
        replaceState: true,
        noScroll: true,
        keepFocus: true
      });
    }
  }

  async function fetchDisplayGroups() {
    if (!$auth.token) return;
    try {
      const res = await fetch('http://localhost:3000/api/display-groups', {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (res.status === 401) {
        logout();
        return;
      }
      if (!res.ok) {
        throw new Error(`Failed to fetch display groups: ${res.status}`);
      }
      displayGroups = await res.json();

      if (!selectedGroupId && displayGroups.length > 0) {
        selectedGroupId = displayGroups[0].id;
      }
    } catch (e) {
      console.error('Failed to fetch display groups:', e);
      displayGroups = [];
      selectedGroupId = null;
    }
  }

  function handleEditTask(event: CustomEvent<Task>) {
    goto(`/?task_id=${event.detail.id}`);
  }

  $: if ($auth.token && !groupsLoaded) {
    groupsLoaded = true;
    fetchDisplayGroups();
  }

  $: if (!$auth.token) {
    groupsLoaded = false;
    displayGroups = [];
    selectedGroupId = null;
  }
</script>

{#if !$auth.initialized}
  <div class="min-h-screen bg-surface-primary text-text-muted flex items-center justify-center font-semibold">読み込み中...</div>
{:else if !$auth.token}
  <Login on:loginSuccess={fetchDisplayGroups} />
{:else}
  <div class="min-h-screen bg-surface-primary text-text-base flex flex-col font-sans">
    <header class="h-12 px-4 sm:px-6 flex items-center justify-between bg-surface-secondary border-b border-border-base shadow-sm sticky top-0 z-10">
      <button on:click={() => goto('/')} class="inline-flex items-center gap-1.5 text-text-muted hover:text-text-base text-sm font-semibold">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"></path></svg>
        戻る
      </button>
      <h1 class="text-sm sm:text-base font-black text-text-base tracking-tight">本日のフォーカス</h1>
      <div class="w-10"></div>
    </header>

    <main class="max-w-6xl w-full mx-auto p-4 sm:p-6 flex-1">
      <section class="mb-3 rounded-xl border border-border-base bg-surface-secondary p-3 shadow-sm">
        <div class="flex flex-wrap items-center gap-2">
          <input
            type="date"
            bind:value={selectedDate}
            class="form-control w-auto rounded px-2 py-1.5 text-xs font-semibold"
          />
          <input
            type="text"
            bind:value={filterText}
            placeholder="絞り込み..."
            class="form-control w-52 rounded px-2.5 py-1.5 text-xs"
          />
          <select
            class="form-control w-auto rounded px-2.5 py-1.5 text-xs font-semibold"
            value={selectedGroupId ? String(selectedGroupId) : ''}
            on:change={(event) => {
              const value = (event.currentTarget as HTMLSelectElement).value;
              selectedGroupId = value ? Number(value) : null;
            }}
          >
            <option value="">全員</option>
            {#each displayGroups as group}
              <option value={group.id}>{group.name}</option>
            {/each}
          </select>
          <button
            type="button"
            on:click={fetchDisplayGroups}
            class="btn-secondary rounded px-2.5 py-1.5 text-[11px] font-bold"
          >
            グループ再読み込み
          </button>
        </div>
      </section>

      <TodayFocusPanel
        {selectedDate}
        {selectedGroupId}
        {filterText}
        {selectedTimelineMemberId}
        on:editTask={handleEditTask}
      />
    </main>
  </div>
{/if}
