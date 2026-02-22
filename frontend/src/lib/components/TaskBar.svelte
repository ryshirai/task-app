<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { type TaskTimeLog, type TaskStatus } from '$lib/types';
  import { getTaskPosition, percentageToDate, snapTo15Min, toLocalISOString } from '$lib/utils';
  
  /** Task log rendered on the timeline bar. */
  export let task: TaskTimeLog;
  /** Day anchor used to convert bar position percentages into datetime values. */
  export let baseDate: Date;

  const dispatch = createEventDispatcher();

  // Keep render position in sync with source-of-truth task data unless dragging.
  $: basePos = getTaskPosition(task);
  $: left = isDragging ? draggedLeft : basePos.left;
  $: width = isDragging ? draggedWidth : basePos.width;
  
  $: taskTitle = task.task_title || `Task #${task.task_id}`;
  $: taskStatus = (task.task_status || 'todo') as TaskStatus;
  $: taskProgressRate = task.task_progress_rate ?? 0;
  $: taskTags = task.task_tags || [];
  $: taskDescription =
    task.task_description ??
    (task as TaskTimeLog & { description?: string | null }).description ??
    null;
  $: taskDescriptionText = taskDescription?.trim() || '';
  $: taskTooltip = taskDescriptionText
    ? `${taskTitle} (${statusMap[taskStatus]}, ${taskProgressRate}%)\n${taskDescriptionText}`
    : `${taskTitle} (${statusMap[taskStatus]}, ${taskProgressRate}%)`;

  let isOverdue = false;
  let interval: ReturnType<typeof setInterval>;

  /** Recomputes overdue state for non-completed tasks. */
  function checkOverdue() {
    if (taskStatus === 'done') {
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

  // Dynamic coloring based on status and overdue state.
  $: colorClass = (() => {
    if (taskStatus === 'done') {
      return 'border-slate-300/65 bg-gradient-to-br from-slate-400 to-slate-500 text-white shadow-slate-900/25';
    }
    if (isOverdue) {
      return 'animate-pulse border-red-300/70 bg-gradient-to-br from-red-500 to-rose-600 text-white shadow-red-900/40';
    }
    if (taskStatus === 'doing') {
      return 'border-blue-300/70 bg-gradient-to-br from-blue-500 to-cyan-500 text-white shadow-blue-900/35';
    }
    return 'border-amber-300/70 bg-gradient-to-br from-amber-300 to-amber-400 text-slate-800 shadow-amber-700/25'; // todo
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

  /** Starts drag or resize interaction and installs global mouse listeners. */
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

  /** Updates temporary drag geometry while pointer is moving. */
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

  /** Finalizes interaction: emits click or updated time range. */
  function handleWindowMouseUp(e: MouseEvent) {
    if (!isDragging) return;
    
    isDragging = false;
    const finishedDragMode = dragMode;
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
    
    if (finishedDragMode === 'move') {
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

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="group/task absolute top-0.5 bottom-0.5 z-20 flex flex-col justify-center overflow-visible rounded-xl border px-2 py-0.5 text-[9px] shadow-[0_10px_18px_-14px] transition-all duration-300 hover:-translate-y-[1px] hover:shadow-[0_14px_22px_-14px] hover:z-30 {colorClass}"
  style="left: {left}%; width: {width}%; cursor: grab;"
  class:cursor-grabbing={isDragging && dragMode === 'move'}
  class:brightness-110={isDragging}
  title={taskTooltip}
  on:mousedown={(e) => handleMouseDown(e, 'move')}
  role="button"
  tabindex="0"
>
  <!-- Resize Handle Left -->
  <div 
    class="absolute left-0 top-0 bottom-0 z-30 -ml-1 flex w-2 cursor-ew-resize items-center justify-center opacity-0 transition-opacity group-hover/task:opacity-100"
    on:mousedown={(e) => handleMouseDown(e, 'resize-l')}
    role="button"
    aria-label="左端をリサイズ"
    tabindex="-1"
  >
    <div class="h-2.5 w-0.5 rounded-full bg-white/55 shadow-sm"></div>
  </div>

  <!-- Content -->
  <div class="pointer-events-none truncate select-none text-[11px] font-extrabold leading-tight drop-shadow-sm">{taskTitle}</div>
  
  {#if taskTags.length > 0}
    <div class="pointer-events-none mt-0.5 flex gap-0.5 overflow-hidden">
      {#each taskTags as tag}
        <span class="whitespace-nowrap rounded-[3px] border border-white/15 bg-white/15 px-0.5 text-[7px] font-black uppercase tracking-tighter">
          {tag}
        </span>
      {/each}
    </div>
  {/if}

  {#if taskStatus === 'doing'}
    <div class="pointer-events-none mt-0.5 h-0.5 w-full overflow-hidden rounded-full bg-black/15">
      <div class="h-full bg-white/90 shadow-sm" style="width: {taskProgressRate}%"></div>
    </div>
  {/if}

  <!-- Resize Handle Right -->
  <div 
    class="absolute right-0 top-0 bottom-0 z-30 -mr-1 flex w-2 cursor-ew-resize items-center justify-center opacity-0 transition-opacity group-hover/task:opacity-100"
    on:mousedown={(e) => handleMouseDown(e, 'resize-r')}
    role="button"
    aria-label="右端をリサイズ"
    tabindex="-1"
  >
    <div class="h-2.5 w-0.5 rounded-full bg-white/55 shadow-sm"></div>
  </div>
</div>
