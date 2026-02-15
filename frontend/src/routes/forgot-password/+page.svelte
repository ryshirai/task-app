<script lang="ts">
    let username = '';
    let loading = false;
    let error = '';
    let success = false;

    async function handleSubmit() {
        loading = true;
        error = '';
        success = false;
        try {
            const res = await fetch('http://localhost:3000/api/auth/forgot-password', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username })
            });

            if (!res.ok) throw new Error('ユーザーが見つかりません。');
            success = true;
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="min-h-screen flex items-center justify-center bg-slate-50 p-4">
    <div class="max-w-md w-full p-8 bg-white rounded-2xl shadow-xl border border-slate-200">
        <h1 class="text-2xl font-bold text-slate-800 mb-2 text-center">パスワード再設定</h1>
        
        {#if success}
            <div class="bg-emerald-50 border border-emerald-100 p-4 rounded-xl text-center">
                <p class="text-emerald-700 font-bold mb-2">リクエストを送信しました</p>
                <p class="text-emerald-600 text-xs leading-relaxed">
                    システムログにリセットトークンが出力されました。<br/>
                    (デモ版のためメール送信は行われません)
                </p>
                <a href="/" class="block mt-4 text-blue-600 hover:underline text-xs font-bold">ログイン画面に戻る</a>
            </div>
        {:else}
            <p class="text-slate-500 text-sm text-center mb-6">ログインに使用しているユーザー名を入力してください</p>
            <form on:submit|preventDefault={handleSubmit} class="space-y-4">
                <input bind:value={username} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500" placeholder="ユーザー名" />
                
                {#if error}
                    <p class="text-red-500 text-xs">{error}</p>
                {/if}

                <button
                    type="submit"
                    disabled={loading}
                    class="w-full py-3 bg-slate-800 hover:bg-slate-900 text-white font-bold rounded-lg transition-colors shadow-lg shadow-slate-100 disabled:opacity-50"
                >
                    {loading ? '送信中...' : '再設定リクエストを送信'}
                </button>
            </form>
            <div class="mt-6 text-center">
                <a href="/" class="text-xs text-blue-600 hover:underline">ログイン画面に戻る</a>
            </div>
        {/if}
    </div>
</div>
