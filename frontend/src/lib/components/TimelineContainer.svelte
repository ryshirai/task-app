<script lang="ts">
  import { type User } from '$lib/types';
  import MemberRow from './MemberRow.svelte';
  import NowLine from './NowLine.svelte';
  
  export let members: User[];
  export let baseDate: Date;

  // Define hours for header (9:00 to 18:00)
  const hours = Array.from({ length: 10 }, (_, i) => 9 + i);
</script>

<div class="border border-slate-200 rounded-lg shadow-xl shadow-slate-200/50 bg-white overflow-hidden relative flex flex-col flex-1">
  <!-- Header -->
  <div class="flex border-b border-slate-200 bg-white h-6 text-[9px] font-bold text-slate-400 uppercase tracking-tighter shrink-0 z-30">
    <div class="w-40 border-r border-slate-100 px-3 flex items-center bg-slate-50/80 shadow-[2px_0_5px_-2px_rgba(0,0,0,0.05)] z-20">メンバー</div>
    <div class="flex-1 relative bg-slate-50/40">
      {#each hours as hour, i}
        <!-- Grid line for header -->
        <div class="absolute top-0 bottom-0 border-l border-slate-200" style="left: {i * (100 / (hours.length - 1))}%"></div>
        <!-- Hour label right-aligned to the line -->
        <div class="absolute top-0 bottom-0 flex items-center justify-end w-8 -ml-8.5 pr-1" style="left: {i * (100 / (hours.length - 1))}%">
          <div class="text-slate-400 font-mono bg-white/80 px-0.5 rounded leading-none">{hour}:00</div>
        </div>
      {/each}
      <NowLine {baseDate} showDot={true} showLabel={true} />
    </div>
  </div>

  <!-- Rows and Vertical NowLine Overlay -->
  <div class="flex-1 relative overflow-hidden flex flex-col">
    <!-- Vertical line spanning all rows -->
    <div class="absolute top-0 bottom-0 pointer-events-none z-40" style="left: 10rem; right: 0;">
       <NowLine {baseDate} showLabel={false} showDot={false} />
    </div>

    <div class="overflow-y-auto flex-1 scrollbar-hide">
      {#each members as member (member.id)}
        <MemberRow 
          {member} 
          {baseDate}
          on:openTaskForm
          on:editTask 
          on:updateTask
        />
      {/each}
    </div>
  </div>
</div>
