<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth } from '$lib/auth';
  import type { TaskReportRow, User } from '$lib/types';

  const statusOptions = [
    { value: 'todo', label: '未着手' },
    { value: 'doing', label: '進行中' },
    { value: 'done', label: '完了' }
  ];

  let users: User[] = [];
  let tasks: TaskReportRow[] = [];
  let loading = true;
  let exporting = false;
  let errorMessage = '';

  let filterMemberId = '';
  let filterStartDate = '';
  let filterEndDate = '';
  let selectedStatuses: string[] = ['doing', 'done'];

  function buildQueryParams(): URLSearchParams {
    const params = new URLSearchParams();
    if (filterMemberId) params.set('member_id', filterMemberId);
    if (filterStartDate) params.set('start_date', filterStartDate);
    if (filterEndDate) params.set('end_date', filterEndDate);
    params.set('statuses', selectedStatuses.join(','));
    return params;
  }

  async function fetchUsers() {
    const res = await fetch('http://localhost:3000/api/users', {
      headers: { Authorization: `Bearer ${$auth.token}` }
    });
    if (!res.ok) throw new Error(`メンバー取得に失敗しました (${res.status})`);
    users = await res.json();
  }

  async function fetchTaskReport() {
    loading = true;
    errorMessage = '';
    try {
      const params = buildQueryParams();
      const res = await fetch(`http://localhost:3000/api/tasks/report?${params.toString()}`, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });
      if (!res.ok) {
        const message = await res.text();
        throw new Error(message || `レポート取得に失敗しました (${res.status})`);
      }
      tasks = await res.json();
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : 'レポート取得に失敗しました。';
      tasks = [];
    } finally {
      loading = false;
    }
  }

  function toggleStatus(status: string) {
    if (selectedStatuses.includes(status)) {
      selectedStatuses = selectedStatuses.filter((s) => s !== status);
      return;
    }
    selectedStatuses = [...selectedStatuses, status];
  }

  function formatStatus(status: string): string {
    if (status === 'todo') return '未着手';
    if (status === 'doing') return '進行中';
    if (status === 'done') return '完了';
    return status;
  }

  function formatDateTime(iso: string): string {
    return new Date(iso).toLocaleString('ja-JP');
  }

  function toHours(totalDurationMinutes: number): string {
    return (totalDurationMinutes / 60).toFixed(2);
  }

  async function exportCsv() {
    if (!$auth.token) {
      errorMessage = '認証が必要です。';
      return;
    }

    exporting = true;
    errorMessage = '';

    try {
      const params = buildQueryParams();
      const res = await fetch(`http://localhost:3000/api/tasks/report/export?${params.toString()}`, {
        headers: { Authorization: `Bearer ${$auth.token}` }
      });

      if (!res.ok) {
        const message = await res.text();
        throw new Error(message || `CSVエクスポートに失敗しました (${res.status})`);
      }

      const blob = await res.blob();
      const contentDisposition = res.headers.get('content-disposition') ?? '';
      const filenameMatch = /filename="?([^"]+)"?/.exec(contentDisposition);
      const filename = filenameMatch?.[1] || 'task_report.csv';

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
      errorMessage = e instanceof Error ? e.message : 'CSVエクスポートに失敗しました。';
    } finally {
      exporting = false;
    }
  }

  onMount(async () => {
    if (!$auth.token) {
      goto('/');
      return;
    }
    if ($auth.user?.role !== 'admin') {
      goto('/');
      return;
    }

    try {
      await fetchUsers();
      await fetchTaskReport();
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : '初期化に失敗しました。';
      loading = false;
    }
  });
</script>

