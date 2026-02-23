<script lang="ts">
  import { type User } from "$lib/types";
  import { START_HOUR, END_HOUR } from "$lib/utils";
  import MemberRow from "./MemberRow.svelte";
  import NowLine from "./NowLine.svelte";

  export let members: User[];
  export let baseDate: Date;
  export let isAdmin = false;

  // Define hours for header (timeline start to end)
  const hours = Array.from(
    { length: END_HOUR - START_HOUR + 1 },
    (_, i) => START_HOUR + i,
  );
  const hourLabelStep = hours.length > 13 ? 2 : 1;
  const labeledHours = hours
    .slice(0, -1)
    .map((hour, i) => ({ hour, i }))
    .filter(({ i }) => i % hourLabelStep === 0);
  const quarterOffsets = [15, 30, 45];
</script>

<div
  class="relative flex flex-1 flex-col overflow-hidden rounded-2xl border bg-[var(--color-surface)] border-[var(--color-border)] shadow-[var(--shadow-elevated)] transition-all duration-300 hover:-translate-y-[1px] hover:shadow-[0_26px_50px_-32px_rgba(15,23,42,0.55)]"
>
  <!-- Header -->
  <div
    class="z-30 flex h-7 shrink-0 border-b text-[9px] font-black uppercase tracking-wider text-[var(--color-muted)] border-[var(--color-border)] bg-[color:color-mix(in_srgb,var(--color-surface)_88%,var(--color-bg)_12%)]"
  >
    <div
      class="z-20 flex w-40 items-center border-r px-3 border-[var(--color-border)] bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_84%,transparent)] shadow-[2px_0_10px_-6px_rgba(15,23,42,0.45)]"
    >
      メンバー
    </div>
    <div
      class="relative min-w-0 flex-1 overflow-hidden bg-[color:color-mix(in_srgb,var(--color-surface-elevated)_48%,transparent)]"
    >
      {#each hours as hour, i}
        <!-- Grid line for header -->
        <div
          class="absolute top-0 bottom-0 border-l-2 border-[var(--color-border)] opacity-75"
          style="left: {i * (100 / (hours.length - 1))}%"
        ></div>
      {/each}
      {#each hours.slice(0, -1) as _, i}
        {#each quarterOffsets as minute}
          <div
            class="absolute top-0 bottom-0 border-l border-dashed border-[var(--color-border)] opacity-12"
            style="left: {(i + minute / 60) * (100 / (hours.length - 1))}%"
          ></div>
        {/each}
      {/each}
      {#each labeledHours as labeledHour}
        <!-- Hour label aligned to each block start (right of hour grid line) -->
        <div
          class="absolute top-0 bottom-0 flex w-10 items-center justify-start pl-1"
          style="left: min({labeledHour.i *
            (100 / (hours.length - 1))}%, calc(100% - 2.5rem));"
        >
          <div
            class="rounded bg-[color:color-mix(in_srgb,var(--color-surface)_84%,transparent)] px-0.5 font-mono leading-none text-[var(--color-muted)]"
          >
            {String(labeledHour.hour).padStart(2, "0")}:00
          </div>
        </div>
      {/each}
      <NowLine {baseDate} showDot={true} showLabel={true} />
    </div>
  </div>

  <!-- Rows and Vertical NowLine Overlay -->
  <div class="relative flex min-w-0 flex-1 flex-col overflow-hidden">
    <!-- Vertical line spanning all rows -->
    <div
      class="absolute top-0 bottom-0 pointer-events-none z-40"
      style="left: 10rem; right: 0;"
    >
      <NowLine {baseDate} showLabel={false} showDot={false} />
    </div>

    <div class="scrollbar-hide flex-1 overflow-y-auto overflow-x-hidden">
      {#each members as member (member.id)}
        <MemberRow
          {member}
          {baseDate}
          {isAdmin}
          on:openTaskForm
          on:editTask
          on:updateTask
        />
      {/each}
    </div>
  </div>
</div>
