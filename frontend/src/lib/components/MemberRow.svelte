<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { type User } from '$lib/types';
  import TaskBar from './TaskBar.svelte';
  import InteractionLayer from './InteractionLayer.svelte';
  import TaskForm from './TaskForm.svelte';

  export let member: User;
  export let baseDate: Date;
  const dispatch = createEventDispatcher();

  let activeSelection: { start: Date; end: Date } | null = null;

  function handleSelect(event: CustomEvent<{ start: Date; end: Date }>) {
    activeSelection = event.detail;
  }

  function handleSubmit(event: CustomEvent<{ title: string; tags: string[]; start: Date; end: Date }>) {
    dispatch('createTask', {
      member_id: member.id,
      ...event.detail
    });
    activeSelection = null;
  }

  // Workload calculation
  $: activeTasks = member.tasks.filter(t => t.status !== 'done');
  $: hasOverlap = activeTasks.some((t1, i) => 
    activeTasks.some((t2, j) => 
      i !== j && 
      new Date(t1.start_at) < new Date(t2.end_at) && 
      new Date(t2.start_at) < new Date(t1.end_at)
    )
  );
  
  type LoadLevel = 'overload' | 'high' | 'moderate' | 'low';
  
  $: loadLevel = (() => {
    if (hasOverlap) return 'overload';
    if (activeTasks.length >= 3) return 'high';
    if (activeTasks.length >= 1) return 'moderate';
    return 'low';
  })() as LoadLevel;

  const loadConfig: Record<LoadLevel, { label: string; color: string }> = {
    overload: { label: '集中', color: 'text-red-600 bg-red-50 border-red-100 shadow-[0_0_8px_rgba(239,68,68,0.2)]' },
    high: { label: '多忙', color: 'text-orange-600 bg-orange-50 border-orange-100' },
    moderate: { label: '稼働中', color: 'text-blue-600 bg-blue-50 border-blue-100' },
    low: { label: '余裕', color: 'text-slate-400 bg-slate-50 border-slate-100' }
  };
</script>

<div class="flex border-b border-slate-100 group bg-white hover:bg-slate-50/50 transition-colors even:bg-slate-50/80">
  <div class="w-40 px-3 h-10 flex items-center border-r border-slate-100 shrink-0 z-10 relative shadow-[2px_0_5px_-2px_rgba(0,0,0,0.05)]">
    {#if member.avatar_url}
      <img src={member.avatar_url} alt={member.name} class="w-6 h-6 rounded-full mr-2 border border-slate-100 shadow-sm object-cover" />
    {:else}
      <div class="w-6 h-6 rounded-full mr-2 bg-gradient-to-br from-slate-100 to-slate-200 flex items-center justify-center text-[10px] font-bold text-slate-500 border border-slate-200 shadow-inner shrink-0">
        {member.name.charAt(0).toUpperCase()}
      </div>
    {/if}
    <div class="flex flex-col min-w-0 flex-1">
        <span class="font-bold text-slate-700 truncate text-[11px] leading-tight">{member.name}</span>
        <div class="flex items-center gap-1 mt-0.5">
            <span class="px-1 py-0.5 rounded-[3px] text-[7px] font-black uppercase border {loadConfig[loadLevel].color} transition-all duration-300">
                {loadConfig[loadLevel].label}
            </span>
            {#if activeTasks.length > 0}
                <span class="text-[8px] text-slate-400 font-bold">{activeTasks.length}件</span>
            {/if}
        </div>
    </div>
  </div>

  <div class="flex-1 relative h-10 bg-slate-50/20">
    <!-- Grid lines for each hour -->
    {#each Array(10) as _, i}
      <div class="absolute top-0 bottom-0 border-l border-slate-200" style="left: {i * (100 / 9)}%;"></div>
    {/each}

    {#each member.tasks as task (task.id)}
      <TaskBar 
        {task} 
        {baseDate}
        on:click={(e) => dispatch('editTask', e.detail)} 
        on:update={(e) => dispatch('updateTask', e.detail)}
      />
    {/each}

    <InteractionLayer {baseDate} on:select={handleSelect} />

    {#if activeSelection}
      <TaskForm 
        start={activeSelection.start} 
        end={activeSelection.end} 
        on:submit={handleSubmit}
        on:cancel={() => activeSelection = null}
      />
    {/if}
  </div>
</div>
