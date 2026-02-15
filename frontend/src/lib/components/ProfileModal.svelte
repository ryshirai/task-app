<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '../auth';

  const dispatch = createEventDispatcher();

  let currentPassword = '';
  let newPassword = '';
  let confirmPassword = '';
  let loading = false;
  let error = '';
  let success = false;
  let dialog: HTMLDialogElement;

  async function handleUpdatePassword() {
    if (newPassword !== confirmPassword) {
      error = '新しいパスワードが一致しません。';
      return;
    }
    if (newPassword.length < 4) {
      error = 'パスワードは4文字以上である必要があります。';
      return;
    }

    loading = true;
    error = '';
    success = false;

    try {
      const res = await fetch('http://localhost:3000/api/users/me/password', {
        method: 'PATCH',
        headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${$auth.token}`
        },
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword
        })
      });

      if (!res.ok) {
        const msg = await res.text();
        throw new Error(msg || 'パスワードの更新に失敗しました。');
      }

      success = true;
      currentPassword = '';
      newPassword = '';
      confirmPassword = '';
      setTimeout(() => {
          success = false;
          dispatch('close');
      }, 2000);
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class="backdrop:bg-black/50 p-0 rounded-xl shadow-2xl w-[400px] open:animate-in open:fade-in open:zoom-in-95 backdrop:animate-in backdrop:fade-in"
  on:close={() => dispatch('close')}
>
  <div class="p-6 bg-white">
    <div class="flex justify-between items-center mb-6">
      <h3 class="text-xl font-bold text-slate-800">プロフィール設定</h3>
      <button on:click={() => dispatch('close')} class="text-slate-400 hover:text-slate-600" aria-label="プロフィール設定を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div class="flex items-center gap-4 p-4 bg-slate-50 rounded-xl border border-slate-100 mb-2">
        <div class="w-12 h-12 rounded-full bg-blue-500 flex items-center justify-center text-xl font-bold text-white shadow-inner">
            {$auth.user?.name.charAt(0).toUpperCase()}
        </div>
        <div>
            <p class="font-bold text-slate-800 text-lg">{$auth.user?.name}</p>
            <p class="text-xs text-slate-400 font-mono">@{$auth.user?.username}</p>
        </div>
      </div>

      <div class="pt-2 border-t border-slate-100">
        <h4 class="text-xs font-bold text-slate-500 uppercase mb-3 tracking-widest">パスワード変更</h4>
        
        <form on:submit|preventDefault={handleUpdatePassword} class="space-y-3">
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase mb-1">現在のパスワード</label>
            <input 
              type="password"
              bind:value={currentPassword} 
              class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
              required
            />
          </div>
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase mb-1">新しいパスワード</label>
            <input 
              type="password"
              bind:value={newPassword} 
              class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
              required
            />
          </div>
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase mb-1">新しいパスワード（確認）</label>
            <input 
              type="password"
              bind:value={confirmPassword} 
              class="w-full px-3 py-2 border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
              required
            />
          </div>

          {#if error}
            <p class="text-xs text-red-500 font-bold">{error}</p>
          {/if}
          {#if success}
            <p class="text-xs text-emerald-600 font-bold flex items-center gap-1">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                更新しました。
            </p>
          {/if}

          <button 
            type="submit"
            disabled={loading || success}
            class="w-full mt-2 px-4 py-2.5 bg-blue-600 text-white font-bold text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-all shadow-lg shadow-blue-100"
          >
            {loading ? '更新中...' : 'パスワードを更新'}
          </button>
        </form>
      </div>
    </div>
  </div>
</dialog>
