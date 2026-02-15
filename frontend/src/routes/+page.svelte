<script lang="ts">
  import TimelineContainer from '$lib/components/TimelineContainer.svelte';
  import TaskForm from '$lib/components/TaskForm.svelte';
  import TaskEditModal from '$lib/components/TaskEditModal.svelte';
  import UserManagementModal from '$lib/components/UserManagementModal.svelte';
  import ProfileModal from '$lib/components/ProfileModal.svelte';
  import Login from '$lib/components/Login.svelte';
  import { TaskWebSocketClient } from '$lib/websocket';
  import { type User, type Task } from '$lib/types';
  import { auth, logout } from '$lib/auth';
  import { toLocalISOString } from '$lib/utils';
  import { upsertTask } from '$lib/taskUtils';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';

  let users: User[] = [];
  let loading = true;
  let error: string | null = null;
  let editingTask: Task | null = null;
  let showUserManagement = false;
  let showProfile = false;
  let taskFormSelection: { member_id: number; start: Date; end: Date } | null = null;
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let taskWebSocketClient: TaskWebSocketClient | null = null;
  let taskEventUnsubscribers: Array<() => void> = [];
  let filterText = '';
  let selectedDate = new Date().toISOString().split('T')[0];

  $: baseDate = new Date(selectedDate);

  $: filteredUsers = users.map(u => ({
    ...u,
    tasks: u.tasks.filter(t => 
      t.title.toLowerCase().includes(filterText.toLowerCase()) ||
      (t.tags || []).some(tag => tag.toLowerCase().includes(filterText.toLowerCase()))
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

  type TaskDeletedPayload = { id: number };

  function applyTaskCreated(task: Task) {
    users = upsertTask(users, task);
  }

  function applyTaskUpdated(task: Task) {
    users = upsertTask(users, task);
  }

  function applyTaskDeleted(payload: TaskDeletedPayload) {
    for (const user of users) {
      const taskIndex = user.tasks.findIndex(t => t.id === payload.id);
      if (taskIndex !== -1) {
        user.tasks.splice(taskIndex, 1);
        users = users;
        return;
      }
    }
  }

  function connectTaskWebSocket() {
    if (!$auth.token || taskWebSocketClient) return;

    const client = new TaskWebSocketClient($auth.token);
    taskEventUnsubscribers = [
      client.subscribe<Task>('task_created', (task: Task) => applyTaskCreated(task)),
      client.subscribe<Task>('task_updated', (task: Task) => applyTaskUpdated(task)),
      client.subscribe<TaskDeletedPayload>('task_deleted', (payload: TaskDeletedPayload) => applyTaskDeleted(payload))
    ];
    client.connect();
    taskWebSocketClient = client;
  }

  function disconnectTaskWebSocket() {
    taskEventUnsubscribers.forEach(unsubscribe => unsubscribe());
    taskEventUnsubscribers = [];
    taskWebSocketClient?.disconnect();
    taskWebSocketClient = null;
  }

  onMount(() => {
    if ($auth.token) {
        fetchUsers();
        pollInterval = setInterval(() => fetchUsers(true), 30000);
        connectTaskWebSocket();
    } else {
        loading = false;
    }
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
    disconnectTaskWebSocket();
  });

  $: if ($auth.token && !pollInterval) {
      fetchUsers();
      pollInterval = setInterval(() => fetchUsers(true), 30000);
      connectTaskWebSocket();
  }

  $: if (!$auth.token) {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
      disconnectTaskWebSocket();
  }

  $: if (selectedDate && $auth.token) {
      fetchUsers();
  }

  type CreateTaskPayload = { member_id: number; title: string; tags: string[]; start: Date; end: Date };

  async function createTask({ member_id, title, tags, start, end }: CreateTaskPayload) {
    const newTaskData = {
      member_id,
      title,
      tags,
      start_at: toLocalISOString(start),
      end_at: toLocalISOString(end)
    };

    try {
      const res = await fetch('http://localhost:3000/api/tasks', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify(newTaskData)
      });

      if (!res.ok) throw new Error('Failed to create task');

      const createdTask = await res.json();
      users = upsertTask(users, createdTask);
    } catch (e) {
      console.error('Error creating task:', e);
      alert('タスクの作成に失敗しました。');
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

  async function handleUpdateTask(event: CustomEvent<Task>) {
    const updatedTask = event.detail;
    try {
      const res = await fetch(`http://localhost:3000/api/tasks/${updatedTask.id}`, {
        method: 'PATCH',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({
          title: updatedTask.title,
          status: updatedTask.status,
          progress_rate: updatedTask.progress_rate,
          start_at: updatedTask.start_at,
          end_at: updatedTask.end_at
        })
      });

      if (!res.ok) throw new Error('Failed to update task');
      
      const savedTask = await res.json();
      users = upsertTask(users, savedTask);
      editingTask = null;
    } catch (e) {
      console.error('Error updating task:', e);
      alert('タスクの更新に失敗しました。');
    }
  }

  async function handleDeleteTask(event: CustomEvent<number>) {
    const taskId = event.detail;
    try {
      const res = await fetch(`http://localhost:3000/api/tasks/${taskId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });

      if (!res.ok) throw new Error('Failed to delete task');
      
      users = users.map(u => ({
        ...u,
        tasks: u.tasks.filter(t => t.id !== taskId)
      }));
      editingTask = null;
    } catch (e) {
      console.error('Error deleting task:', e);
      alert('タスクの削除に失敗しました。');
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
      users = [...users, { ...newUser, tasks: [] }];
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

  function getMyTasks(): Task[] {
      if (!$auth.user) return [];
      const me = users.find(u => u.id === $auth.user?.id);
      return me ? me.tasks : [];
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
            on:click={() => goto('/reports/new')}
            class="px-3 py-1 bg-emerald-600 hover:bg-emerald-700 text-white rounded-md transition-all text-[10px] font-bold shadow-sm"
        >
            日報提出
        </button>

        <div class="flex items-center gap-1">
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
        on:openTaskForm={handleOpenTaskForm}
        on:editTask={(e) => editingTask = e.detail}
        on:updateTask={(e) => handleUpdateTask(e)}
      />
    {/if}
  </main>

  {#if editingTask}
    <TaskEditModal 
      task={editingTask} 
      on:close={() => editingTask = null}
      on:save={handleUpdateTask}
      on:delete={handleDeleteTask}
    />
  {/if}

  {#if taskFormSelection}
    <TaskForm
      start={taskFormSelection.start}
      end={taskFormSelection.end}
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
