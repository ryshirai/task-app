<script lang="ts">
  import { apiFetch } from '$lib/api';
  import { createEventDispatcher } from 'svelte';
  import { auth, logout } from '$lib/auth';
  import type { Task, User } from '$lib/types';

  export let selectedDate: string;
  export let selectedGroupId: number | null = null;
  export let filterText = '';
  export let selectedTimelineMemberId: number | null = null;

  const dispatch = createEventDispatcher<{ editTask: Task }>();

  const statusOrder: Task['status'][] = ['doing', 'todo', 'done'];
  const statusLabels: Record<Task['status'], string> = {
    doing: '進行中',
    todo: '未着手',
    done: '完了'
  };
  const statusDotClass: Record<Task['status'], string> = {
    doing: 'bg-blue-500',
    todo: 'bg-yellow-400',
    done: 'bg-slate-400'
  };

  let tasks: Task[] = [];
  let memberNameMap: Record<number, string> = {};
  let loading = false;
  let error: string | null = null;
  let activeRequestKey = '';
  let currentAbortController: AbortController | null = null;
  let updatingTaskIds = new Set<number>();
  let manualMemberIdFilter: number | null = null;
  let syncWithTimelineSelection = true;

  $: effectiveMemberId = syncWithTimelineSelection ? selectedTimelineMemberId : manualMemberIdFilter;
  $: requestKey = `${selectedDate}|${selectedGroupId ?? 'all'}|${filterText}|${effectiveMemberId ?? 'all'}`;

  $: groupedTasks = statusOrder
    .map((status) => ({
      status,
      label: statusLabels[status],
      items: tasks.filter((task) => task.status === status)
    }))
    .filter((group) => group.items.length > 0);

  $: totalTasks = tasks.length;
  $: overdueTasks = tasks.filter((task) => isOverdue(task));
  $: completedPlannedMinutes = tasks
    .filter((task) => task.status === 'done')
    .reduce((acc, task) => acc + Number(task.total_duration_minutes || 0), 0);
  $: completedTasks = tasks.filter((task) => task.status === 'done').length;
  $: progressPercent = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  $: selectedDateLabel = selectedDate
    ? new Date(`${selectedDate}T00:00:00`).toLocaleDateString('ja-JP', { month: 'numeric', day: 'numeric', weekday: 'short' })
    : '';

  $: if ($auth.token && selectedDate && requestKey !== activeRequestKey) {
    activeRequestKey = requestKey;
    void fetchTodayFocus();
  }

  function formatMinutes(totalMinutes: number) {
    const minutes = Math.max(0, Math.round(Number(totalMinutes) || 0));
    const hours = Math.floor(minutes / 60);
    const remaining = minutes % 60;
    if (hours === 0) return `${remaining}分`;
    if (remaining === 0) return `${hours}時間`;
    return `${hours}時間${remaining}分`;
  }

  function openTask(task: Task) {
    dispatch('editTask', task);
  }

  function isOverdue(task: Task) {
    if (task.status === 'done') return false;
    const endAt = Date.parse(task.end_at || '');
    if (Number.isNaN(endAt)) return false;
    return endAt < Date.now();
  }

  function formatDateTime(value: string | null | undefined) {
    if (!value) return '-';
    const date = new Date(value);
    if (Number.isNaN(date.getTime()) || date.getTime() <= 86400000) return '-';
    return date.toLocaleString('ja-JP', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  function getTaskDescription(task: Task) {
    const legacyDescription = (task as Task & { task_description?: string | null }).task_description;
    return task.description ?? legacyDescription ?? null;
  }

  function setUpdatingTask(taskId: number, isUpdating: boolean) {
    const next = new Set(updatingTaskIds);
    if (isUpdating) next.add(taskId);
    else next.delete(taskId);
    updatingTaskIds = next;
  }

  async function updateTaskStatus(task: Task, status: Task['status']) {
    if (!$auth.token || task.status === status || updatingTaskIds.has(task.id)) return;
    setUpdatingTask(task.id, true);
    try {
      const res = await apiFetch(`/api/tasks/${task.id}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${$auth.token}`
        },
        body: JSON.stringify({ status })
      });

      if (res.status === 401) {
        logout();
        return;
      }
      if (!res.ok) {
        throw new Error(`Failed to update task status: ${res.status}`);
      }

      const updatedTask: Task = await res.json();
      tasks = tasks.map((t) => (t.id === updatedTask.id ? { ...t, ...updatedTask } : t));
    } catch (e) {
      console.error('Failed to update task status:', e);
      alert('ステータス更新に失敗しました。');
    } finally {
      setUpdatingTask(task.id, false);
    }
  }

  async function fetchTodayFocus() {
    if (!$auth.token) return;
    currentAbortController?.abort();
    currentAbortController = new AbortController();

    loading = true;
    try {
      const taskParams = new URLSearchParams({ date: selectedDate });
      if (selectedGroupId) taskParams.set('group_id', String(selectedGroupId));
      if (filterText.trim()) taskParams.set('q', filterText.trim());
      if (effectiveMemberId) taskParams.set('member_id', String(effectiveMemberId));

      const [taskRes, usersRes] = await Promise.all([
        apiFetch(`/api/tasks?${taskParams.toString()}`, {
          headers: { Authorization: `Bearer ${$auth.token}` },
          signal: currentAbortController.signal
        }),
        apiFetch(`/api/users?date=${selectedDate}`, {
          headers: { Authorization: `Bearer ${$auth.token}` },
          signal: currentAbortController.signal
        })
      ]);

      if (taskRes.status === 401 || usersRes.status === 401) {
        logout();
        return;
      }
      if (!taskRes.ok) {
        throw new Error(`Failed to fetch tasks: ${taskRes.status}`);
      }

      tasks = await taskRes.json();

      if (usersRes.ok) {
        const users: User[] = await usersRes.json();
        memberNameMap = Object.fromEntries(users.map((user) => [user.id, user.name]));
      } else {
        memberNameMap = {};
      }

      error = null;
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      console.error('Failed to fetch today focus tasks:', e);
      tasks = [];
      memberNameMap = {};
      error = '本日フォーカスの取得に失敗しました。';
    } finally {
      loading = false;
    }
  }
