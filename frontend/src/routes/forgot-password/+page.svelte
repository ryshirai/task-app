<script lang="ts">
  import { apiFetch } from '$lib/api';
    let identity = '';
    let loading = false;
    let error = '';
    let success = false;

    async function handleSubmit() {
        loading = true;
        error = '';
        success = false;
        try {
            const res = await apiFetch('/api/auth/forgot-password', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ identity })
            });

            if (!res.ok) {
                const data = await res.json();
                throw new Error(data.error || 'リクエストの送信に失敗しました。');
            }
            success = true;
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="auth-shell flex items-center justify-center p-4">
    <div class="auth-card w-full max-w-md p-8">
        <h1 class="mb-2 text-center text-2xl font-black tracking-tight text-[var(--text-primary)]">パスワード再設定</h1>
        
        {#if success}
            <div class="rounded-2xl border border-emerald-300/60 bg-emerald-100/55 p-4 text-center dark:border-emerald-900 dark:bg-emerald-950/30">
                <p class="mb-2 font-bold text-emerald-700 dark:text-emerald-200">リクエストを送信しました</p>
                <p class="text-xs leading-relaxed text-emerald-700/85 dark:text-emerald-200/85">
                    入力されたアカウントが存在する場合、再設定用のリンクを記載したメールを送信しました。
                </p>
                <a href="/" class="mt-4 block text-xs font-bold text-blue-600 hover:brightness-110">ログイン画面に戻る</a>
            </div>
        {:else}
            <p class="mb-6 text-center text-sm text-[var(--text-muted)]">ログインに使用しているユーザー名またはメールアドレスを入力してください</p>
            <form on:submit|preventDefault={handleSubmit} class="space-y-4">
                <input bind:value={identity} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="ユーザー名またはメールアドレス" />
                
                {#if error}
                    <p class="text-xs text-red-500 dark:text-red-300">{error}</p>
                {/if}

                <button
                    type="submit"
                    disabled={loading}
                    class="btn-primary w-full py-3 text-sm"
                >
                    {loading ? '送信中...' : '再設定リクエストを送信'}
                </button>
            </form>
            <div class="mt-6 text-center">
                <a href="/" class="text-xs font-semibold text-blue-600 hover:brightness-110">ログイン画面に戻る</a>
            </div>
        {/if}
    </div>
</div>
