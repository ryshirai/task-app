<script lang="ts">
  import { apiFetch } from '$lib/api';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';

    let token = '';
    let newPassword = '';
    let confirmPassword = '';
    let loading = false;
    let error = '';
    let success = false;

    onMount(() => {
        token = page.url.searchParams.get('token') || '';
        if (!token) {
            error = 'リセットトークンが見つかりません。';
        }
    });

    async function handleReset() {
        if (newPassword !== confirmPassword) {
            error = 'パスワードが一致しません。';
            return;
        }

        loading = true;
        error = '';
        try {
            const res = await apiFetch('/api/auth/reset-password', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ token, new_password: newPassword })
            });

            if (!res.ok) throw new Error('トークンが無効か、期限が切れています。');
            success = true;
            setTimeout(() => goto('/'), 2000);
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="auth-shell flex items-center justify-center p-4">
    <div class="auth-card w-full max-w-md p-8">
        <h1 class="mb-2 text-center text-2xl font-black tracking-tight text-[var(--text-primary)]">新しいパスワードの設定</h1>
        
        {#if success}
            <div class="rounded-2xl border border-emerald-300/60 bg-emerald-100/55 p-4 text-center dark:border-emerald-900 dark:bg-emerald-950/30">
                <p class="font-bold text-emerald-700 dark:text-emerald-200">更新が完了しました</p>
                <p class="mt-1 text-xs text-emerald-700/85 dark:text-emerald-200/85">ログイン画面に移動します...</p>
            </div>
        {:else if error && !token}
            <div class="text-center">
                <p class="mb-4 font-bold text-red-500 dark:text-red-300">{error}</p>
                <a href="/" class="text-sm font-semibold text-blue-600 hover:brightness-110">トップページへ</a>
            </div>
        {:else}
            <form on:submit|preventDefault={handleReset} class="space-y-4">
                <input type="password" bind:value={newPassword} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="新しいパスワード" />
                <input type="password" bind:value={confirmPassword} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="確認用パスワード" />
                
                {#if error}
                    <p class="text-xs text-red-500 dark:text-red-300">{error}</p>
                {/if}

                <button
                    type="submit"
                    disabled={loading}
                    class="btn-primary w-full py-3 text-sm"
                >
                    {loading ? '更新中...' : 'パスワードを更新'}
                </button>
            </form>
        {/if}
    </div>
</div>
