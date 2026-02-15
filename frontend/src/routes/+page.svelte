<script lang="ts">
  import TimelineContainer from '$lib/components/TimelineContainer.svelte';
  import TaskForm from '$lib/components/TaskForm.svelte';
  import TaskEditModal from '$lib/components/TaskEditModal.svelte';
  import UserManagementModal from '$lib/components/UserManagementModal.svelte';
  import ProfileModal from '$lib/components/ProfileModal.svelte';
  import Login from '$lib/components/Login.svelte';
  import { type User, type TaskTimeLog, type Notification as AppNotification, type PaginatedNotifications } from '$lib/types';
  import { auth, logout } from '$lib/auth';
  import { toLocalISOString, getTodayJSTString, getJSTDateString, formatDateTime } from '$lib/utils';
  import { upsertTimeLog } from '$lib/taskUtils';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { page } from '$app/stores';

  let users: User[] = [];
  let loading = true;
  let error: string | null = null;
  let editingTask: TaskTimeLog | null = null;
  let showUserManagement = false;
  let showProfile = false;
  let taskFormSelection: { member_id: number; start: Date; end: Date } | null = null;
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let filterText = '';
  let selectedDate = getTodayJSTString();
  let observedTaskIdParam: string | null = null;
  let deepLinkHandled = false;
  let notifications: AppNotification[] = [];
  let notificationsLoading = false;
  let notificationsError: string | null = null;
  let showNotifications = false;
  let notificationPollInterval: ReturnType<typeof setInterval> | null = null;
  let notificationMenu: HTMLDivElement | null = null;

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
  }));

  async function fetchUsers(silent = false) {
    if (!$auth.token || editingTask || showUserManagement || showProfile) return;

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
    if (!showNotifications || !notificationMenu) return;
    if (!notificationMenu.contains(event.target as Node)) {
      showNotifications = false;
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

  type CreateTaskPayload = { member_id: number; title: string; tags: string[]; start: Date; end: Date };

  async function createTask({ member_id, title, tags, start, end }: CreateTaskPayload) {
    const existingTaskTimeLog = users
      .find((user) => user.id === member_id)
      ?.time_logs?.find(
        (timeLog) => (timeLog.task_title || '').trim().toLowerCase() === title.trim().toLowerCase()
      );

    const newTaskData = {
      user_id: member_id,
      task_id: existingTaskTimeLog?.task_id ?? null,
      title: existingTaskTimeLog ? null : title,
      tags: existingTaskTimeLog ? null : tags,
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

      const createdTask = await res.json();
      if (getJSTDateString(new Date(createdTask.start_at)) === selectedDate) {
        users = upsertTimeLog(users, createdTask);
      }
    } catch (e) {
      console.error('Error creating task:', e);
      alert('作業ログの作成に失敗しました。');
    } finally {
      taskFormSelection = null;
    }
  }

  function handleOpenTaskForm(event: CustomEvent<{ member_id: number; start: Date; end: Date }>) {
    taskFormSelection = event.detail;
  }

  async function handleTaskFormSubmit(event: CustomEvent<{ title: string; tags: string[]; start: Date; end: Date }>) {
    if (!taskFormSelection) return;
    await createTask({
      member_id: taskFormSelection.member_id,
      ...event.detail
    });
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
  <header class="h-10 px-4 flex items-center justify-between shrink-0 bg-white border-b border-slate-200 shadow-sm z-20">
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
      
      <div class="flex items-center gap-1.5 border-l border-slate-200 pl-3">
        <button 
            on:click={() => goto('/reports')}
            class="px-2.5 py-1 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-md transition-all text-[10px] font-bold border border-transparent hover:border-slate-200"
        >
            日報を表示
        </button>
        <button 
            on:click={() => goto('/activity-log')}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors"
            title="操作履歴"
            aria-label="操作履歴を開く"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v4l3 3"></path><path d="M3.05 11a9 9 0 1 1 .5 4m-.5 5v-5h5"></path></svg>
        </button>
        <button
            on:click={() => goto('/analytics')}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors"
            title="個人分析"
            aria-label="個人分析を開く"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>
        </button>
        {#if $auth.user?.role === 'admin'}
        <button
            on:click={() => goto('/admin/task-reports')}
            class="px-2.5 py-1 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-md transition-all text-[10px] font-bold border border-transparent hover:border-slate-200"
        >
            タスクレポート
        </button>
        {/if}
        <button 
            on:click={() => goto('/reports/new')}
            class="px-3 py-1 bg-emerald-600 hover:bg-emerald-700 text-white rounded-md transition-all text-[10px] font-bold shadow-sm"
        >
            日報提出
        </button>

        <div class="flex items-center gap-1">
            <div class="relative" bind:this={notificationMenu}>
              <button
                on:click|stopPropagation={() => showNotifications = !showNotifications}
                class="relative p-1 text-slate-400 hover:text-slate-600 transition-colors"
                title="通知"
                aria-label="通知を開く"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 0 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
                {#if unreadNotificationCount > 0}
                  <span class="absolute -top-1 -right-1 min-w-[14px] h-[14px] px-1 bg-red-500 text-white text-[8px] font-bold rounded-full flex items-center justify-center">
                    {unreadNotificationCount > 99 ? '99+' : unreadNotificationCount}
                  </span>
                {/if}
              </button>

              {#if showNotifications}
                <div class="absolute right-0 top-8 z-50 w-80 max-h-96 overflow-hidden rounded-lg border border-slate-200 bg-white shadow-xl">
                  <div class="flex items-center justify-between px-3 py-2 border-b border-slate-100">
                    <h3 class="text-xs font-bold text-slate-700">通知</h3>
                    <button
                      on:click={markAllNotificationsAsRead}
                      disabled={unreadNotificationCount === 0}
                      class="text-[10px] font-bold text-blue-600 disabled:text-slate-300"
                    >
                      すべて既読
                    </button>
                  </div>

                  <div class="max-h-80 overflow-y-auto">
                    {#if notificationsLoading}
                      <div class="px-3 py-4 text-[11px] text-slate-400">読み込み中...</div>
                    {:else if notificationsError}
                      <div class="px-3 py-4 text-[11px] text-red-500">{notificationsError}</div>
                    {:else if notifications.length === 0}
                      <div class="px-3 py-4 text-[11px] text-slate-400">通知はありません。</div>
                    {:else}
                      {#each notifications as notification (notification.id)}
                        <button
                          on:click={() => handleNotificationClick(notification)}
                          class="w-full text-left px-3 py-2 border-b border-slate-100 hover:bg-slate-50 transition-colors {notification.is_read ? 'opacity-70' : 'bg-blue-50/50'}"
                        >
                          <div class="text-[11px] font-bold text-slate-800">{notification.title}</div>
                          {#if notification.body}
                            <div class="text-[10px] text-slate-500 mt-0.5 leading-snug">{notification.body}</div>
                          {/if}
                          <div class="text-[9px] text-slate-400 mt-1">{new Date(notification.created_at).toLocaleString('ja-JP')}</div>
                        </button>
                      {/each}
                    {/if}
                  </div>
                </div>
              {/if}
            </div>

            <button on:click={() => showProfile = true} class="p-1 text-slate-400 hover:text-slate-600 transition-colors" title="プロフィール設定" aria-label="プロフィール設定を開く">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>
            </button>

            {#if $auth.user?.role === 'admin'}
            <button on:click={() => showUserManagement = true} class="p-1 text-slate-400 hover:text-slate-600 transition-colors" title="ユーザー管理" aria-label="ユーザー管理を開く">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><line x1="19" y1="8" x2="19" y2="14"></line><line x1="22" y1="11" x2="16" y2="11"></line></svg>
            </button>
            {/if}

            <button on:click={logout} class="p-1 text-slate-400 hover:text-red-600 transition-colors" title="ログアウト" aria-label="ログアウト">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path><polyline points="16 17 21 12 16 7"></polyline><line x1="21" y1="12" x2="9" y2="12"></line></svg>
            </button>
        </div>
      </div>
    </div>
  </header>

  <main class="flex-1 min-h-0 flex flex-col p-1 overflow-hidden">
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
      existingTasks={users.find(u => u.id === taskFormSelection?.member_id)?.time_logs?.map(l => l.task_title).filter((v, i, a) => v && a.indexOf(v) === i) || []}
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

  {#if showProfile}
    <ProfileModal
      on:close={() => showProfile = false}
    />
  {/if}
</div>
{/if}
