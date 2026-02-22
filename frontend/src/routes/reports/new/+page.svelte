<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/auth';
  import type { Task, TaskTimeLog } from '$lib/types';
  import { goto } from '$app/navigation';
  import ReportPreview from '$lib/components/ReportPreview.svelte';

  let content = '';
  let reportDate = new Date().toISOString().split('T')[0];
  let loading = false;
  let tasksLoading = true;
  let userTasks: Task[] = [];

  async function fetchMyTasks() {
    if (!$auth.user) return;
    try {
      const res = await fetch(`http://localhost:3000/api/users?date=${reportDate}`, {
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });
      if (res.ok) {
        const users = await res.json();
        const me = users.find((u: any) => u.id === $auth.user?.id);
        if (me) {
            const logs: TaskTimeLog[] = me.time_logs || [];
            const taskMap = new Map<number, Task>();
            for (const log of logs) {
              if (taskMap.has(log.task_id)) continue;
              taskMap.set(log.task_id, {
                id: log.task_id,
                organization_id: log.organization_id,
                member_id: log.user_id,
                title: log.task_title || `Task #${log.task_id}`,
                status: log.task_status || 'todo',
                progress_rate: log.task_progress_rate ?? 0,
                total_duration_minutes: 0,
                tags: log.task_tags || [],
                start_at: log.start_at,
                end_at: log.end_at,
                created_at: log.start_at
              });
            }
            userTasks = Array.from(taskMap.values());
            generateTemplate();
        }
      }
    } catch (e) {
      console.error(e);
    } finally {
      tasksLoading = false;
    }
  }

  function generateTemplate() {
    const completedTasks = userTasks.filter(t => t.status === 'done');
    const ongoingTasks = userTasks.filter(t => t.status === 'doing' || t.status === 'todo');

    content = `## 今日の成果\n`;
    if (completedTasks.length > 0) {
      completedTasks.forEach(t => content += `- ${t.title} (完了)\n`);
    } else {
      content += `- 特になし\n`;
    }

    content += `\n## 明日の予定 / 残課題\n`;
    if (ongoingTasks.length > 0) {
      ongoingTasks.forEach(t => content += `- ${t.title} (継続)\n`);
    } else {
      content += `- 特になし\n`;
    }

    content += `\n## ひとこと\n- お疲れ様でした。`;
  }

  async function saveReport() {
    if (!content.trim()) return;
    loading = true;
    try {
      const res = await fetch('http://localhost:3000/api/reports', {
        method: 'POST',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({
          report_date: reportDate,
          content
        })
      });

      if (!res.ok) throw new Error('Failed to save report');
      goto('/');
    } catch (e) {
      console.error(e);
      alert('日報の保存に失敗しました。');
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    if (!$auth.token) {
        goto('/');
        return;
    }
    fetchMyTasks();
  });
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
      <h1 class="text-sm font-black text-text-base tracking-tight uppercase">日報作成</h1>
    </div>
    <div class="flex items-center gap-3">
        <button 
          on:click={saveReport} 
          disabled={loading || !content.trim()}
          class="btn-primary text-xs active:scale-95"
        >
          {loading ? '保存中...' : '日報を提出'}
        </button>
    </div>
  </header>

  <main class="max-w-[1200px] w-full mx-auto p-4 flex-1 flex flex-col md:flex-row gap-4 overflow-hidden">
    <!-- Editor Side -->
    <div class="flex-1 flex flex-col gap-4 min-w-0">
      <div class="bg-surface-primary p-4 rounded-2xl border border-border-base shadow-sm flex flex-col flex-1 min-h-[400px]">
        <div class="flex items-center justify-between mb-3">
          <label for="new-report-content" class="block text-[10px] font-bold text-text-muted uppercase tracking-widest">内容 (Markdown)</label>
          <div class="flex items-center gap-2">
            <span class="text-[10px] font-bold text-text-muted uppercase">対象日:</span>
            <input 
              type="date" 
              bind:value={reportDate}
              on:change={fetchMyTasks}
              class="form-control px-2 py-1 text-[10px] font-bold"
            />
          </div>
        </div>
        
        <textarea 
          id="new-report-content"
          bind:value={content} 
          class="form-control flex-1 p-4 font-mono text-xs resize-none leading-relaxed"
          placeholder="成果や課題を入力してください..."
        ></textarea>
      </div>
    </div>

    <!-- Preview & Side Info -->
    <div class="w-full md:w-[320px] flex flex-col gap-4 shrink-0 overflow-y-auto">
      <div class="h-[400px] md:h-2/3 shrink-0">
        <ReportPreview {content} />
      </div>

      <div class="bg-surface-primary p-4 rounded-2xl border border-border-base shadow-sm flex-1">
        <h3 class="text-[10px] font-black text-text-base uppercase tracking-widest mb-3 flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-500"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
            タスク状況
        </h3>
        
        {#if tasksLoading}
            <div class="py-2 text-center text-text-muted text-[10px] animate-pulse font-bold uppercase tracking-widest">Loading...</div>
        {:else if userTasks.length === 0}
            <div class="py-2 text-center text-text-muted text-[10px] italic">本日のタスクなし</div>
        {:else}
            <div class="space-y-2 max-h-[200px] overflow-y-auto pr-1">
                {#each userTasks as task}
                    <div class="p-2 bg-surface-secondary rounded-lg border border-border-base">
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-[8px] font-bold uppercase px-1 rounded {task.status === 'done' ? 'bg-emerald-100 text-emerald-600' : 'bg-blue-100 text-blue-600'}">
                                {task.status === 'done' ? '完了' : '進行中'}
                            </span>
                            <span class="text-[8px] font-black text-text-muted">{task.progress_rate}%</span>
                        </div>
                        <p class="text-[10px] font-bold text-text-base line-clamp-1">{task.title}</p>
                    </div>
                {/each}
            </div>
            <button 
                on:click={generateTemplate}
                class="btn-secondary w-full mt-3 py-1.5 border-dashed text-[9px]"
            >
                テンプレートを適用
            </button>
        {/if}
      </div>
    </div>
  </main>
</div>
{/if}
