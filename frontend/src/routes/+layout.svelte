<script lang="ts">
  import { onMount } from 'svelte';
  import { auth, resendVerification } from '$lib/auth';
  import '../app.css';
  import { initializeTheme, theme } from '$lib/theme';

  let { children } = $props();
  let resendLoading = $state(false);
  let resendMessage = $state('');
  let resendError = $state('');

  onMount(() => {
    initializeTheme();
  });

  async function handleResendVerification() {
    resendLoading = true;
    resendMessage = '';
    resendError = '';

    try {
      const res = await resendVerification();
      if (!res.ok) {
        const data = await res.json().catch(() => null);
        throw new Error(data?.error || '認証メールの再送に失敗しました。');
      }
      resendMessage = '認証メールを再送しました。受信ボックスをご確認ください。';
      setTimeout(() => {
        resendMessage = '';
      }, 3000);
    } catch (e: any) {
      resendError = e.message || '認証メールの再送に失敗しました。';
    } finally {
      resendLoading = false;
    }
  }
</script>

<svelte:body class:dark={$theme === 'dark'} />

<div class="min-h-screen text-[var(--color-text)]">
  {#if $auth.user && $auth.user.email_verified === 0}
    <div class="sticky top-0 z-50 border-b border-amber-300/55 bg-[linear-gradient(120deg,rgba(255,251,235,0.94),rgba(254,243,199,0.88))] backdrop-blur dark:border-amber-900/80 dark:bg-[linear-gradient(120deg,rgba(69,39,0,0.85),rgba(120,53,15,0.72))]">
      <div class="mx-auto flex max-w-7xl flex-col gap-2 px-4 py-2.5 md:flex-row md:items-center md:justify-between">
        <p class="text-xs font-semibold text-amber-900 dark:text-amber-100">
          メールアドレスの認証が完了していません。認証メールを再送しますか？
        </p>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="rounded-lg border border-amber-400/70 bg-white/70 px-3 py-1.5 text-[11px] font-bold text-amber-900 transition hover:bg-white/95 disabled:cursor-not-allowed disabled:opacity-60 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-100 dark:hover:bg-amber-950/70"
            disabled={resendLoading}
            onclick={handleResendVerification}
          >
            {resendLoading ? '再送中...' : '認証メールを再送'}
          </button>
          {#if resendMessage}
            <p class="text-[11px] font-semibold text-emerald-700 dark:text-emerald-300">{resendMessage}</p>
          {:else if resendError}
            <p class="text-[11px] font-semibold text-red-600 dark:text-red-300">{resendError}</p>
          {/if}
        </div>
      </div>
    </div>
  {/if}
  {@render children()}
</div>
