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

  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="backdrop:bg-black/50 p-0 rounded-xl shadow-2xl w-[500px] open:animate-in open:fade-in open:zoom-in-95 backdrop:animate-in backdrop:fade-in"
  on:close={handleClose}
>
  <div class="p-6 bg-white">
    <div class="flex justify-between items-center mb-6">
      <h3 class="text-xl font-bold text-slate-800">ユーザー管理</h3>
      <button on:click={handleClose} class="text-slate-400 hover:text-slate-600" aria-label="ユーザー管理を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-6">
      <!-- Invitation -->
      <div class="bg-blue-50 p-4 rounded-xl border border-blue-100">
        <h4 class="text-xs font-bold text-blue-600 uppercase mb-3 tracking-widest">招待リンクで追加</h4>
        {#if invitationLink}
            <div class="flex gap-2">
                <input readOnly value={invitationLink} class="flex-1 px-3 py-2 bg-white border border-blue-200 rounded-lg text-[10px] font-mono outline-none" />
                <button on:click={handleCopyLink} class="px-3 py-2 bg-blue-600 text-white rounded-lg text-xs font-bold whitespace-nowrap">コピー</button>
            </div>
        {:else}
            <button on:click={handleIssueInvitation} class="w-full py-2 bg-white border border-blue-200 text-blue-600 rounded-lg text-xs font-bold hover:bg-blue-100 transition-colors flex items-center justify-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
                招待URLを発行する
            </button>
        {/if}
      </div>

      <!-- User List -->
      <div>
        <h4 class="text-xs font-bold text-slate-500 uppercase mb-2 tracking-widest px-1">登録済みユーザー</h4>
        <div class="border border-slate-200 rounded-xl divide-y divide-slate-100 max-h-[250px] overflow-y-auto">
          {#each members as member}
            <div class="p-3 flex items-center justify-between hover:bg-slate-50 transition-colors">
              <div class="flex items-center gap-3">
                 {#if member.avatar_url}
                   <img src={member.avatar_url} alt={member.name} class="w-8 h-8 rounded-full object-cover" />
                 {:else}
                   <div class="w-8 h-8 rounded-full bg-slate-200 flex items-center justify-center text-xs font-bold text-slate-500">
                      {member.name.charAt(0).toUpperCase()}
                   </div>
                 {/if}
                 <div class="flex flex-col">
                    <span class="font-bold text-slate-700 text-sm">{member.name}</span>
                    <span class="text-[10px] text-slate-400 font-mono">@{member.username || 'no-id'} · {member.role}</span>
                 </div>
              </div>
              {#if member.role !== 'admin'}
              <button 
                on:click={() => handleDeleteMember(member.id)}
                class="text-slate-300 hover:text-red-600 p-2 rounded-lg hover:bg-red-50 transition-all"
                title="削除"
                aria-label={`${member.name}を削除`}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
              </button>
              {/if}
            </div>
          {/each}
          {#if members.length === 0}
            <div class="p-4 text-center text-slate-400 text-sm italic">ユーザーがいません</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</dialog>
