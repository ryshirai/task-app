<script lang="ts">
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
            const res = await fetch('http://localhost:3000/api/auth/reset-password', {
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

<div class="min-h-screen flex items-center justify-center bg-slate-50 p-4">
    <div class="max-w-md w-full p-8 bg-white rounded-2xl shadow-xl border border-slate-200">
        <h1 class="text-2xl font-bold text-slate-800 mb-2 text-center">新しいパスワードの設定</h1>
        
        {#if success}
            <div class="bg-emerald-50 border border-emerald-100 p-4 rounded-xl text-center">
                <p class="text-emerald-700 font-bold">更新が完了しました</p>
                <p class="text-emerald-600 text-xs mt-1">ログイン画面に移動します...</p>
            </div>
        {:else if error && !token}
            <div class="text-center">
                <p class="text-red-500 font-bold mb-4">{error}</p>
                <a href="/" class="text-blue-600 hover:underline text-sm">トップページへ</a>
            </div>
        {:else}
            <form on:submit|preventDefault={handleReset} class="space-y-4">
                <input type="password" bind:value={newPassword} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500" placeholder="新しいパスワード" />
                <input type="password" bind:value={confirmPassword} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500" placeholder="確認用パスワード" />
                
                {#if error}
                    <p class="text-red-500 text-xs">{error}</p>
                {/if}

                <button
                    type="submit"
                    disabled={loading}
                    class="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-lg transition-colors shadow-lg shadow-blue-100 disabled:opacity-50"
                >
                    {loading ? '更新中...' : 'パスワードを更新'}
                </button>
            </form>
        {/if}
    </div>
</div>
