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
  class="backdrop:bg-slate-900/50 p-0 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in fade-in zoom-in-95 duration-200"
  on:close={() => dispatch('close')}
>
  <div class="bg-white flex flex-col h-[600px]">
    <div class="px-6 py-4 border-b border-slate-100 flex justify-between items-center bg-slate-50/50">
      <h3 class="text-sm font-black text-slate-800 uppercase tracking-tight">表示グループ設定</h3>
      <button type="button" on:click={() => dispatch('close')} class="text-slate-400 hover:text-slate-600 transition-colors" aria-label="表示グループ設定を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-6 pb-24">
      {#if showForm}
        <div class="space-y-6 animate-in slide-in-from-top-2 duration-200">
          <div>
            <label for="display-group-name" class="block text-[10px] font-bold text-slate-500 uppercase mb-2">グループ名</label>
            <input
              id="display-group-name"
              bind:value={name}
              placeholder="チームA, プロジェクトXなど..."
              class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-xs font-bold text-slate-800 outline-none focus:border-blue-500 focus:ring-4 focus:ring-blue-50 transition-all"
            />
          </div>

          <div>
            <p class="block text-[10px] font-bold text-slate-500 uppercase mb-2">メンバーを選択</p>
            <div class="grid grid-cols-2 gap-2">
              {#each members as member}
                <button
                  type="button"
                  on:click={() => toggleMember(member.id)}
                  class="flex items-center gap-2 p-2 rounded-lg border text-left transition-all {selectedMemberIds.includes(member.id) ? 'bg-blue-50 border-blue-200 ring-1 ring-blue-200' : 'bg-white border-slate-100 hover:border-slate-200'}"
                >
                  <div class="w-6 h-6 rounded-full bg-slate-100 flex items-center justify-center text-[10px] font-bold text-slate-500 shrink-0">
                    {member.name.charAt(0).toUpperCase()}
                  </div>
                  <span class="text-[11px] font-bold text-slate-700 truncate">{member.name}</span>
                  {#if selectedMemberIds.includes(member.id)}
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="ml-auto text-blue-500"><polyline points="20 6 9 17 4 12"/></svg>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        </div>

        <!-- Fixed Footer for Form Actions -->
        <div class="absolute bottom-0 left-0 right-0 p-6 bg-white/80 backdrop-blur-md border-t border-slate-100 flex gap-2">
          <button
            type="button"
            on:click={handleSave}
            class="flex-1 bg-slate-900 text-white py-3 rounded-xl text-[11px] font-black hover:bg-slate-800 transition-all shadow-lg shadow-slate-200 active:scale-95"
          >
            {editingGroupId ? '更新する' : '作成する'}
          </button>
          <button
            type="button"
            on:click={resetForm}
            class="px-6 py-3 rounded-xl text-[11px] font-bold text-slate-500 hover:bg-slate-50 transition-all"
          >
            キャンセル
          </button>
        </div>
      {:else}
        <div class="space-y-4">
          <div class="flex justify-between items-center">
            <h4 class="text-[10px] font-bold text-slate-400 uppercase tracking-widest">保存済みグループ</h4>
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
            <div class="py-12 text-center border-2 border-dashed border-slate-100 rounded-2xl">
              <p class="text-[11px] text-slate-400 font-bold">グループがまだありません</p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each groups as group}
                <div class="group flex items-center justify-between p-3 rounded-xl border border-slate-100 hover:border-slate-200 hover:bg-slate-50/50 transition-all">
                  <div>
                    <div class="text-[12px] font-bold text-slate-800">{group.name}</div>
                    <div class="text-[9px] font-bold text-slate-400 uppercase mt-0.5">{group.member_ids.length}人のメンバー</div>
                  </div>
                  <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      type="button"
                      on:click={() => startEdit(group)}
                      class="p-1.5 text-slate-400 hover:text-blue-500 transition-colors"
                      title="編集"
                      aria-label="グループを編集"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
                    </button>
                    <button
                      type="button"
                      on:click={() => handleDelete(group.id)}
                      class="p-1.5 text-slate-400 hover:text-red-500 transition-colors"
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

    <div class="px-6 py-4 border-t border-slate-50 bg-slate-50/30">
      <p class="text-[10px] text-slate-400 leading-relaxed font-medium">
        グループを作成すると、タイムラインの表示を特定のメンバーのみに素早く切り替えることができます。
      </p>
    </div>
  </div>
</dialog>
