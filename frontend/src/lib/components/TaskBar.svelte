<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { type Task } from '$lib/types';
  import { getTaskPosition, percentageToDate, snapTo15Min, toLocalISOString } from '$lib/utils';
  
  export let task: Task;
  export let baseDate: Date;

  const dispatch = createEventDispatcher();

  // Derived from task (source of truth)
  $: basePos = getTaskPosition(task);
  $: left = isDragging ? draggedLeft : basePos.left;
  $: width = isDragging ? draggedWidth : basePos.width;
  
  let isOverdue = false;
  let interval: ReturnType<typeof setInterval>;

  function checkOverdue() {
    if (task.status === 'done') {
      isOverdue = false;
      return;
    }
    const now = new Date();
    const end = new Date(task.end_at);
    isOverdue = now > end;
  }

  onMount(() => {
    checkOverdue();
    interval = setInterval(checkOverdue, 60000);
  });

  onDestroy(() => {
    clearInterval(interval);
  });

  // Dynamic coloring
  $: colorClass = (() => {
    if (task.status === 'done') return 'bg-gray-400';
    if (isOverdue) return 'bg-red-500 animate-pulse';
    if (task.status === 'doing') return 'bg-blue-500';
    return 'bg-yellow-400 text-gray-800'; // todo
  })();

  const statusMap = {
    todo: '未着手',
    doing: '進行中',
    done: '完了'
  };

  // Drag & Drop Logic
  let isDragging = false;
  let dragMode: 'move' | 'resize-l' | 'resize-r' | null = null;
  let startX = 0;
  let initialLeft = 0;
  let initialWidth = 0;
  let containerWidth = 0;
  let draggedLeft = 0;
  let draggedWidth = 0;
  let hasMoved = false;

  function handleMouseDown(e: MouseEvent, mode: 'move' | 'resize-l' | 'resize-r') {
    e.stopPropagation(); // Stop bubbling to MemberRow creation
    
    // Only left click
    if (e.button !== 0) return;

    isDragging = true;
    dragMode = mode;
    startX = e.clientX;
    hasMoved = false;

    // Get container width for px -> % conversion
    const parent = (e.currentTarget as HTMLElement).closest('.relative.h-10');
    containerWidth = parent ? parent.getBoundingClientRect().width : 0;

    initialLeft = basePos.left;
    initialWidth = basePos.width;
    draggedLeft = initialLeft;
    draggedWidth = initialWidth;

    window.addEventListener('mousemove', handleWindowMouseMove);
    window.addEventListener('mouseup', handleWindowMouseUp);
  }

  function handleWindowMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    
    const deltaPx = e.clientX - startX;
    if (Math.abs(deltaPx) > 3) hasMoved = true;

    const deltaPercent = containerWidth > 0 ? (deltaPx / containerWidth) * 100 : 0;

    if (dragMode === 'move') {
      draggedLeft = initialLeft + deltaPercent;
      // Clamp? Maybe later. For now let it float.
    } else if (dragMode === 'resize-r') {
      draggedWidth = Math.max(1, initialWidth + deltaPercent); // Min width 1%
    } else if (dragMode === 'resize-l') {
      const newLeft = initialLeft + deltaPercent;
      const newWidth = initialWidth - deltaPercent;
      if (newWidth > 1) {
        draggedLeft = newLeft;
        draggedWidth = newWidth;
      }
    }
  }

  function handleWindowMouseUp(e: MouseEvent) {
    if (!isDragging) return;
    
    isDragging = false;
    dragMode = null; // Clear mode
    window.removeEventListener('mousemove', handleWindowMouseMove);
    window.removeEventListener('mouseup', handleWindowMouseUp);

    if (!hasMoved) {
      // It was a click
      dispatch('click', task);
      return;
    }

    // Calculate new times from visual state
    const newStart = snapTo15Min(percentageToDate(draggedLeft, baseDate));
    let newEnd: Date;
    
    if (dragMode === 'move') {
      // Keep exact duration
      const durationMs = new Date(task.end_at).getTime() - new Date(task.start_at).getTime();
      newEnd = new Date(newStart.getTime() + durationMs);
    } else {
      // Resize: Calculate end from width
      const endPercent = draggedLeft + draggedWidth;
      newEnd = snapTo15Min(percentageToDate(endPercent, baseDate));
    }

    // Validation
    if (newEnd <= newStart) {
        newEnd = new Date(newStart.getTime() + 15 * 60000); // Min 15 min
    }

    dispatch('update', {
      ...task,
      start_at: toLocalISOString(newStart),
      end_at: toLocalISOString(newEnd)
    });
  }
</script>

<div
  class="absolute top-0.5 bottom-0.5 rounded shadow-sm border border-white/20 text-[9px] text-white px-1.5 flex flex-col justify-center {colorClass} z-20 transition-all duration-200 overflow-visible group/task hover:shadow-md hover:-translate-y-[1px] hover:z-30"
  style="left: {left}%; width: {width}%; cursor: grab;"
  class:cursor-grabbing={isDragging && dragMode === 'move'}
  class:brightness-110={isDragging}
  title="{task.title} ({statusMap[task.status]}, {task.progress_rate}%)"
  on:mousedown={(e) => handleMouseDown(e, 'move')}
  role="button"
  tabindex="0"
>
  <!-- Resize Handle Left -->
  <div 
    class="absolute left-0 top-0 bottom-0 w-2 -ml-1 cursor-ew-resize z-30 opacity-0 group-hover/task:opacity-100 transition-opacity flex items-center justify-center"
    on:mousedown={(e) => handleMouseDown(e, 'resize-l')}
    role="separator"
  >
    <div class="w-0.5 h-2.5 bg-white/50 rounded-full shadow-sm"></div>
  </div>

  <!-- Content -->
  <div class="font-bold truncate pointer-events-none select-none text-[10px] leading-none drop-shadow-sm">{task.title}</div>
  
  {#if task.tags && task.tags.length > 0}
    <div class="flex gap-0.5 mt-0.5 pointer-events-none overflow-hidden">
      {#each task.tags as tag}
        <span class="bg-white/20 px-0.5 rounded-[1px] text-[7px] font-black uppercase tracking-tighter whitespace-nowrap border border-white/5">
          {tag}
        </span>
      {/each}
    </div>
  {/if}

  {#if task.status === 'doing'}
    <div class="w-full bg-black/10 h-0.5 mt-0.5 rounded-full overflow-hidden pointer-events-none">
      <div class="bg-white/90 h-full shadow-sm" style="width: {task.progress_rate}%"></div>
    </div>
  {/if}

  <!-- Resize Handle Right -->
  <div 
    class="absolute right-0 top-0 bottom-0 w-2 -mr-1 cursor-ew-resize z-30 opacity-0 group-hover/task:opacity-100 transition-opacity flex items-center justify-center"
    on:mousedown={(e) => handleMouseDown(e, 'resize-r')}
    role="separator"
  >
    <div class="w-0.5 h-2.5 bg-white/50 rounded-full shadow-sm"></div>
  </div>
</div>