<div class="min-h-screen bg-slate-50 flex flex-col font-sans">
  <header class="h-12 px-6 flex items-center gap-4 bg-white border-b border-slate-200 shadow-sm sticky top-0 z-10">
    <button on:click={() => goto('/')} class="p-1.5 -ml-1.5 text-slate-400 hover:text-slate-600 transition-colors" aria-label="ホームへ戻る">
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
    </button>
    <h1 class="text-sm font-black text-slate-800 tracking-tight uppercase">管理者タスクレポート</h1>
  </header>

  <main class="max-w-7xl w-full mx-auto p-4 md:p-6 flex-1">
    <div class="bg-white rounded-2xl shadow-sm border border-slate-200 p-4 md:p-5">
      <div class="mb-4 border-b border-slate-100 pb-4">
        <h2 class="text-xs font-bold text-slate-700 mb-3">絞り込み</h2>
        <div class="flex flex-wrap items-end gap-3">
          <div class="min-w-[180px]">
            <label for="member-id" class="mb-1 block text-[10px] font-bold text-slate-500">メンバー</label>
            <select id="member-id" bind:value={filterMemberId} class="w-full rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700">
              <option value="">全員</option>
              {#each users as user}
                <option value={user.id}>{user.name}</option>
              {/each}
            </select>
          </div>

          <div>
            <label for="start-date" class="mb-1 block text-[10px] font-bold text-slate-500">開始日</label>
            <input id="start-date" type="date" bind:value={filterStartDate} class="rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700" />
          </div>

          <div>
            <label for="end-date" class="mb-1 block text-[10px] font-bold text-slate-500">終了日</label>
            <input id="end-date" type="date" bind:value={filterEndDate} class="rounded border border-slate-300 px-2 py-1.5 text-xs text-slate-700" />
          </div>

          <div>
            <p class="mb-1 block text-[10px] font-bold text-slate-500">ステータス</p>
            <div class="flex flex-wrap gap-2">
              {#each statusOptions as option}
                <label class="inline-flex items-center gap-1 text-xs text-slate-700">
                  <input
                    type="checkbox"
                    checked={selectedStatuses.includes(option.value)}
                    on:change={() => toggleStatus(option.value)}
                  />
                  <span>{option.label}</span>
                </label>
              {/each}
            </div>
          </div>

          <button
            class="rounded border border-slate-300 px-3 py-1.5 text-xs font-semibold text-slate-700 disabled:opacity-50"
            on:click={fetchTaskReport}
            disabled={loading || exporting}
          >
            検索
          </button>

          <button
            class="rounded bg-blue-600 px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
            on:click={exportCsv}
            disabled={loading || exporting}
          >
            {exporting ? '出力中...' : 'CSV出力'}
          </button>
        </div>
      </div>

      {#if errorMessage}
        <div class="mb-4 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
          {errorMessage}
        </div>
      {/if}

      {#if loading}
        <div class="text-center text-slate-400">読み込み中...</div>
      {:else if tasks.length === 0}
        <div class="text-center text-slate-400 italic">対象データがありません。</div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-xs">
            <thead>
              <tr class="border-b border-slate-200 bg-slate-50">
                <th class="px-3 py-2 text-left font-bold text-slate-600">担当者</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">タスク名</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">ステータス</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">進捗</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">タグ</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">開始</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">終了</th>
                <th class="px-3 py-2 text-left font-bold text-slate-600">Total Hours</th>
              </tr>
            </thead>
            <tbody>
              {#each tasks as task}
                <tr class="border-b border-slate-100">
                  <td class="px-3 py-2 text-slate-700">{task.user_name}</td>
                  <td class="px-3 py-2 text-slate-800 font-semibold">{task.title}</td>
                  <td class="px-3 py-2 text-slate-700">{formatStatus(task.status)}</td>
                  <td class="px-3 py-2 text-slate-700">{task.progress_rate}%</td>
                  <td class="px-3 py-2 text-slate-700">{task.tags?.join(', ') || '-'}</td>
                  <td class="px-3 py-2 text-slate-700 whitespace-nowrap">{formatDateTime(task.start_at)}</td>
                  <td class="px-3 py-2 text-slate-700 whitespace-nowrap">{formatDateTime(task.end_at)}</td>
                  <td class="px-3 py-2 text-slate-700 whitespace-nowrap">{toHours(task.total_duration_minutes)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  </main>
</div>
