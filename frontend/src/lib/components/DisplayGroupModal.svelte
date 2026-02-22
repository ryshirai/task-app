<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '$lib/auth';
  import type { User, DisplayGroup } from '$lib/types';

  /** Available members that can be assigned to display groups. */
  export let members: User[] = [];
  /** Current list of persisted display groups. */
  export let groups: DisplayGroup[] = [];

  const dispatch = createEventDispatcher();
  
  let name = '';
  let selectedMemberIds: number[] = [];
  let editingGroupId: number | null = null;
  let showForm = false;

  /** Resets form state and returns to list mode. */
  function resetForm() {
    name = '';
    selectedMemberIds = [];
    editingGroupId = null;
    showForm = false;
  }

  /** Opens empty form for creating a new group. */
  function startCreate() {
    resetForm();
    showForm = true;
  }

  /** Opens form prefilled with an existing group. */
  function startEdit(group: DisplayGroup) {
    name = group.name;
    selectedMemberIds = [...group.member_ids];
    editingGroupId = group.id;
    showForm = true;
  }

  /** Creates or updates a display group, then emits updated list. */
  async function handleSave() {
    if (!name.trim() || selectedMemberIds.length === 0) {
      alert('名前と少なくとも1人のメンバーを選択してください。');
      return;
    }

    const payload = { name, member_ids: selectedMemberIds };
    const url = editingGroupId 
      ? `http://localhost:3000/api/display-groups/${editingGroupId}`
      : 'http://localhost:3000/api/display-groups';
    
    try {
      const res = await fetch(url, {
        method: editingGroupId ? 'PATCH' : 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify(payload)
      });

      if (!res.ok) throw new Error('保存に失敗しました。');
      
      const savedGroup = await res.json();
      if (editingGroupId) {
        groups = groups.map(g => g.id === editingGroupId ? savedGroup : g);
      } else {
        groups = [...groups, savedGroup];
      }
      resetForm();
      dispatch('groupsUpdated', groups);
    } catch (e) {
      console.error(e);
      alert('エラーが発生しました。');
    }
  }

  /** Deletes a display group after confirmation, then emits updated list. */
  async function handleDelete(id: number) {
    if (!confirm('このグループを削除しますか？')) return;

    try {
      const res = await fetch(`http://localhost:3000/api/display-groups/${id}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${$auth.token}` }
      });

      if (!res.ok) throw new Error('削除に失敗しました。');
      
      groups = groups.filter(g => g.id !== id);
      dispatch('groupsUpdated', groups);
    } catch (e) {
      console.error(e);
      alert('エラーが発生しました。');
    }
  }

  /** Toggles member selection for the group form. */
  function toggleMember(id: number) {
    if (selectedMemberIds.includes(id)) {
      selectedMemberIds = selectedMemberIds.filter(m => m !== id);
    } else {
      selectedMemberIds = [...selectedMemberIds, id];
    }
  }

  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="w-full max-w-lg overflow-hidden rounded-2xl p-0 shadow-2xl backdrop:bg-slate-900/50 animate-in fade-in zoom-in-95 duration-200"
  on:close={() => dispatch('close')}
>
  <div class="flex h-[600px] flex-col bg-surface-primary text-text-base">
    <div class="flex items-center justify-between border-b border-border-base bg-surface-secondary/70 px-6 py-4">
      <h3 class="text-sm font-black uppercase tracking-tight text-text-base">表示グループ設定</h3>
      <button type="button" on:click={() => dispatch('close')} class="text-text-muted transition-colors hover:text-text-base" aria-label="表示グループ設定を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-6 pb-24">
      {#if showForm}
        <div class="space-y-6 animate-in slide-in-from-top-2 duration-200">
          <div>
            <label for="display-group-name" class="mb-2 block text-[10px] font-bold uppercase text-text-muted">グループ名</label>
            <input
              id="display-group-name"
              bind:value={name}
              placeholder="チームA, プロジェクトXなど..."
              class="w-full rounded-xl border border-border-base bg-surface-secondary px-3 py-2 text-xs font-bold text-text-base outline-none transition-all focus:border-blue-500 focus:ring-4 focus:ring-blue-500/15"
            />
          </div>

          <div>
            <p class="mb-2 block text-[10px] font-bold uppercase text-text-muted">メンバーを選択</p>
            <div class="grid grid-cols-2 gap-2">
              {#each members as member}
                <button
                  type="button"
                  on:click={() => toggleMember(member.id)}
                  class="flex items-center gap-2 rounded-lg border p-2 text-left transition-all {selectedMemberIds.includes(member.id) ? 'border-blue-300 bg-blue-500/15 ring-1 ring-blue-300' : 'border-border-base bg-surface-primary hover:border-border-strong'}"
                >
                  <div class="h-6 w-6 shrink-0 rounded-full bg-surface-muted text-[10px] font-bold text-text-muted flex items-center justify-center">
                    {member.name.charAt(0).toUpperCase()}
                  </div>
                  <span class="truncate text-[11px] font-bold text-text-base">{member.name}</span>
                  {#if selectedMemberIds.includes(member.id)}
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="ml-auto text-blue-500"><polyline points="20 6 9 17 4 12"/></svg>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        </div>

        <!-- Fixed Footer for Form Actions -->
        <div class="absolute bottom-0 left-0 right-0 flex gap-2 border-t border-border-base bg-surface-primary/90 p-6 backdrop-blur-md">
          <button
            type="button"
            on:click={handleSave}
            class="flex-1 rounded-xl bg-slate-900 py-2.5 text-[11px] font-black text-white transition-all hover:bg-slate-800 active:scale-95"
          >
            {editingGroupId ? '更新する' : '作成する'}
          </button>
          <button
            type="button"
            on:click={resetForm}
            class="rounded-xl px-5 py-2.5 text-[11px] font-bold text-text-muted transition-all hover:bg-surface-secondary"
          >
            キャンセル
          </button>
        </div>
      {:else}
        <div class="space-y-4">
          <div class="flex justify-between items-center">
            <h4 class="text-[10px] font-bold uppercase tracking-widest text-text-muted">保存済みグループ</h4>
            <button
              type="button"
              on:click={startCreate}
              class="text-[10px] font-black text-blue-600 hover:text-blue-700 flex items-center gap-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
              新規作成
            </button>
          </div>

          {#if groups.length === 0}
            <div class="rounded-2xl border-2 border-dashed border-border-base py-12 text-center">
              <p class="text-[11px] font-bold text-text-muted">グループがまだありません</p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each groups as group}
                <div class="group flex items-center justify-between rounded-xl border border-border-base p-3 transition-all hover:border-border-strong hover:bg-surface-secondary/60">
                  <div>
                    <div class="text-[12px] font-bold text-text-base">{group.name}</div>
                    <div class="mt-0.5 text-[9px] font-bold uppercase text-text-muted">{group.member_ids.length}人のメンバー</div>
                  </div>
                  <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      type="button"
                      on:click={() => startEdit(group)}
                      class="p-1.5 text-text-muted transition-colors hover:text-blue-500"
                      title="編集"
                      aria-label="グループを編集"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
                    </button>
                    <button
                      type="button"
                      on:click={() => handleDelete(group.id)}
                      class="p-1.5 text-text-muted transition-colors hover:text-red-500"
                      title="削除"
                      aria-label="グループを削除"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="border-t border-border-base bg-surface-secondary/50 px-6 py-4">
      <p class="text-[10px] font-medium leading-relaxed text-text-muted">
        グループを作成すると、タイムラインの表示を特定のメンバーのみに素早く切り替えることができます。
      </p>
    </div>
  </div>
</dialog>
