<script lang="ts">
  import TimelineContainer from '$lib/components/TimelineContainer.svelte';
  import TaskForm from '$lib/components/TaskForm.svelte';
  import TaskEditModal from '$lib/components/TaskEditModal.svelte';
  import UserManagementModal from '$lib/components/UserManagementModal.svelte';
  import DisplayGroupModal from '$lib/components/DisplayGroupModal.svelte';
  import ProfileModal from '$lib/components/ProfileModal.svelte';
  import Login from '$lib/components/Login.svelte';
  import { type User, type TaskTimeLog, type Notification as AppNotification, type PaginatedNotifications, type DisplayGroup } from '$lib/types';
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

  async function handleTaskFormSubmit(event: CustomEvent<{ title: string; tags: string[]; task_id?: number; start: Date; end: Date }>) {
    if (!taskFormSelection) return;
    const { title, tags, task_id, start, end } = event.detail;

    const newTaskData = {
      user_id: taskFormSelection.member_id,
      task_id: task_id || null,
      title: task_id ? null : title,
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
      
      const savedTask = await res.json();
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

</script>

{#if !$auth.token}
  <Login on:loginSuccess={() => fetchUsers()} />
{:else}
<div class="h-full flex flex-col font-sans relative">
  <!-- Top Bar -->
  <header class="h-10 px-4 flex items-center justify-between shrink-0 bg-white border-b border-slate-200 shadow-sm z-[100]">
    <div class="flex items-center gap-3">
      <h2 class="text-sm font-black text-slate-800 whitespace-nowrap tracking-tighter uppercase">GlanceFlow</h2>
      {#if $auth.user}
        <span class="px-1.5 py-0.5 bg-slate-100 text-slate-500 rounded text-[9px] font-bold uppercase">{$auth.user.role}</span>
      {/if}
    </div>
    
    <div class="flex items-center gap-3">
      <div class="flex items-center bg-slate-100 rounded-lg p-0.5 border border-slate-200">
        <button 
            on:click={() => {
                const d = new Date(selectedDate);
                d.setDate(d.getDate() - 1);
                selectedDate = d.toISOString().split('T')[0];
            }}
            class="p-1 hover:bg-white hover:shadow-sm rounded-md transition-all text-slate-500"
            aria-label="前日へ移動"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
        </button>
        <input 
            type="date" 
            bind:value={selectedDate}
            class="bg-transparent border-none text-[11px] font-bold text-slate-700 outline-none px-1 cursor-pointer w-28 text-center"
        />
        <button 
            on:click={() => {
                const d = new Date(selectedDate);
                d.setDate(d.getDate() + 1);
                selectedDate = d.toISOString().split('T')[0];
            }}
            class="p-1 hover:bg-white hover:shadow-sm rounded-md transition-all text-slate-500"
            aria-label="翌日へ移動"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
        </button>
      </div>

      <div class="relative group">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 group-focus-within:text-blue-500 transition-colors"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
        <input 
            type="text" 
            bind:value={filterText}
            placeholder="絞り込み..." 
            class="pl-8 pr-3 py-1 bg-slate-100 border-transparent border focus:bg-white focus:border-blue-200 focus:ring-2 focus:ring-blue-50 rounded-lg text-[11px] outline-none w-40 transition-all"
        />
      </div>

      <div class="flex gap-3 text-[9px] font-bold text-slate-400 uppercase tracking-tighter bg-slate-50 px-2 py-1 rounded-md border border-slate-100">
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
      
      <div class="flex items-center gap-1 border-l border-slate-200 pl-3">
        <!-- Navigation Menu Dropdown -->
        <div class="relative" bind:this={navMenu}>
          <button 
            on:click|stopPropagation={() => showNavDropdown = !showNavDropdown}
            class="px-2.5 py-1 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-md transition-all text-[10px] font-bold flex items-center gap-1"
          >
            ツール
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="transition-transform {showNavDropdown ? 'rotate-180' : ''}"><path d="m6 9 6 6 6-6"/></svg>
          </button>

          {#if showNavDropdown}
            <div class="absolute left-0 top-9 w-44 bg-white border border-slate-200 rounded-xl shadow-xl z-50 py-1.5 animate-in fade-in zoom-in-95 duration-100">
              <button on:click={() => { goto('/reports'); showNavDropdown = false; }} class="w-full text-left px-3 py-2 text-[11px] font-bold text-slate-600 hover:bg-slate-50 flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
                日報一覧
              </button>
              <button on:click={() => { goto('/activity-log'); showNavDropdown = false; }} class="w-full text-left px-3 py-2 text-[11px] font-bold text-slate-600 hover:bg-slate-50 flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v4l3 3"/><path d="M3.05 11a9 9 0 1 1 .5 4m-.5 5v-5h5"/></svg>
                操作履歴
              </button>
              <button on:click={() => { goto('/analytics'); showNavDropdown = false; }} class="w-full text-left px-3 py-2 text-[11px] font-bold text-slate-600 hover:bg-slate-50 flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="20" x2="12" y2="10"/><line x1="18" y1="20" x2="18" y2="4"/><line x1="6" y1="20" x2="6" y2="16"/></svg>
                個人分析
              </button>
              {#if $auth.user?.role === 'admin'}
                <div class="h-px bg-slate-100 my-1"></div>
                <button on:click={() => { goto('/admin/task-reports'); showNavDropdown = false; }} class="w-full text-left px-3 py-2 text-[11px] font-bold text-blue-600 hover:bg-blue-50 flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M18 7a4 4 0 0 0-3 3.87"/></svg>
                  タスクレポート
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <button 
            on:click={() => goto('/reports/new')}
            class="px-3 py-1 bg-slate-900 hover:bg-slate-800 text-white rounded-md transition-all text-[10px] font-bold shadow-sm"
        >
            日報提出
        </button>

        <div class="flex items-center gap-0.5 ml-1">
            <div class="relative" bind:this={notificationMenu}>
              <button
                on:click|stopPropagation={() => showNotifications = !showNotifications}
                class="relative p-1.5 text-slate-400 hover:text-slate-600 transition-colors"
                title="通知"
                aria-label="通知を開く"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 0 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
                {#if unreadNotificationCount > 0}
                  <span class="absolute top-0.5 right-0.5 min-w-[14px] h-[14px] px-1 bg-red-500 text-white text-[8px] font-bold rounded-full flex items-center justify-center border-2 border-white">
                    {unreadNotificationCount > 99 ? '99+' : unreadNotificationCount}
                  </span>
                {/if}
              </button>

              {#if showNotifications}
                <div class="absolute right-0 top-9 z-50 w-80 max-h-96 overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl animate-in fade-in slide-in-from-top-2 duration-200">
                  <div class="flex items-center justify-between px-4 py-3 border-b border-slate-100 bg-slate-50/50">
                    <h3 class="text-xs font-black text-slate-800 uppercase tracking-tight">Notifications</h3>
                    <button
                      on:click={markAllNotificationsAsRead}
                      disabled={unreadNotificationCount === 0}
                      class="text-[10px] font-bold text-blue-600 disabled:text-slate-300 hover:underline"
                    >
                      すべて既読にする
                    </button>
                  </div>

                  <div class="max-h-80 overflow-y-auto">
                    {#if notificationsLoading}
                      <div class="px-4 py-8 text-center text-[11px] text-slate-400 font-medium">読み込み中...</div>
                    {:else if notificationsError}
                      <div class="px-4 py-8 text-center text-[11px] text-red-500 font-medium">{notificationsError}</div>
                    {:else if notifications.length === 0}
                      <div class="px-4 py-12 text-center">
                        <div class="text-slate-200 mb-2 flex justify-center">
                          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
                        </div>
                        <div class="text-[11px] text-slate-400 font-bold">通知はありません</div>
                      </div>
                    {:else}
                      {#each notifications as notification (notification.id)}
                        <button
                          on:click={() => handleNotificationClick(notification)}
                          class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-slate-50 transition-colors {notification.is_read ? 'opacity-60' : 'bg-blue-50/30'}"
                        >
                          <div class="text-[11px] font-bold text-slate-800 mb-0.5">{notification.title}</div>
                          {#if notification.body}
                            <div class="text-[10px] text-slate-500 leading-relaxed line-clamp-2">{notification.body}</div>
                          {/if}
                          <div class="text-[9px] text-slate-400 mt-1.5 font-medium">{new Date(notification.created_at).toLocaleString('ja-JP')}</div>
                        </button>
                      {/each}
                    {/if}
                  </div>
                </div>
              {/if}
            </div>

            <!-- User Menu Dropdown -->
            <div class="relative" bind:this={userMenu}>
              <button 
                on:click|stopPropagation={() => showUserDropdown = !showUserDropdown}
                class="p-1 rounded-full hover:bg-slate-100 transition-all border-2 border-transparent {showUserDropdown ? 'border-slate-200 bg-slate-50' : ''}"
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
                <div class="absolute right-0 top-9 w-52 bg-white border border-slate-200 rounded-xl shadow-2xl z-50 py-2 animate-in fade-in slide-in-from-top-2 duration-200">
                  <div class="px-4 py-2 border-b border-slate-50 mb-1">
                    <div class="text-[11px] font-black text-slate-800 truncate">{$auth.user?.name}</div>
                    <div class="text-[9px] font-bold text-slate-400 uppercase tracking-tighter">{$auth.user?.role}</div>
                  </div>
                  
                  <button on:click={() => { showProfile = true; showUserDropdown = false; }} class="w-full text-left px-4 py-2 text-[11px] font-bold text-slate-600 hover:bg-slate-50 flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                    プロフィール設定
                  </button>

                  {#if $auth.user?.role === 'admin'}
                    <button on:click={() => { showUserManagement = true; showUserDropdown = false; }} class="w-full text-left px-4 py-2 text-[11px] font-bold text-slate-600 hover:bg-slate-50 flex items-center gap-2">
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="19" y1="8" x2="19" y2="14"/><line x1="22" y1="11" x2="16" y2="11"/></svg>
                      メンバー管理
                    </button>
                  {/if}

                  <div class="h-px bg-slate-100 my-1"></div>
                  
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

  <main class="flex-1 min-h-0 flex flex-col p-1 overflow-hidden">
    <!-- Sub Header: Group Selector -->
    <div class="px-2 py-1.5 flex items-center gap-2 mb-1">
      <div class="flex items-center bg-slate-100/80 p-1 rounded-xl border border-slate-200/60 shadow-sm">
        <button 
          on:click={() => selectedGroupId = null}
          class="px-3 py-1 text-[10px] font-black uppercase tracking-tight rounded-lg transition-all {selectedGroupId === null ? 'bg-white text-slate-900 shadow-sm' : 'text-slate-400 hover:text-slate-600'}"
        >
          全員
        </button>
        {#each displayGroups as group}
          <button 
            on:click={() => selectedGroupId = group.id}
            class="px-3 py-1 text-[10px] font-black uppercase tracking-tight rounded-lg transition-all {selectedGroupId === group.id ? 'bg-white text-blue-600 shadow-sm' : 'text-slate-400 hover:text-slate-600'}"
          >
            {group.name}
          </button>
        {/each}
        <div class="w-px h-3 bg-slate-200 mx-1"></div>
        <button 
          on:click={() => showDisplayGroupSettings = true}
          class="p-1 text-slate-400 hover:text-slate-600 hover:bg-white rounded-lg transition-all"
          title="グループ設定を編集"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
        </button>
      </div>
      <div class="text-[9px] font-bold text-slate-400 uppercase tracking-widest ml-auto px-2">
        {filteredUsers.length} members visible
      </div>
    </div>

    {#if loading}
      <div class="flex-1 flex items-center justify-center text-slate-400 font-bold animate-pulse">ダッシュボードを読み込み中...</div>
    {:else if error}
      <div class="flex-1 flex items-center justify-center text-red-500 bg-red-50 rounded-xl border border-red-100">{error}</div>
    {:else}
      <TimelineContainer 
        members={filteredUsers} 
        {baseDate}
        isAdmin={$auth.user?.role === 'admin'}
        on:openTaskForm={handleOpenTaskForm}
        on:editTask={(e) => editingTask = e.detail}
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
