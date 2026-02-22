<script lang="ts">
  import TimelineContainer from '$lib/components/TimelineContainer.svelte';
  import TaskForm from '$lib/components/TaskForm.svelte';
  import TaskEditModal from '$lib/components/TaskEditModal.svelte';
  import UserManagementModal from '$lib/components/UserManagementModal.svelte';
  import DisplayGroupModal from '$lib/components/DisplayGroupModal.svelte';
  import ProfileModal from '$lib/components/ProfileModal.svelte';
  import Login from '$lib/components/Login.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { type User, type Task, type TaskTimeLog, type Notification as AppNotification, type PaginatedNotifications, type DisplayGroup } from '$lib/types';
  import { auth, logout } from '$lib/auth';
  import { toLocalISOString, getTodayJSTString, getJSTDateString, formatDateTime } from '$lib/utils';
  import { upsertTimeLog } from '$lib/taskUtils';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { page } from '$app/stores';

  let users: User[] = [];
  let displayGroups: DisplayGroup[] = [];
  let selectedGroupId: number | null = (browser ? Number(localStorage.getItem('glanceflow_selected_group')) : null) || null;
  let loading = true;
  let error: string | null = null;
  let editingTask: TaskTimeLog | null = null;
  let showUserManagement = false;
  let showDisplayGroupSettings = false;
  let showProfile = false;
  let taskFormSelection: { member_id: number; start: Date; end: Date } | null = null;
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let filterText = '';
  let selectedDate = (browser ? localStorage.getItem('glanceflow_selected_date') : null) || getTodayJSTString();
  let observedTaskIdParam: string | null = null;
  let deepLinkHandled = false;
  let notifications: AppNotification[] = [];
  let notificationsLoading = false;
  let notificationsError: string | null = null;
  let showNotifications = false;
  let notificationPollInterval: ReturnType<typeof setInterval> | null = null;
  let notificationMenu: HTMLDivElement | null = null;
  let navMenu: HTMLDivElement | null = null;
  let userMenu: HTMLDivElement | null = null;
  let showNavDropdown = false;
  let showUserDropdown = false;

  $: if (browser) {
    if (selectedGroupId) localStorage.setItem('glanceflow_selected_group', selectedGroupId.toString());
    else localStorage.removeItem('glanceflow_selected_group');
  }

  $: if (browser && selectedDate) {
    localStorage.setItem('glanceflow_selected_date', selectedDate);
  }

  $: baseDate = new Date(selectedDate + 'T00:00:00+09:00');
  $: taskIdParam = $page.url.searchParams.get('task_id');
  $: unreadNotificationCount = notifications.filter(n => !n.is_read).length;

  $: if (taskIdParam !== observedTaskIdParam) {
    observedTaskIdParam = taskIdParam;
    deepLinkHandled = false;
  }

  $: filteredUsers = users.map(u => ({
    ...u,
    time_logs: (u.time_logs || []).filter(t => 
      (t.task_title || '').toLowerCase().includes(filterText.toLowerCase()) ||
      (t.task_tags || []).some(tag => tag.toLowerCase().includes(filterText.toLowerCase()))
    )
  })).filter(u => {
    if (!selectedGroupId) return true;
    const group = displayGroups.find(g => g.id === selectedGroupId);
    return group ? group.member_ids.includes(u.id) : true;
  }).sort((a, b) => {
    if (a.id === $auth.user?.id) return -1;
    if (b.id === $auth.user?.id) return 1;
    return 0;
  });

  async function fetchUsers(silent = false) {
    if (!$auth.token || editingTask || showUserManagement || showProfile || showDisplayGroupSettings) return;

    try {
      const res = await fetch(`http://localhost:3000/api/users?date=${selectedDate}`, {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) {
        if (res.status === 401) {
            logout();
            return;
        }
        throw new Error(`Failed to fetch users: ${res.statusText}`);
      }
      const data = await res.json();
      users = data;
      error = null;
    } catch (e) {
      console.error('Backend connection failed:', e);
      if (!silent) {
        error = 'データの取得に失敗しました。バックエンドが起動しているか確認してください。';
      }
    } finally {
      loading = false;
    }
  }

  async function fetchDisplayGroups() {
    if (!$auth.token) return;
    try {
      const res = await fetch('http://localhost:3000/api/display-groups', {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (res.ok) {
        displayGroups = await res.json();
      }
    } catch (e) {
      console.error('Failed to fetch display groups:', e);
    }
  }

  async function fetchNotifications(silent = false) {
    if (!$auth.token) return;

    if (!silent) notificationsLoading = true;

    try {
      const res = await fetch('http://localhost:3000/api/notifications?page=1&per_page=20', {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });

      if (!res.ok) {
        if (res.status === 401) {
          logout();
          return;
        }
        throw new Error(`Failed to fetch notifications: ${res.statusText}`);
      }

      const data: PaginatedNotifications = await res.json();
      notifications = data.items;
      notificationsError = null;
    } catch (e) {
      console.error('Failed to fetch notifications:', e);
      if (!silent) {
        notificationsError = '通知の取得に失敗しました。';
      }
    } finally {
      notificationsLoading = false;
    }
  }

  async function markNotificationAsRead(notificationId: number) {
    if (!$auth.token) return;
    try {
      const res = await fetch(`http://localhost:3000/api/notifications/${notificationId}/read`, {
        method: 'PATCH',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) {
        if (res.status === 401) {
          logout();
          return;
        }
        throw new Error('Failed to mark notification as read');
      }
      notifications = notifications.map(n => n.id === notificationId ? { ...n, is_read: true } : n);
    } catch (e) {
      console.error('Error marking notification as read:', e);
    }
  }

  async function markAllNotificationsAsRead() {
    if (!$auth.token) return;
    try {
      const res = await fetch('http://localhost:3000/api/notifications/read-all', {
        method: 'PATCH',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) {
        if (res.status === 401) {
          logout();
          return;
        }
        throw new Error('Failed to mark all notifications as read');
      }
      notifications = notifications.map(n => ({ ...n, is_read: true }));
    } catch (e) {
      console.error('Error marking all notifications as read:', e);
      alert('通知の更新に失敗しました。');
    }
  }

  function getNotificationLink(notification: AppNotification): string | null {
    if (!notification.target_id) return null;
    if (notification.target_type === 'task') return `/?task_id=${notification.target_id}`;
    if (notification.target_type === 'report') return `/reports/${notification.target_id}`;
    return null;
  }

  async function handleNotificationClick(notification: AppNotification) {
    if (!notification.is_read) {
      await markNotificationAsRead(notification.id);
    }
    showNotifications = false;
    const link = getNotificationLink(notification);
    if (link) {
      goto(link);
    }
  }

  function handleDocumentClick(event: MouseEvent) {
    const target = event.target as Node;
    if (showNotifications && notificationMenu && !notificationMenu.contains(target)) {
      showNotifications = false;
    }
    if (showNavDropdown && navMenu && !navMenu.contains(target)) {
      showNavDropdown = false;
    }
    if (showUserDropdown && userMenu && !userMenu.contains(target)) {
      showUserDropdown = false;
    }
  }

  function removeTimeLogById(timeLogId: number) {
    users = users.map((user) => ({
      ...user,
      time_logs: (user.time_logs || []).filter((timeLog) => timeLog.id !== timeLogId)
    }));
  }

  onMount(() => {
    if (browser) {
      document.addEventListener('click', handleDocumentClick);
    }
    if ($auth.token) {
        fetchUsers();
        fetchNotifications();
        fetchDisplayGroups();
        pollInterval = setInterval(() => fetchUsers(true), 30000);
        notificationPollInterval = setInterval(() => fetchNotifications(true), 30000);
    } else {
        loading = false;
    }
  });

  onDestroy(() => {
    if (browser) {
      document.removeEventListener('click', handleDocumentClick);
    }
    if (pollInterval) clearInterval(pollInterval);
    if (notificationPollInterval) clearInterval(notificationPollInterval);
  });

  $: if ($auth.token && !pollInterval) {
      fetchUsers();
      fetchNotifications();
      fetchDisplayGroups();
      pollInterval = setInterval(() => fetchUsers(true), 30000);
      notificationPollInterval = setInterval(() => fetchNotifications(true), 30000);
  }

  $: if ($auth.token && !notificationPollInterval) {
      fetchNotifications(true);
      notificationPollInterval = setInterval(() => fetchNotifications(true), 30000);
  }

  $: if (!$auth.token) {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
      if (notificationPollInterval) {
        clearInterval(notificationPollInterval);
        notificationPollInterval = null;
      }
      notifications = [];
  }

  $: if (selectedDate && $auth.token) {
      fetchUsers();
  }

  $: if (!deepLinkHandled && !loading) {
    deepLinkHandled = true;
    if (taskIdParam) {
      const taskId = Number(taskIdParam);
      if (Number.isInteger(taskId)) {
        let deepLinkedTask: TaskTimeLog | undefined;
        for (const user of users) {
          deepLinkedTask = (user.time_logs || []).find((timeLog) => timeLog.task_id === taskId);
          if (deepLinkedTask) break;
        }

        if (deepLinkedTask) {
          editingTask = deepLinkedTask;
        }
      }
    }
  }

  function handleOpenTaskForm(event: CustomEvent<{ member_id: number; start: Date; end: Date }>) {
    taskFormSelection = event.detail;
  }

  async function handleTaskFormSubmit(event: CustomEvent<{ title: string; description?: string | null; tags: string[]; task_id?: number; start: Date; end: Date }>) {
    if (!taskFormSelection) return;
    const { title, description, tags, task_id, start, end } = event.detail;

    const newTaskData = {
      user_id: taskFormSelection.member_id,
      task_id: task_id || null,
      title: task_id ? null : title,
      description: task_id ? null : (description || null),
      tags: task_id ? null : tags,
      start_at: toLocalISOString(start),
      end_at: toLocalISOString(end)
    };

    try {
      const res = await fetch('http://localhost:3000/api/tasks/time-logs', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify(newTaskData)
      });

      if (!res.ok) throw new Error('Failed to create time log');

      const savedTask = await res.json();
      if (getJSTDateString(new Date(savedTask.start_at)) === selectedDate) {
        users = upsertTimeLog(users, savedTask);
      }
    } catch (e) {
      console.error('Error creating task:', e);
      alert('作業ログの作成に失敗しました。');
    } finally {
      taskFormSelection = null;
    }
  }

  async function handleUpdateTask(event: CustomEvent<TaskTimeLog>) {
    const updatedTask = event.detail;
    try {
      const res = await fetch(`http://localhost:3000/api/tasks/time-logs/${updatedTask.id}`, {
        method: 'PATCH',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({
          start_at: updatedTask.start_at,
          end_at: updatedTask.end_at
        })
      });

      if (!res.ok) throw new Error('Failed to update time log');
      
      const savedTask: TaskTimeLog = await res.json();

      const shouldUpdateDescription =
        !!editingTask &&
        updatedTask.id === editingTask.id &&
        updatedTask.task_description !== editingTask.task_description;

      if (shouldUpdateDescription) {
        const taskUpdateRes = await fetch(`http://localhost:3000/api/tasks/${updatedTask.task_id}`, {
          method: 'PATCH',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
          },
          body: JSON.stringify({
            description: updatedTask.task_description
          })
        });

        if (!taskUpdateRes.ok) throw new Error('Failed to update task description');
        const savedTaskMeta: Task = await taskUpdateRes.json();
        savedTask.task_description = savedTaskMeta.description ?? null;
      }

      if (getJSTDateString(new Date(savedTask.start_at)) === selectedDate) {
        users = upsertTimeLog(users, savedTask);
      } else {
        removeTimeLogById(savedTask.id);
      }
      editingTask = null;
    } catch (e) {
      console.error('Error updating task:', e);
      alert('作業ログの更新に失敗しました。');
    }
  }

  async function handleDeleteTask(event: CustomEvent<number>) {
    const taskId = event.detail;
    try {
      const res = await fetch(`http://localhost:3000/api/tasks/time-logs/${taskId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });

      if (!res.ok) throw new Error('Failed to delete time log');
      
      removeTimeLogById(taskId);
      editingTask = null;
    } catch (e) {
      console.error('Error deleting task:', e);
      alert('作業ログの削除に失敗しました。');
    }
  }

  async function handleAddMember(event: CustomEvent<{ name: string; username: string; password: string }>) {
    const { name, username, password } = event.detail;
    try {
      const res = await fetch('http://localhost:3000/api/users', {
        method: 'POST',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({ name, username, password, avatar_url: null, role: 'user' })
      });
      if (!res.ok) throw new Error('Failed to create user');
      const newUser = await res.json();
      users = [...users, { ...newUser, time_logs: [] }];
    } catch (e) {
      console.error(e);
      alert('ユーザーの追加に失敗しました。');
    }
  }

  async function handleDeleteMember(event: CustomEvent<number>) {
    const memberId = event.detail;
    try {
      const res = await fetch(`http://localhost:3000/api/users/${memberId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (!res.ok) throw new Error('Failed to delete user');
      users = users.filter(u => u.id !== memberId);
    } catch (e) {
      console.error(e);
      alert('ユーザーの削除に失敗しました。');
    }
  }

  function handleTimelineEditTask(event: CustomEvent<TaskTimeLog>) {
    editingTask = event.detail;
  }

</script>

{#if !$auth.initialized}
  <div class="flex h-full items-center justify-center text-slate-400 font-bold animate-pulse">読み込み中...</div>
{:else if !$auth.token}
  <Login on:loginSuccess={() => fetchUsers()} />
{:else}
<div class="relative flex h-full flex-col font-sans text-[var(--color-text)]">
  <!-- Top Bar -->
  <header class="z-[100] flex h-11 shrink-0 flex-nowrap items-center justify-between border-b px-1 sm:px-3 backdrop-blur-xl bg-[color:color-mix(in_srgb,var(--color-surface)_82%,transparent)] border-[color:color-mix(in_srgb,var(--color-border)_90%,transparent)] shadow-[0_14px_34px_-24px_rgba(15,23,42,0.6)]">
    <div class="flex min-w-0 shrink items-center gap-1 sm:gap-2">
      <h2 class="truncate whitespace-nowrap text-[12px] font-black uppercase tracking-tighter text-[var(--color-text)] sm:text-sm">GlanceFlow</h2>
      {#if $auth.user}
        <span class="hidden rounded-md border px-1.5 py-0.5 text-[9px] font-bold uppercase bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_85%,transparent)] border-[var(--color-border)] text-[var(--color-muted)] md:inline-flex">{$auth.user.role}</span>
      {/if}
    </div>
    
    <div class="flex min-w-0 flex-1 items-center justify-end gap-0 sm:gap-1">
      <div class="flex min-w-0 shrink items-center gap-0 rounded-xl border px-0.5 py-[1px] bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] border-[var(--color-border)] shadow-[0_8px_18px_-14px_rgba(15,23,42,0.55)] sm:gap-1">
        <button 
            on:click={() => {
                const d = new Date(selectedDate);
                d.setDate(d.getDate() - 1);
                selectedDate = d.toISOString().split('T')[0];
            }}
            class="shrink-0 rounded-md p-[1px] text-[var(--color-muted)] transition-all hover:bg-[var(--color-surface)] hover:text-[var(--color-text)] hover:shadow-sm sm:p-0.5"
            aria-label="前日へ移動"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
        </button>
        <input 
            type="date" 
            bind:value={selectedDate}
            class="w-[5.5rem] min-w-0 cursor-pointer appearance-none border-none bg-transparent px-0 text-center text-[9px] font-bold text-[var(--color-text)] outline-none [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-datetime-edit-fields-wrapper]:p-0 [&::-webkit-datetime-edit]:p-0 sm:w-24 sm:px-0.5 sm:text-[10px] md:w-28 md:px-1 md:text-[11px]"
        />
        <button 
            on:click={() => {
                const d = new Date(selectedDate);
                d.setDate(d.getDate() + 1);
                selectedDate = d.toISOString().split('T')[0];
            }}
            class="shrink-0 rounded-md p-[1px] text-[var(--color-muted)] transition-all hover:bg-[var(--color-surface)] hover:text-[var(--color-text)] hover:shadow-sm sm:p-0.5"
            aria-label="翌日へ移動"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        </button>
      </div>

      <div class="relative hidden group xl:block">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 group-focus-within:text-blue-500 transition-colors"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
        <input 
            type="text" 
            bind:value={filterText}
            placeholder="絞り込み..." 
            class="w-40 rounded-lg border border-transparent bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_84%,transparent)] py-1 pl-8 pr-3 text-[11px] text-[var(--color-text)] outline-none transition-all focus:border-blue-300 focus:bg-[var(--color-surface)] focus:ring-2 focus:ring-blue-500/15"
        />
      </div>

      <div class="hidden gap-1.5 rounded-lg border px-1.5 py-1 text-[9px] font-bold uppercase tracking-tighter bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_84%,transparent)] border-[var(--color-border)] text-[var(--color-muted)] lg:flex xl:gap-2">
        <div class="flex items-center gap-1">
          <div class="w-1.5 h-1.5 bg-yellow-400 rounded-full"></div> 未着手
        </div>
        <div class="flex items-center gap-1">
          <div class="w-1.5 h-1.5 bg-blue-500 rounded-full"></div> 進行中
        </div>
        <div class="flex items-center gap-1">
          <div class="w-1.5 h-1.5 bg-gray-400 rounded-full"></div> 完了
        </div>
        <div class="flex items-center gap-1">
          <div class="w-1.5 h-1.5 bg-red-500 rounded-full animate-pulse"></div> 期限切れ
        </div>
      </div>
      
      <div class="flex min-w-0 shrink items-center gap-0 border-l border-[var(--color-border)] pl-0 sm:gap-0.5 sm:pl-1">
        <!-- Navigation Menu Dropdown -->
        <div class="relative" bind:this={navMenu}>
          <button 
            on:click|stopPropagation={() => showNavDropdown = !showNavDropdown}
            class="flex items-center gap-0.5 rounded-md px-1 py-1 text-[10px] font-bold text-[var(--color-muted)] transition-all hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_90%,transparent)] hover:text-[var(--color-text)] sm:gap-1 sm:px-2"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.05.05a2 2 0 1 1-2.83 2.83l-.05-.05A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-.4-1.1 1.7 1.7 0 0 0-1-.6 1.7 1.7 0 0 0-1.87.34l-.05.05a2 2 0 1 1-2.83-2.83l.05-.05A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H2.9a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.1-.4 1.7 1.7 0 0 0 .6-1 1.7 1.7 0 0 0-.34-1.87l-.05-.05a2 2 0 1 1 2.83-2.83l.05.05A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V2.9a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 .4 1.1 1.7 1.7 0 0 0 1 .6 1.7 1.7 0 0 0 1.87-.34l.05-.05a2 2 0 1 1 2.83 2.83l-.05.05A1.7 1.7 0 0 0 19.4 9c.26.3.5.66.6 1a1.7 1.7 0 0 0 1.1.4h.1a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.1.4 1.7 1.7 0 0 0-.6 1z"/></svg>
            <span>ツール</span>
            <svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="transition-transform {showNavDropdown ? 'rotate-180' : ''}"><path d="m6 9 6 6 6-6"/></svg>
          </button>

          {#if showNavDropdown}
            <div class="absolute left-0 top-9 z-[110] w-52 rounded-xl border py-1.5 shadow-2xl animate-in fade-in zoom-in-95 duration-100 bg-[var(--color-surface)] border-[var(--color-border-strong)]">
              <button on:click={() => { goto('/reports'); showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
                日報一覧
              </button>
              <button on:click={() => { goto('/activity-log'); showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v4l3 3"/><path d="M3.05 11a9 9 0 1 1 .5 4m-.5 5v-5h5"/></svg>
                操作履歴
              </button>
              <button on:click={() => { goto('/analytics'); showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="20" x2="12" y2="10"/><line x1="18" y1="20" x2="18" y2="4"/><line x1="6" y1="20" x2="6" y2="16"/></svg>
                個人分析
              </button>
              <button on:click={() => { goto('/today-focus'); showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3h18v18H3z"/><path d="M3 9h18"/><path d="M9 21V9"/></svg>
                本日のフォーカス
              </button>
              <div class="my-1 h-px bg-[color:color-mix(in_srgb,var(--color-border)_88%,transparent)]"></div>
              {#if $auth.user?.role === 'admin'}
                <button on:click={() => { showUserManagement = true; showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="19" y1="8" x2="19" y2="14"/><line x1="22" y1="11" x2="16" y2="11"/></svg>
                  メンバー管理
                </button>
              {/if}
              <button on:click={() => { showDisplayGroupSettings = true; showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)] hover:text-[var(--color-text)]">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
                グループ設定
              </button>
              {#if $auth.user?.role === 'admin'}
                <button on:click={() => { goto('/admin/task-reports'); showNavDropdown = false; }} class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[11px] font-bold text-blue-600 hover:bg-blue-500/10">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M18 7a4 4 0 0 0-3 3.87"/></svg>
                  タスクレポート
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <button 
            on:click={() => goto('/reports/new')}
            class="inline-flex items-center gap-0.5 rounded-md bg-slate-900 px-1 py-1 text-[10px] font-bold text-white shadow-sm transition-all hover:bg-slate-800 sm:gap-1 sm:px-2"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14"/><path d="M5 12h14"/></svg>
            <span class="hidden md:inline">日報提出</span>
        </button>

        <div class="ml-0 flex items-center gap-0 sm:ml-1 sm:gap-0.5">
            <div class="relative" bind:this={notificationMenu}>
              <button
                on:click|stopPropagation={() => showNotifications = !showNotifications}
                class="relative rounded-md p-1 text-[var(--color-muted)] transition-colors hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_90%,transparent)] hover:text-[var(--color-text)] sm:p-1.5"
                title="通知"
                aria-label="通知を開く"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 0 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
                {#if unreadNotificationCount > 0}
                  <span class="absolute right-0.5 top-0.5 flex h-[14px] min-w-[14px] items-center justify-center rounded-full border-2 border-[var(--color-surface)] bg-red-500 px-1 text-[8px] font-bold text-white">
                    {unreadNotificationCount > 99 ? '99+' : unreadNotificationCount}
                  </span>
                {/if}
              </button>

              {#if showNotifications}
                <div class="absolute right-0 top-9 z-[110] max-h-96 w-80 overflow-hidden rounded-xl border shadow-2xl animate-in fade-in slide-in-from-top-2 duration-200 bg-[var(--color-surface)] border-[var(--color-border-strong)]">
                  <div class="flex items-center justify-between border-b px-4 py-3 border-[var(--color-border)] bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_76%,transparent)]">
                    <h3 class="text-xs font-black uppercase tracking-tight text-[var(--color-text)]">通知</h3>
                    <button
                      on:click={markAllNotificationsAsRead}
                      disabled={unreadNotificationCount === 0}
                      class="text-[10px] font-bold text-blue-600 hover:underline disabled:text-slate-400/70"
                    >
                      すべて既読にする
                    </button>
                  </div>

                  <div class="max-h-80 overflow-y-auto">
                    {#if notificationsLoading}
                      <div class="px-4 py-8 text-center text-[11px] font-medium text-[var(--color-muted)]">読み込み中...</div>
                    {:else if notificationsError}
                      <div class="px-4 py-8 text-center text-[11px] text-red-500 font-medium">{notificationsError}</div>
                    {:else if notifications.length === 0}
                      <div class="px-4 py-12 text-center">
                        <div class="mb-2 flex justify-center text-[var(--color-border)]">
                          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
                        </div>
                        <div class="text-[11px] font-bold text-[var(--color-muted)]">通知はありません</div>
                      </div>
                    {:else}
                      {#each notifications as notification (notification.id)}
                        <button
                          on:click={() => handleNotificationClick(notification)}
                          class="w-full border-b px-4 py-3 text-left transition-colors border-[var(--color-border)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_86%,transparent)] {notification.is_read ? 'opacity-60' : 'bg-blue-500/8'}"
                        >
                          <div class="mb-0.5 text-[11px] font-bold text-[var(--color-text)]">{notification.title}</div>
                          {#if notification.body}
                            <div class="line-clamp-2 text-[10px] leading-relaxed text-[var(--color-muted)]">{notification.body}</div>
                          {/if}
                          <div class="mt-1.5 text-[9px] font-medium text-[var(--color-muted)]">{new Date(notification.created_at).toLocaleString('ja-JP')}</div>
                        </button>
                      {/each}
                    {/if}
                  </div>
                </div>
              {/if}
            </div>

            <ThemeToggle />

            <!-- User Menu Dropdown -->
            <div class="relative" bind:this={userMenu}>
              <button 
                on:click|stopPropagation={() => showUserDropdown = !showUserDropdown}
                class="rounded-full border-2 border-transparent p-0.5 transition-all hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_90%,transparent)] sm:p-1 {showUserDropdown ? 'border-[var(--color-border)] bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_82%,transparent)]' : ''}"
              >
                {#if $auth.user?.avatar_url}
                  <img src={$auth.user.avatar_url} alt="Profile" class="w-6 h-6 rounded-full object-cover shadow-sm" />
                {:else}
                  <div class="w-6 h-6 rounded-full bg-gradient-to-br from-slate-700 to-slate-900 flex items-center justify-center text-[10px] font-black text-white shadow-sm">
                    {$auth.user?.name.charAt(0).toUpperCase()}
                  </div>
                {/if}
              </button>

              {#if showUserDropdown}
                <div class="absolute right-0 top-9 z-[110] w-52 rounded-xl border py-2 shadow-2xl animate-in fade-in slide-in-from-top-2 duration-200 bg-[var(--color-surface)] border-[var(--color-border-strong)]">
                  <div class="mb-1 border-b px-4 py-2 border-[var(--color-border)]">
                    <div class="truncate text-[11px] font-black text-[var(--color-text)]">{$auth.user?.name}</div>
                    <div class="text-[9px] font-bold uppercase tracking-tighter text-[var(--color-muted)]">{$auth.user?.role}</div>
                  </div>
                  
                  <button on:click={() => { showProfile = true; showUserDropdown = false; }} class="flex w-full items-center gap-2 px-4 py-2 text-left text-[11px] font-bold text-[var(--color-muted)] hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_78%,transparent)]">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                    プロフィール設定
                  </button>

                  <div class="my-1 h-px bg-[var(--color-border)]"></div>
                  
                  <button on:click={logout} class="w-full text-left px-4 py-2 text-[11px] font-bold text-red-500 hover:bg-red-50 flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
                    ログアウト
                  </button>
                </div>
              {/if}
            </div>
        </div>
      </div>
    </div>
  </header>

  <main class="flex min-h-0 flex-1 flex-col overflow-hidden p-2">
    <!-- Sub Header: Group Selector -->
    <div class="mb-2 flex items-center gap-2 px-1 py-1.5">
      <div class="flex items-center rounded-xl border p-1 shadow-sm bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_84%,transparent)] border-[var(--color-border)]">
        <button 
          on:click={() => selectedGroupId = null}
          class="rounded-lg px-3 py-1 text-[10px] font-black uppercase tracking-tight transition-all {selectedGroupId === null ? 'bg-[var(--color-surface)] text-[var(--color-text)] shadow-sm' : 'text-[var(--color-muted)] hover:text-[var(--color-text)]'}"
        >
          全員
        </button>
        {#each displayGroups as group}
          <button 
            on:click={() => selectedGroupId = group.id}
            class="rounded-lg px-3 py-1 text-[10px] font-black uppercase tracking-tight transition-all {selectedGroupId === group.id ? 'bg-[var(--color-surface)] text-blue-600 shadow-sm' : 'text-[var(--color-muted)] hover:text-[var(--color-text)]'}"
          >
            {group.name}
          </button>
        {/each}
      </div>
      <div class="ml-auto px-2 text-[9px] font-bold uppercase tracking-widest text-[var(--color-muted)]">
        {filteredUsers.length}名のメンバーを表示中
      </div>
    </div>

    {#if loading}
      <div class="flex-1 flex items-center justify-center text-slate-400 font-bold animate-pulse">ダッシュボードを読み込み中...</div>
    {:else if error}
      <div class="flex flex-1 items-center justify-center rounded-xl border border-red-200/60 bg-red-500/10 text-red-500">{error}</div>
    {:else}
      <TimelineContainer 
        members={filteredUsers} 
        {baseDate}
        isAdmin={$auth.user?.role === 'admin'}
        on:openTaskForm={handleOpenTaskForm}
        on:editTask={handleTimelineEditTask}
        on:updateTask={(e) => handleUpdateTask(e)}
      />
    {/if}
  </main>

  {#if editingTask}
    <TaskEditModal 
      timeLog={editingTask} 
      on:close={() => editingTask = null}
      on:save={handleUpdateTask}
      on:delete={handleDeleteTask}
    />
  {/if}

  {#if taskFormSelection}
    <TaskForm
      start={taskFormSelection.start}
      end={taskFormSelection.end}
      member_id={taskFormSelection.member_id}
      existingTasks={[...new Set(users.find(u => u.id === taskFormSelection?.member_id)?.time_logs?.map(l => l.task_title).filter((t): t is string => !!t) || [])]}
      on:submit={handleTaskFormSubmit}
      on:cancel={() => taskFormSelection = null}
    />
  {/if}

  {#if showUserManagement}
    <UserManagementModal
      members={users}
      on:close={() => showUserManagement = false}
      on:addMember={handleAddMember}
      on:deleteMember={handleDeleteMember}
    />
  {/if}

  {#if showDisplayGroupSettings}
    <DisplayGroupModal
      members={users}
      groups={displayGroups}
      on:close={() => showDisplayGroupSettings = false}
      on:groupsUpdated={(e) => displayGroups = e.detail}
    />
  {/if}

  {#if showProfile}
    <ProfileModal
      on:close={() => showProfile = false}
    />
  {/if}
</div>
{/if}
