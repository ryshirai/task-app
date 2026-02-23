<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { apiFetch } from '$lib/api';
  import { auth } from '$lib/auth';
  import { onMount } from 'svelte';

  let verifying = true;
  let success = false;
  let error = '';

  onMount(async () => {
    const token = page.url.searchParams.get('token') || '';

    if (!token) {
      verifying = false;
      error = '認証トークンが見つかりません。メール内のリンクをご確認ください。';
      return;
    }

    try {
      const res = await apiFetch('/api/auth/verify-email', {
        method: 'POST',
        body: JSON.stringify({ token })
      });

      if (!res.ok) {
        const data = await res.json().catch(() => null);
        throw new Error(data?.error || 'メール認証に失敗しました。リンクの期限切れをご確認ください。');
      }

      success = true;
      auth.update((state) =>
        state.user ? { ...state, user: { ...state.user, email_verified: 1 } } : state
      );
    } catch (e: any) {
      error = e.message || 'メール認証に失敗しました。';
    } finally {
      verifying = false;
    }
  });
</script>

<div class="auth-shell flex items-center justify-center p-4">
  <div class="auth-card w-full max-w-md p-8 text-center">
    <h1 class="mb-2 text-2xl font-black tracking-tight text-[var(--text-primary)]">メール認証</h1>
    {#if verifying}
      <p class="text-sm text-[var(--text-muted)]">認証処理を実行しています...</p>
    {:else if success}
      <div class="rounded-2xl border border-emerald-300/60 bg-emerald-100/55 p-4 dark:border-emerald-900 dark:bg-emerald-950/30">
        <p class="font-bold text-emerald-700 dark:text-emerald-200">メール認証が完了しました。</p>
        <p class="mt-1 text-xs text-emerald-700/85 dark:text-emerald-200/85">
          引き続きGlanceFlowをご利用いただけます。
        </p>
      </div>
      <button class="btn-primary mt-6 w-full py-2.5 text-sm" onclick={() => goto('/')}>
        ホームへ戻る
      </button>
    {:else}
      <div class="rounded-2xl border border-red-300/60 bg-red-100/55 p-4 dark:border-red-900 dark:bg-red-950/30">
        <p class="font-bold text-red-600 dark:text-red-300">認証できませんでした</p>
        <p class="mt-1 text-xs text-red-600/85 dark:text-red-200/85">{error}</p>
      </div>
      <button class="btn-secondary mt-6 w-full py-2.5 text-sm" onclick={() => goto('/')}>
        トップページへ
      </button>
    {/if}
  </div>
</div>
