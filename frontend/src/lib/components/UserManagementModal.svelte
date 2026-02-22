<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '../auth';
  import type { User } from '$lib/types';

  export let members: User[];
  const dispatch = createEventDispatcher();

  let newMemberName = '';
  let newUsername = '';
  let newPassword = '';
  let invitationLink = '';
  let updatingRoleMemberId: number | null = null;
  let dialog: HTMLDialogElement;

  async function handleIssueInvitation() {
    try {
      const res = await fetch('http://localhost:3000/api/invitations', {
        method: 'POST',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({ role: 'user' })
      });
      if (!res.ok) throw new Error('招待URLの発行に失敗しました');
      const data = await res.json();
      invitationLink = `${window.location.origin}/join?token=${data.token}`;
    } catch (e: any) {
      alert(e.message);
    }
  }

  function handleCopyLink() {
    navigator.clipboard.writeText(invitationLink);
    alert('コピーしました！');
    invitationLink = '';
  }

  function handleClose() {
    dispatch('close');
  }

  async function handleAddMember() {
    if (!newMemberName.trim() || !newUsername.trim() || !newPassword.trim()) return;
    dispatch('addMember', { 
        name: newMemberName,
        username: newUsername,
        password: newPassword
    });
    newMemberName = '';
    newUsername = '';
    newPassword = '';
  }

  function handleDeleteMember(memberId: number) {
    if (confirm('このユーザーを削除しますか？関連するすべてのタスクも削除されます。')) {
      dispatch('deleteMember', memberId);
    }
  }

  async function handleUpdateRole(memberId: number, newRole: 'admin' | 'user') {
    try {
      updatingRoleMemberId = memberId;
      const res = await fetch(`http://localhost:3000/api/users/${memberId}/role`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({ role: newRole })
      });

      if (!res.ok) throw new Error('ロールの変更に失敗しました');

      members = members.map((member) =>
        member.id === memberId ? { ...member, role: newRole } : member
      );
      dispatch('updateMember', { memberId, role: newRole });
    } catch (e: any) {
      alert(e.message || 'ロールの変更に失敗しました');
    } finally {
      updatingRoleMemberId = null;
    }
  }

  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="w-[500px] rounded-xl p-0 shadow-2xl backdrop:bg-black/50 open:animate-in open:fade-in open:zoom-in-95 backdrop:animate-in backdrop:fade-in"
  on:close={handleClose}
>
  <div class="bg-surface-primary p-6 text-text-base">
    <div class="mb-6 flex items-center justify-between">
      <h3 class="text-xl font-bold text-text-base">ユーザー管理</h3>
      <button on:click={handleClose} class="text-text-muted hover:text-text-base" aria-label="ユーザー管理を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-6">
      <!-- Invitation -->
      <div class="rounded-xl border border-blue-300/35 bg-blue-500/10 p-4">
        <h4 class="text-xs font-bold text-blue-600 uppercase mb-3 tracking-widest">招待リンクで追加</h4>
        {#if invitationLink}
            <div class="flex gap-2">
                <input readOnly value={invitationLink} class="flex-1 rounded-lg border border-blue-300/45 bg-surface-primary px-3 py-2 text-[10px] font-mono text-text-base outline-none" />
                <button on:click={handleCopyLink} class="px-3 py-2 bg-blue-600 text-white rounded-lg text-xs font-bold whitespace-nowrap">コピー</button>
            </div>
        {:else}
            <button on:click={handleIssueInvitation} class="flex w-full items-center justify-center gap-2 rounded-lg border border-blue-300/45 bg-surface-primary py-2 text-xs font-bold text-blue-600 transition-colors hover:bg-blue-500/10">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
                招待URLを発行する
            </button>
        {/if}
      </div>

      <!-- User List -->
      <div>
        <h4 class="mb-2 px-1 text-xs font-bold uppercase tracking-widest text-text-muted">登録済みユーザー</h4>
        <div class="max-h-[250px] overflow-y-auto rounded-xl border border-border-base divide-y divide-border-base">
          {#each members as member}
            {@const isSelfAdmin = member.id === $auth.user?.id && member.role === 'admin'}
            {@const isUpdatingRole = updatingRoleMemberId === member.id}
            <div class="flex items-center justify-between p-3 transition-colors hover:bg-surface-secondary">
              <div class="flex items-center gap-3">
                 {#if member.avatar_url}
                   <img src={member.avatar_url} alt={member.name} class="w-8 h-8 rounded-full object-cover" />
                 {:else}
                   <div class="flex h-8 w-8 items-center justify-center rounded-full bg-surface-muted text-xs font-bold text-text-muted">
                      {member.name.charAt(0).toUpperCase()}
                   </div>
                 {/if}
                 <div class="flex flex-col">
                    <span class="text-sm font-bold text-text-base">{member.name}</span>
                    <span class="font-mono text-[10px] text-text-muted">@{member.username || 'no-id'} · {member.role === 'user' ? 'member' : member.role}</span>
                 </div>
              </div>
              <div class="flex items-center gap-2">
                <div class="inline-flex items-center rounded-lg border border-border-base bg-surface-primary p-0.5">
                  <button
                    on:click={() => handleUpdateRole(member.id, 'admin')}
                    disabled={isSelfAdmin || isUpdatingRole || member.role === 'admin'}
                    class="rounded-md px-2 py-1 text-[10px] font-bold transition-colors {member.role === 'admin' ? 'bg-slate-800 text-white' : 'text-text-muted hover:bg-surface-secondary'} disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    admin
                  </button>
                  <button
                    on:click={() => handleUpdateRole(member.id, 'user')}
                    disabled={isSelfAdmin || isUpdatingRole || member.role === 'user'}
                    class="rounded-md px-2 py-1 text-[10px] font-bold transition-colors {member.role === 'user' ? 'bg-slate-800 text-white' : 'text-text-muted hover:bg-surface-secondary'} disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    member
                  </button>
                </div>
                {#if member.role !== 'admin'}
                <button 
                  on:click={() => handleDeleteMember(member.id)}
                  class="rounded-lg p-2 text-text-muted transition-all hover:bg-red-500/10 hover:text-red-600"
                  title="削除"
                  aria-label={`${member.name}を削除`}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
                </button>
                {/if}
              </div>
            </div>
          {/each}
          {#if members.length === 0}
            <div class="p-4 text-center text-sm italic text-text-muted">ユーザーがいません</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</dialog>
