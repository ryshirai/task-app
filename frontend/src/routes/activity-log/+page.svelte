<script lang="ts">
  import { apiFetch } from '$lib/api';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth } from '$lib/auth';
  import type { ActivityLog, User } from '$lib/types';

  type LogChange = {
    field: string;
    old?: unknown;
    new?: unknown;
  };

  type DetailRow = {
    label: string;
    old?: unknown;
    new?: unknown;
    value?: unknown;
  };

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

  const fieldMap: Record<string, string> = {
    title: 'タイトル',
    status: 'ステータス',
    progress_rate: '進捗率',
    description: '詳細',
    start_at: '開始日時',
    end_at: '終了日時',
    duration_minutes: '所要時間(分)',
    role: 'ロール'
  };

  const statusMap: Record<string, string> = {
    todo: '未着手',
    doing: '進行中',
    done: '完了'
  };

  const roleMap: Record<string, string> = {
    admin: '管理者',
    user: '一般メンバー'
  };

  const isoDateTimePattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d{1,6})?)?(?:Z|[+-]\d{2}:\d{2})$/;

  function localizeErrorMessage(message: string, fallback: string): string {
    const normalized = message.trim();
    if (!normalized) return fallback;

    const translationMap: Array<[RegExp, string]> = [
      [/^Failed to fetch users\b/i, 'ユーザーの取得に失敗しました'],
      [/^Failed to fetch logs\b/i, '操作履歴の取得に失敗しました'],
      [/^Authentication required\b/i, '認証が必要です'],
      [/^Failed to export CSV\b/i, 'CSVのエクスポートに失敗しました']
    ];

    for (const [pattern, translated] of translationMap) {
      if (pattern.test(normalized)) {
        return normalized.replace(pattern, translated);
      }
    }

    return normalized;
  }

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
      const res = await apiFetch('/api/users', {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error(`ユーザーの取得に失敗しました (${res.status})`);
      users = await res.json();
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? localizeErrorMessage(e.message, 'ユーザーの取得に失敗しました') : 'ユーザーの取得に失敗しました';
    }
  }

  async function fetchLogs() {
    loading = true;
    errorMessage = '';
    try {
      const params = buildQueryParams(true);
      const res = await apiFetch(`/api/logs?${params.toString()}`, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error(`操作履歴の取得に失敗しました (${res.status})`);
      const data = await res.json();
      logs = data.items;
      totalPages = data.total_pages;
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? localizeErrorMessage(e.message, '操作履歴の取得に失敗しました') : '操作履歴の取得に失敗しました';
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
      errorMessage = '認証が必要です';
      return;
    }

    exporting = true;
    errorMessage = '';
    try {
      const params = buildQueryParams(false);
      const url = `/api/logs/export?${params.toString()}`;
      const res = await apiFetch(url, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });

      if (!res.ok) {
        const message = await res.text();
        throw new Error(message || `CSVのエクスポートに失敗しました (${res.status})`);
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
      errorMessage = e instanceof Error ? localizeErrorMessage(e.message, 'CSVのエクスポートに失敗しました') : 'CSVのエクスポートに失敗しました';
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
      report_updated: '日報を更新',
      time_log_added: '作業ログを追加',
      time_log_updated: '作業ログを更新',
      time_log_deleted: '作業ログを削除',
      password_changed: 'パスワードを変更',
      user_role_updated: 'ロールを変更',
      update_email: 'メールアドレスを変更',
      login: 'ログイン'
    };
    return map[action] || action;
  }

  function getActionColor(action: string) {
    if (action.includes('created')) return 'bg-emerald-500/15 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-300';
    if (action.includes('deleted')) return 'bg-red-500/15 text-red-700 dark:bg-red-500/20 dark:text-red-300';
    return 'bg-blue-500/15 text-blue-700 dark:bg-blue-500/20 dark:text-blue-300';
  }

  function parseDetails(details?: string): { rows: DetailRow[]; text: string | null } {
    if (!details) return { rows: [], text: null };

    const raw = details.trim();
    if (!raw) return { rows: [], text: null };

    try {
      const parsed = JSON.parse(raw) as unknown;

      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        const asRecord = parsed as Record<string, unknown>;

        if (Array.isArray(asRecord.changes)) {
          const rows = asRecord.changes
            .filter((item): item is LogChange => {
              return (
                !!item &&
                typeof item === 'object' &&
                'field' in item &&
                typeof (item as { field: unknown }).field === 'string'
              );
            })
            .map((item) => ({
              label: formatField(item.field),
              old: item.old,
              new: item.new
            }));

          if (rows.length > 0) return { rows, text: null };
        }

        const rows = Object.entries(asRecord)
          .filter(([key]) => key !== 'changes')
          .map(([key, value]) => ({
            label: formatField(key),
            value
          }));

        if (rows.length > 0) return { rows, text: null };
      }
    } catch {
      // plain text fallback
    }

    const pairs = raw
      .split(/[\n,;]+/)
      .map((part) => part.trim())
      .filter((part) => part.length > 0);

    if (pairs.length > 0 && pairs.every((part) => part.includes('='))) {
      const rows = pairs
        .map((part) => {
          const index = part.indexOf('=');
          const key = part.slice(0, index).trim();
          const value = part.slice(index + 1).trim();
          if (!key) return null;
          return {
            label: formatField(key),
            value: value || '-'
          } satisfies DetailRow;
        })
        .filter((row): row is DetailRow => row !== null);

      if (rows.length > 0) return { rows, text: null };
    }

    return { rows: [], text: raw };
  }

  function formatField(field: string): string {
    return fieldMap[field] || `項目 (${field})`;
  }

  function formatIsoDateTime(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;

    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hour = String(date.getHours()).padStart(2, '0');
    const minute = String(date.getMinutes()).padStart(2, '0');
    return `${month}/${day} ${hour}:${minute}`;
  }

  function formatDetailValue(value: unknown): string {
    if (value === null || value === undefined) return '-';

    if (Array.isArray(value)) {
      if (value.length === 0) return '-';
      return value.map((item) => formatDetailValue(item)).join(', ');
    }

    if (typeof value === 'string') {
      const normalized = value.trim();
      if (!normalized) return '-';

      const lowered = normalized.toLowerCase();
      if (statusMap[lowered]) return statusMap[lowered];
      if (roleMap[lowered]) return roleMap[lowered];
      if (isoDateTimePattern.test(normalized)) return formatIsoDateTime(normalized);

      return normalized;
    }

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

{#if !$auth.initialized}
  <div class="min-h-screen bg-surface-muted text-text-muted flex items-center justify-center font-semibold">読み込み中...</div>
{:else}
<div class="min-h-screen flex flex-col bg-surface-muted font-sans text-text-base">
  <header class="sticky top-0 z-10 flex h-12 items-center gap-4 border-b border-border-base bg-surface-primary px-6 shadow-sm">
    <button on:click={() => goto('/')} class="-ml-1.5 p-1.5 text-text-muted transition-colors hover:text-text-base" aria-label="ホームへ戻る">
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
    </button>
    <h1 class="text-sm font-black uppercase tracking-tight text-text-base">操作履歴</h1>
  </header>

  <main class="max-w-4xl w-full mx-auto p-4 md:p-6 flex-1">
    <div class="rounded-2xl border border-border-base bg-surface-primary p-4 shadow-sm md:p-5">
      <div class="mb-4 flex flex-wrap items-end gap-3 border-b border-border-base pb-4">
        <div class="min-w-[160px]">
          <label for="filter-user-id" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-text-muted">ユーザー</label>
          <select
            id="filter-user-id"
            bind:value={filterUserId}
            class="w-full rounded border border-border-base bg-surface-secondary px-2 py-1.5 text-xs text-text-base"
          >
            <option value="">全員</option>
            {#each users as user}
              <option value={user.id}>{user.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="filter-start-date" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-text-muted">開始日</label>
          <input
            id="filter-start-date"
            type="date"
            bind:value={filterStartDate}
            class="rounded border border-border-base bg-surface-secondary px-2 py-1.5 text-xs text-text-base"
          />
        </div>
        <div>
          <label for="filter-end-date" class="mb-1 block text-[10px] font-bold uppercase tracking-wider text-text-muted">終了日</label>
          <input
            id="filter-end-date"
            type="date"
            bind:value={filterEndDate}
            class="rounded border border-border-base bg-surface-secondary px-2 py-1.5 text-xs text-text-base"
          />
        </div>
        <button
          class="rounded border border-border-base bg-surface-secondary px-3 py-1.5 text-xs font-semibold text-text-base disabled:opacity-50"
          on:click={applyFilters}
          disabled={loading || exporting}
        >
          適用
        </button>
        <button
          class="rounded bg-blue-600 px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
          on:click={exportCsv}
          disabled={loading || exporting}
        >
          {exporting ? '書き出し中...' : 'CSVエクスポート'}
        </button>
      </div>

      {#if errorMessage}
        <div class="mb-4 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
          {errorMessage}
        </div>
      {/if}
      {#if loading}
        <div class="text-center text-text-muted">読み込み中...</div>
      {:else if logs.length === 0}
        <div class="text-center italic text-text-muted">まだログがありません。</div>
      {:else}
        <div class="overflow-x-auto">
          <table class="table-fixed w-full border-collapse text-xs">
            <thead>
              <tr class="border-b border-border-base bg-surface-secondary">
                <th class="px-3 py-1.5 text-left font-bold text-text-muted">日時</th>
                <th class="px-3 py-1.5 text-left font-bold text-text-muted">ユーザー</th>
                <th class="px-3 py-1.5 text-left font-bold text-text-muted">操作</th>
                <th class="px-3 py-1.5 text-left font-bold text-text-muted">詳細</th>
              </tr>
            </thead>
            <tbody>
              {#each logs as log}
                <tr class="align-top border-b border-border-base">
                  <td class="whitespace-nowrap px-3 py-1.5 font-mono text-text-muted">
                    {new Date(log.created_at).toLocaleString('ja-JP')}
                  </td>
                  <td class="whitespace-nowrap px-3 py-1.5 font-semibold text-text-base">
                    {log.user_name}
                  </td>
                  <td class="px-3 py-1.5">
                    <div class="flex items-center gap-2">
                      <span class="inline-block px-2 py-0.5 rounded text-[10px] font-bold uppercase {getActionColor(log.action)}">
                        {formatAction(log.action)}
                      </span>
                      {#if canDeepLink(log)}
                        <button
                          class="inline-flex items-center rounded border border-border-base px-1.5 py-0.5 text-[10px] font-semibold text-text-muted hover:bg-surface-secondary"
                          on:click={() => navigateToTarget(log)}
                          aria-label="詳細を見る"
                          title="詳細を見る"
                        >
                          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07L11.9 5" />
                            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07L12.1 19" />
                          </svg>
                        </button>
                      {/if}
                    </div>
                  </td>
                  <td class="max-w-[480px] break-words whitespace-pre-wrap px-3 py-2 text-text-muted">
                    {#if log.details}
                      {@const parsed = parseDetails(log.details)}
                      {#if parsed.rows.length > 0}
                        <div class="space-y-2 py-0.5">
                          {#each parsed.rows as row}
                            <div class="flex items-center gap-2.5">
                              <span class="shrink-0 rounded-md bg-surface-secondary px-1.5 py-0.5 text-[9px] font-black text-text-muted uppercase tracking-tighter border border-border-base">
                                {row.label}
                              </span>
                              <div class="flex-1 min-w-0">
                                {#if row.old !== undefined || row.new !== undefined}
                                  <div class="flex flex-wrap items-center gap-x-2 text-[11px]">
                                    <span class="truncate max-w-[120px] text-text-muted/60 line-through decoration-text-muted/30">{formatDetailValue(row.old)}</span>
                                    <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="text-text-muted/30"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
                                    <span class="font-bold text-emerald-600 drop-shadow-sm">{formatDetailValue(row.new)}</span>
                                  </div>
                                {:else}
                                  <div class="text-[11px] font-bold text-text-base/80">{formatDetailValue(row.value)}</div>
                                {/if}
                              </div>
                            </div>
                          {/each}
                        </div>
                      {:else if parsed.text}
                        <span class="text-[11px]">{parsed.text}</span>
                      {:else}
                        <span class="italic text-text-muted">-</span>
                      {/if}
                    {:else}
                      <span class="italic text-text-muted">-</span>
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
          class="rounded border border-border-base bg-surface-secondary px-3 py-1.5 text-text-base disabled:cursor-not-allowed disabled:opacity-40"
          on:click={() => changePage(currentPage - 1)}
          disabled={loading || currentPage <= 1}
        >
          前へ
        </button>
        <span class="font-mono text-text-muted">{currentPage} / {totalPages} ページ</span>
        <button
          class="rounded border border-border-base bg-surface-secondary px-3 py-1.5 text-text-base disabled:cursor-not-allowed disabled:opacity-40"
          on:click={() => changePage(currentPage + 1)}
          disabled={loading || currentPage >= totalPages}
        >
          次へ
        </button>
      </div>
    </div>
  </main>
</div>
{/if}
