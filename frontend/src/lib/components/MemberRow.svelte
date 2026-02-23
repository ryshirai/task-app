<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { goto } from '$app/navigation';
  import { type User } from '$lib/types';
  import { START_HOUR, END_HOUR } from '$lib/utils';
  import TaskBar from './TaskBar.svelte';
  import InteractionLayer from './InteractionLayer.svelte';

  export let member: User;
  export let baseDate: Date;
  export let isAdmin = false;
  const hourCount = END_HOUR - START_HOUR + 1;
  const hourSegments = hourCount - 1;
  const quarterOffsets = [15, 30, 45];
  const dispatch = createEventDispatcher();

  function handleSelect(event: CustomEvent<{ start: Date; end: Date }>) {
    dispatch('openTaskForm', {
      member_id: member.id,
      ...event.detail
    });
  }

</script>

<div class="group flex border-b border-[var(--color-border)] bg-[color:color-mix(in_srgb,var(--color-surface)_90%,var(--color-bg)_10%)] transition-colors hover:bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_94%,transparent)] even:bg-[color:color-mix(in_srgb,var(--color-surface)_84%,var(--color-bg)_16%)]">
  <div class="relative z-10 flex h-10 w-40 shrink-0 items-center border-r border-[var(--color-border)] px-3 shadow-[2px_0_7px_-4px_rgba(15,23,42,0.4)]">
    {#if member.avatar_url}
      <img src={member.avatar_url} alt={member.name} class="mr-2 h-6 w-6 rounded-full border border-[var(--color-border)] object-cover shadow-sm" />
    {:else}
      <div class="mr-2 flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-[var(--color-border)] bg-gradient-to-br from-[var(--color-surface-elevated)] to-[color:color-mix(in_srgb,var(--color-surface-elevated)_62%,var(--color-border)_38%)] text-[10px] font-bold text-[var(--color-muted)] shadow-inner">
        {(member.name || '?').charAt(0).toUpperCase()}
      </div>
    {/if}
    <div class="flex flex-col min-w-0 flex-1">
        <div class="flex items-center gap-1 min-w-0">
          <span class="truncate text-[11px] font-bold leading-tight text-[var(--color-text)]">{member.name}</span>
          {#if isAdmin}
            <button
              on:click={() => goto(`/analytics?user_id=${member.id}`)}
              class="shrink-0 rounded p-0.5 text-[var(--color-muted)] transition-colors hover:bg-[color:color-mix(in_srgb,var(--color-surface)_82%,transparent)] hover:text-[var(--color-text)]"
              title="分析を表示"
              aria-label={`${member.name}の分析を表示`}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>
            </button>
          {/if}
        </div>
    </div>
  </div>

  <div class="relative h-10 min-w-0 flex-1 overflow-hidden bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_26%,transparent)]">
    <!-- Grid lines for each hour -->
    {#each Array(hourCount) as _, i}
      <div class="absolute top-0 bottom-0 border-l-2 border-[var(--color-border)] opacity-75" style="left: {i * (100 / hourSegments)}%;"></div>
    {/each}

    {#each Array(hourSegments) as _, i}
      {#each quarterOffsets as minute}
        <div
          class="absolute top-0 bottom-0 border-l border-dashed border-[var(--color-border)] opacity-12"
          style="left: {(i + minute / 60) * (100 / hourSegments)}%;"
        ></div>
      {/each}
    {/each}

    {#each member.time_logs || [] as task (task.id)}
      <TaskBar 
        {task} 
        {baseDate}
        on:click={(e) => dispatch('editTask', e.detail)} 
        on:update={(e) => dispatch('updateTask', e.detail)}
      />
    {/each}

    <InteractionLayer {baseDate} on:select={handleSelect} />

  </div>
</div>