</script>

<section class="mb-1.5 overflow-hidden rounded-2xl border border-[var(--border-base)] bg-[var(--surface-primary)] shadow-[var(--shadow-elevated)]">
  <div class="border-b border-[var(--border-base)] bg-[color:color-mix(in_srgb,var(--surface-secondary)_82%,transparent)] px-5 py-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <h3 class="text-[14px] font-black uppercase tracking-tight text-[var(--text-primary)]">
          本日フォーカス
        </h3>
        <p class="text-[11px] font-semibold text-[var(--text-muted)]">
          本日（{selectedDateLabel}）のサマリー
        </p>
      </div>
      <div class="flex items-center gap-2">
        <div class="rounded-xl border border-[var(--border-base)] bg-[var(--surface-primary)] px-3 py-2">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">合計タスク</div>
          <div class="mt-0.5 text-[14px] font-black leading-none text-[var(--text-primary)]">{totalTasks}</div>
        </div>
        <div class="rounded-xl border border-[var(--border-base)] bg-[var(--surface-primary)] px-3 py-2">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">完了予定時間</div>
          <div class="mt-0.5 text-[14px] font-black leading-none text-blue-600">{formatMinutes(completedPlannedMinutes)}</div>
        </div>
        <div class="rounded-xl border border-red-300/60 bg-red-100/50 px-3 py-2 dark:border-red-800/70 dark:bg-red-950/30">
          <div class="text-[10px] font-semibold uppercase tracking-wide text-red-600 dark:text-red-300">遅延</div>
          <div class="mt-0.5 text-[14px] font-black leading-none text-red-700 dark:text-red-200">{overdueTasks.length}</div>
        </div>
      </div>
    </div>

    <div class="mt-3.5 space-y-2">
      <div>
        <div class="mb-1.5 flex items-center justify-between text-[10px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          <span>進捗</span>
          <span>{completedTasks} / {totalTasks}（{progressPercent}%）</span>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-[color:color-mix(in_srgb,var(--surface-primary)_88%,var(--border-base)_12%)]">
          <div
            class="h-full rounded-full bg-gradient-to-r from-blue-500 to-teal-400 transition-all duration-300"
            style={`width: ${progressPercent}%`}
          ></div>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2">
      <label class="inline-flex items-center gap-1.5 text-[10px] font-semibold text-[var(--text-primary)]">
        <input type="checkbox" bind:checked={syncWithTimelineSelection} class="h-3.5 w-3.5 accent-blue-600" />
        タイムライン選択者に連動
      </label>
      {#if !syncWithTimelineSelection}
        <select
          class="form-control rounded-lg px-2 py-1 text-[11px] font-semibold focus:ring-2"
          value={manualMemberIdFilter ? String(manualMemberIdFilter) : ''}
          on:change={(event) => {
            const value = (event.currentTarget as HTMLSelectElement).value;
            manualMemberIdFilter = value ? Number(value) : null;
          }}
        >
          <option value="">担当者: 全員</option>
          {#each Object.entries(memberNameMap) as [memberId, memberName]}
            <option value={memberId}>担当者: {memberName}</option>
          {/each}
        </select>
      {:else}
        <div class="rounded-lg border border-blue-300/70 bg-blue-100/50 px-2 py-1 text-[10px] font-semibold text-blue-700 dark:border-blue-800 dark:bg-blue-950/30 dark:text-blue-200">
          {#if selectedTimelineMemberId}
            対象: {memberNameMap[selectedTimelineMemberId] || `Member #${selectedTimelineMemberId}`}
          {:else}
            タイムライン未選択（全員表示）
          {/if}
        </div>
      {/if}
      </div>
    </div>
  </div>

  <div class="px-5 py-4">
    {#if loading}
      <div class="text-[12px] font-semibold text-[var(--text-muted)]">読み込み中...</div>
    {:else if error}
      <div class="text-[12px] font-semibold text-red-600 dark:text-red-300">{error}</div>
    {:else if groupedTasks.length === 0}
      <div class="rounded-xl border border-dashed border-[var(--border-base)] bg-[var(--surface-secondary)] px-3 py-5 text-center text-[12px] font-semibold text-[var(--text-muted)]">
        該当するタスクはありません。
      </div>
    {:else}
      <div class="grid gap-3 md:grid-cols-3">
        {#each groupedTasks as group}
          <div class="rounded-2xl border border-[var(--border-base)] bg-[var(--surface-primary)]">
            <div class="flex items-center gap-1.5 border-b border-[var(--border-base)] px-3 py-2.5">
              <span class="h-2 w-2 rounded-full {statusDotClass[group.status]}"></span>
              <h4 class="text-[11px] font-black uppercase tracking-tight text-[var(--text-primary)]">
                {group.label}
              </h4>
              <span class="ml-auto text-[10px] font-semibold text-[var(--text-muted)]">{group.items.length}</span>
            </div>
            <div class="scrollbar-hide max-h-56 overflow-y-auto p-2">
              <div class="space-y-2">
                {#each group.items as task (task.id)}
                  <article class="w-full rounded-xl border px-3 py-2.5 text-left transition-all {isOverdue(task) ? 'border-red-300/70 bg-red-100/45 dark:border-red-800 dark:bg-red-950/35' : 'border-[var(--border-base)] bg-[var(--surface-secondary)] hover:border-[var(--border-strong)]'}">
                    <button type="button" class="w-full text-left" on:click={() => openTask(task)}>
                      <div class="flex items-start justify-between gap-2">
                        <div class="line-clamp-2 text-[12px] font-extrabold text-[var(--text-primary)]">{task.title}</div>
                        {#if isOverdue(task)}
                          <span class="shrink-0 rounded border border-red-300/70 bg-red-100/70 px-1.5 py-0.5 text-[9px] font-black text-red-700 dark:border-red-800 dark:bg-red-950/45 dark:text-red-200">遅延</span>
                        {/if}
                      </div>
                      {#if getTaskDescription(task)}
                        <div class="mt-1 line-clamp-2 text-[10px] font-medium text-[var(--text-muted)]">
                          {getTaskDescription(task)}
                        </div>
                      {/if}
                    </button>
                    <div class="mt-1 text-[10px] font-semibold text-[var(--text-muted)]">
                      担当: {memberNameMap[task.member_id] || `Member #${task.member_id}`}
                    </div>
                    {#if isOverdue(task)}
                      <div class="mt-0.5 text-[10px] font-semibold text-red-700 dark:text-red-300">
                        予定終了: {formatDateTime(task.end_at)}
                      </div>
                    {/if}
                    <div class="mt-1 flex items-center justify-between gap-2">
                      <div class="flex min-w-0 flex-wrap gap-1">
                        {#if task.tags && task.tags.length > 0}
                          {#each task.tags.slice(0, 3) as tag}
                            <span class="max-w-24 truncate rounded-md border border-[var(--border-base)] bg-[var(--surface-primary)] px-1.5 py-0.5 text-[10px] font-semibold text-[var(--text-muted)]">
                              #{tag}
                            </span>
                          {/each}
                        {:else}
                          <span class="text-[10px] text-[var(--text-muted)]/80">タグなし</span>
                        {/if}
                      </div>
                      <span class="shrink-0 text-[10px] font-black text-[var(--text-muted)]">
                        予定 {formatMinutes(task.total_duration_minutes)}
                      </span>
                    </div>
                    <div class="mt-2 flex items-center gap-1.5">
                      <button
                        type="button"
                        disabled={task.status === 'doing' || updatingTaskIds.has(task.id)}
                        class="rounded-lg border px-2.5 py-1 text-[10px] font-black transition-all disabled:cursor-not-allowed disabled:opacity-50 {task.status === 'doing' ? 'border-blue-600 bg-blue-600 text-white' : 'border-blue-300/60 bg-blue-100/45 text-blue-700 hover:brightness-105 dark:border-blue-800 dark:bg-blue-950/40 dark:text-blue-200'}"
                        on:click={() => updateTaskStatus(task, 'doing')}
                      >
                        進行中
                      </button>
                      <button
                        type="button"
                        disabled={task.status === 'done' || updatingTaskIds.has(task.id)}
                        class="rounded-lg border px-2.5 py-1 text-[10px] font-black transition-all disabled:cursor-not-allowed disabled:opacity-50 {task.status === 'done' ? 'border-emerald-600 bg-emerald-600 text-white' : 'border-emerald-300/70 bg-emerald-100/45 text-emerald-700 hover:brightness-105 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-200'}"
                        on:click={() => updateTaskStatus(task, 'done')}
                      >
                        完了
                      </button>
                      {#if updatingTaskIds.has(task.id)}
                        <span class="text-[10px] font-semibold text-[var(--text-muted)]">更新中...</span>
                      {/if}
                    </div>
                  </article>
                {/each}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
