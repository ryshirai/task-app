<script lang="ts">
    import { auth } from '../../lib/auth';
    import { goto } from '$app/navigation';

    let organization_name = '';
    let admin_name = '';
    let username = '';
    let email = '';
    let password = '';
    let error = '';
    let loading = false;

    async function handleRegister() {
        loading = true;
        error = '';
        try {
            const res = await fetch('http://localhost:3000/api/auth/register', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ organization_name, admin_name, username, email, password })
            });

            if (!res.ok) {
                const data = await res.text();
                throw new Error(data || '登録に失敗しました');
            }

            const data = await res.json();
            auth.set({ token: data.token, user: data.user });
            goto('/');
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="min-h-screen flex items-center justify-center bg-slate-50 p-4">
    <div class="max-w-md w-full p-8 bg-white rounded-2xl shadow-xl border border-slate-200">
        <h1 class="text-3xl font-black text-slate-800 mb-1 text-center tracking-tighter">GlanceFlow</h1>
        <p class="text-slate-500 text-xs text-center mb-6 uppercase tracking-widest font-bold">New Workspace</p>
        
        <form on:submit|preventDefault={handleRegister} class="space-y-4">
            <div>
                <label class="block text-[10px] font-bold text-slate-400 uppercase mb-1 ml-1">組織名</label>
                <input bind:value={organization_name} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="株式会社サンプル" />
            </div>
            
            <div class="pt-4 border-t border-slate-100">
                <h2 class="text-[10px] font-bold text-slate-400 uppercase mb-3 ml-1 tracking-widest">管理者アカウント設定</h2>
                <div class="space-y-3">
                    <input bind:value={admin_name} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="お名前" />
                    <div>
                        <input bind:value={username} required pattern="^[a-zA-Z0-9_-]+$" class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="ユーザー名 (ログイン用)" />
                        <p class="text-[9px] text-slate-400 mt-1 ml-1">※半角英数字、_、- が使用可能です</p>
                    </div>
                    <input type="email" bind:value={email} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="メールアドレス" />
                    <input type="password" bind:value={password} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="パスワード" />
                </div>
            </div>
            
            {#if error}
                <p class="text-red-500 text-xs mt-2">{error}</p>
            {/if}
            
            <button
                type="submit"
                disabled={loading}
                class="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-lg transition-colors shadow-lg shadow-blue-100 disabled:opacity-50"
            >
                {loading ? '作成中...' : '組織を作成して開始'}
            </button>
        </form>

        <div class="mt-6 text-center">
            <a href="/" class="text-xs text-blue-600 hover:underline">ログイン画面に戻る</a>
        </div>
    </div>
</div>
