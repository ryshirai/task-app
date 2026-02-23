<script lang="ts">
  import { apiFetch } from '$lib/api';
  import { createEventDispatcher, onMount } from 'svelte';
  import { auth } from '$lib/auth';

  const dispatch = createEventDispatcher();

  let currentPassword = '';
  let newPassword = '';
  let confirmPassword = '';
  let showCurrentPassword = false;
  let showNewPassword = false;
  let showConfirmPassword = false;
  let loading = false;
  let error = '';
  let success = false;
  let dialog: HTMLDialogElement;

  function isSecurePassword(value: string): boolean {
    return /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[^A-Za-z0-9]).{8,}$/.test(value);
  }

  /** Validates and submits a password update request for the current user. */
  async function handleUpdatePassword() {
    if (newPassword !== confirmPassword) {
      error = '新しいパスワードが一致しません。';
      return;
    }
    if (!isSecurePassword(newPassword)) {
      error = 'パスワードは8文字以上で、英大文字・英小文字・数字・記号をそれぞれ含めてください。';
      return;
    }

    loading = true;
    error = '';
    success = false;

    try {
      const res = await apiFetch('/api/users/me/password', {
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
  class="w-[400px] rounded-xl p-0 shadow-2xl backdrop:bg-black/50 open:animate-in open:fade-in open:zoom-in-95 backdrop:animate-in backdrop:fade-in"
  on:close={() => dispatch('close')}
>
  <div class="bg-surface-primary p-6 text-text-base">
    <div class="mb-6 flex items-center justify-between">
      <h3 class="text-xl font-bold text-text-base">プロフィール設定</h3>
      <button type="button" on:click={() => dispatch('close')} class="text-text-muted hover:text-text-base" aria-label="プロフィール設定を閉じる">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </button>
    </div>

    <div class="space-y-4">
      <div class="mb-2 flex items-center gap-4 rounded-xl border border-border-base bg-surface-secondary p-4">
        <div class="flex h-12 w-12 items-center justify-center rounded-full bg-blue-500 text-xl font-bold text-white shadow-inner">
            {$auth.user?.name.charAt(0).toUpperCase()}
        </div>
        <div>
            <p class="text-lg font-bold text-text-base">{$auth.user?.name}</p>
            <p class="font-mono text-xs text-text-muted">@{$auth.user?.username}</p>
        </div>
      </div>

      <div class="border-t border-border-base pt-2">
        <h4 class="mb-3 text-xs font-bold uppercase tracking-widest text-text-muted">パスワード変更</h4>
        
        <form on:submit|preventDefault={handleUpdatePassword} class="space-y-3">
          <div>
            <label for="current-password" class="mb-1 block text-[10px] font-bold uppercase text-text-muted">現在のパスワード</label>
            <div class="relative">
              <input 
                id="current-password"
                type={showCurrentPassword ? 'text' : 'password'}
                bind:value={currentPassword} 
                class="w-full rounded-lg border border-border-base bg-surface-secondary px-3 py-2 pr-10 text-sm text-text-base focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
              <button
                type="button"
                class="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-text-muted transition-colors hover:text-text-base"
                aria-label={showCurrentPassword ? '現在のパスワードを隠す' : '現在のパスワードを表示'}
                on:click={() => (showCurrentPassword = !showCurrentPassword)}
              >
                {#if showCurrentPassword}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.58 10.58a2 2 0 1 0 2.83 2.83"></path><path d="M9.88 4.24A10.94 10.94 0 0 1 12 4c7 0 10 8 10 8a17.22 17.22 0 0 1-2.17 3.19"></path><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 8 10 8a10.94 10.94 0 0 0 5.76-1.62"></path><line x1="2" y1="2" x2="22" y2="22"></line></svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.06 12a11 11 0 0 1 19.88 0 11 11 0 0 1-19.88 0z"></path><circle cx="12" cy="12" r="3"></circle></svg>
                {/if}
              </button>
            </div>
          </div>
          <div>
            <label for="new-password" class="mb-1 block text-[10px] font-bold uppercase text-text-muted">新しいパスワード</label>
            <div class="relative">
              <input 
                id="new-password"
                type={showNewPassword ? 'text' : 'password'}
                bind:value={newPassword} 
                class="w-full rounded-lg border border-border-base bg-surface-secondary px-3 py-2 pr-10 text-sm text-text-base focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
              <button
                type="button"
                class="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-text-muted transition-colors hover:text-text-base"
                aria-label={showNewPassword ? '新しいパスワードを隠す' : '新しいパスワードを表示'}
                on:click={() => (showNewPassword = !showNewPassword)}
              >
                {#if showNewPassword}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.58 10.58a2 2 0 1 0 2.83 2.83"></path><path d="M9.88 4.24A10.94 10.94 0 0 1 12 4c7 0 10 8 10 8a17.22 17.22 0 0 1-2.17 3.19"></path><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 8 10 8a10.94 10.94 0 0 0 5.76-1.62"></path><line x1="2" y1="2" x2="22" y2="22"></line></svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.06 12a11 11 0 0 1 19.88 0 11 11 0 0 1-19.88 0z"></path><circle cx="12" cy="12" r="3"></circle></svg>
                {/if}
              </button>
            </div>
            <p class="mt-1 text-[10px] text-text-muted">8文字以上、英大文字・英小文字・数字・記号を各1文字以上含めてください</p>
          </div>
          <div>
            <label for="confirm-password" class="mb-1 block text-[10px] font-bold uppercase text-text-muted">新しいパスワード（確認）</label>
            <div class="relative">
              <input 
                id="confirm-password"
                type={showConfirmPassword ? 'text' : 'password'}
                bind:value={confirmPassword} 
                class="w-full rounded-lg border border-border-base bg-surface-secondary px-3 py-2 pr-10 text-sm text-text-base focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
              <button
                type="button"
                class="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-text-muted transition-colors hover:text-text-base"
                aria-label={showConfirmPassword ? '確認用パスワードを隠す' : '確認用パスワードを表示'}
                on:click={() => (showConfirmPassword = !showConfirmPassword)}
              >
                {#if showConfirmPassword}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.58 10.58a2 2 0 1 0 2.83 2.83"></path><path d="M9.88 4.24A10.94 10.94 0 0 1 12 4c7 0 10 8 10 8a17.22 17.22 0 0 1-2.17 3.19"></path><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 8 10 8a10.94 10.94 0 0 0 5.76-1.62"></path><line x1="2" y1="2" x2="22" y2="22"></line></svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.06 12a11 11 0 0 1 19.88 0 11 11 0 0 1-19.88 0z"></path><circle cx="12" cy="12" r="3"></circle></svg>
                {/if}
              </button>
            </div>
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
            class="mt-2 w-full rounded-lg bg-blue-600 px-4 py-2.5 text-sm font-bold text-white transition-all hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? '更新中...' : 'パスワードを更新'}
          </button>
        </form>
      </div>
    </div>
  </div>
</dialog>
