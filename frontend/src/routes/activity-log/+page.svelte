<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth } from '$lib/auth';
  import type { ActivityLog, User } from '$lib/types';

  let logs: ActivityLog[] = [];
  let users: User[] = [];
  let loading = true;
  let exporting = false;
  let errorMessage = '';
  let currentPage = 1;
  let totalPages = 1;
  let filterUserId = '';
  let filterStartDate = '';
  let filterEndDate = '';
  type LogChange = {
    field: string;
    old: unknown;
    new: unknown;
  };

  function buildQueryParams(includePagination: boolean): URLSearchParams {
    const params = new URLSearchParams();
    if (includePagination) params.set('page', String(currentPage));
    if (filterUserId) params.set('user_id', filterUserId);
    if (filterStartDate) params.set('start_date', filterStartDate);
    if (filterEndDate) params.set('end_date', filterEndDate);
    return params;
  }

  async function fetchUsers() {
    try {
      const res = await fetch('http://localhost:3000/api/users', {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error(`Failed to fetch users (${res.status})`);
      users = await res.json();
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : 'Failed to fetch users';
    }
  }

  async function fetchLogs() {
    loading = true;
    errorMessage = '';
    try {
      const params = buildQueryParams(true);
      const res = await fetch(`http://localhost:3000/api/logs?${params.toString()}`, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error(`Failed to fetch logs (${res.status})`);
      const data = await res.json();
      logs = data.items;
      totalPages = data.total_pages;
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : 'Failed to fetch logs';
    } finally {
      loading = false;
    }
  }

  async function changePage(page: number) {
    if (page < 1 || page > totalPages || page === currentPage) return;
    currentPage = page;
    await fetchLogs();
  }

  async function applyFilters() {
    currentPage = 1;
    await fetchLogs();
  }

  async function exportCsv() {
    if (!$auth.token) {
      errorMessage = 'Authentication required';
      return;
    }

    exporting = true;
    errorMessage = '';
    try {
      const params = buildQueryParams(false);
      const url = `http://localhost:3000/api/logs/export?${params.toString()}`;
      const res = await fetch(url, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });

      if (!res.ok) {
        const message = await res.text();
        throw new Error(message || `Failed to export CSV (${res.status})`);
      }

      const blob = await res.blob();
      const contentDisposition = res.headers.get('content-disposition') ?? '';
      const filenameMatch = /filename="?([^"]+)"?/.exec(contentDisposition);
      const filename = filenameMatch?.[1] || 'activity_logs.csv';

      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = filename;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(objectUrl);
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : 'Failed to export CSV';
    } finally {
      exporting = false;
    }
  }

  onMount(async () => {
    if (!$auth.token) {
      goto('/');
      return;
    }
    await Promise.all([fetchUsers(), fetchLogs()]);
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

  function parseDetails(details?: string): { changes: LogChange[]; text: string | null } {
    if (!details) {
      return { changes: [], text: null };
    }

    try {
      const parsed = JSON.parse(details) as { changes?: unknown };
      if (parsed && Array.isArray(parsed.changes)) {
        const changes = parsed.changes
          .filter((item): item is LogChange => {
            return (
              !!item &&
              typeof item === 'object' &&
              'field' in item &&
              'old' in item &&
              'new' in item &&
              typeof (item as { field: unknown }).field === 'string'
            );
          })
          .map((item) => ({
            field: item.field,
            old: item.old,
            new: item.new
          }));

        return { changes, text: null };
      }
    } catch {
      // Backward compatibility for legacy plain-string details
    }

    return { changes: [], text: details };
  }

  function formatField(field: string): string {
    return field.replace(/_/g, ' ');
  }

  function formatDetailValue(value: unknown): string {
    if (value === null || value === undefined) return '-';
    if (Array.isArray(value)) return value.length > 0 ? value.join(', ') : '-';
    if (typeof value === 'object') {
      try {
        return JSON.stringify(value);
      } catch {
        return String(value);
      }
    }
    return String(value);
  }

  function navigateToTarget(log: ActivityLog) {
    if (!log.target_id) return;
    if (log.target_type === 'task') {
      goto(`/?task_id=${log.target_id}`);
      return;
    }
    if (log.target_type === 'report') {
      goto(`/reports/${log.target_id}`);
    }
  }

  function canDeepLink(log: ActivityLog): boolean {
    return !!log.target_id && (log.target_type === 'task' || log.target_type === 'report');
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
      <div class="mb-4 flex flex-wrap items-end gap-3 border-b border-slate-100 pb-4">
        <div class="min-w-[160px]">
          <label for="filter-user-id" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">User</label>
          <select
            id="filter-user-id"
            bind:value={filterUserId}
            class="w-full rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700"
          >
            <option value="">All</option>
            {#each users as user}
              <option value={user.id}>{user.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="filter-start-date" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Start Date</label>
          <input
            id="filter-start-date"
            type="date"
            bind:value={filterStartDate}
            class="rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700"
          />
        </div>
        <div>
          <label for="filter-end-date" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">End Date</label>
          <input
            id="filter-end-date"
            type="date"
            bind:value={filterEndDate}
            class="rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700"
          />
        </div>
        <button
          class="rounded border border-slate-300 px-3 py-1.5 text-xs font-semibold text-slate-700 disabled:opacity-50"
          on:click={applyFilters}
          disabled={loading || exporting}
        >
          Filter
        </button>
        <button
          class="rounded bg-blue-600 px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
          on:click={exportCsv}
          disabled={loading || exporting}
        >
          {exporting ? 'Exporting...' : 'Export CSV'}
        </button>
      </div>

      {#if errorMessage}
        <div class="mb-4 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
          {errorMessage}
        </div>
      {/if}
      {#if loading}
        <div class="text-center text-slate-400">読み込み中...</div>
      {:else if logs.length === 0}
        <div class="text-center text-slate-400 italic">まだログがありません。</div>
      {:else}
        <div class="overflow-x-auto">
          <table class="table-fixed w-full border-collapse text-xs">
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
                    <div class="flex items-center gap-2">
                      <span class="inline-block px-2 py-0.5 rounded text-[10px] font-bold uppercase {getActionColor(log.action)}">
                        {formatAction(log.action)}
                      </span>
                      {#if canDeepLink(log)}
                        <button
                          class="inline-flex items-center rounded border border-slate-300 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600 hover:bg-slate-50"
                          on:click={() => navigateToTarget(log)}
                          aria-label="Open target"
                          title="Open target"
                        >
                          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07L11.9 5" />
                            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07L12.1 19" />
                          </svg>
                        </button>
                      {/if}
                    </div>
                  </td>
                  <td class="px-3 py-1.5 text-slate-600 break-words whitespace-pre-wrap max-w-[480px]">
                    {#if log.details}
                      {@const parsed = parseDetails(log.details)}
                      {#if parsed.changes.length > 0}
                        <div class="space-y-1">
                          {#each parsed.changes as change}
                            <div class="flex flex-wrap items-center gap-1">
                              <span class="rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600">{formatField(change.field)}</span>
                              <span class="text-red-600">{formatDetailValue(change.old)}</span>
                              <span class="text-slate-400">→</span>
                              <span class="text-emerald-600">{formatDetailValue(change.new)}</span>
                            </div>
                          {/each}
                        </div>
                      {:else if parsed.text}
                        {parsed.text}
                      {:else}
                        <span class="text-slate-400 italic">-</span>
                      {/if}
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
