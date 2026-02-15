<script lang="ts">
    import { auth } from '../auth';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    let username = '';
    let password = '';
    let error = '';
    let loading = false;

    async function handleLogin() {
        loading = true;
        error = '';
        try {
            const res = await fetch('http://localhost:3000/api/auth/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username, password })
            });

            if (!res.ok) {
                const data = await res.text();
                throw new Error(data || 'ログインに失敗しました');
            }

            const data = await res.json();
            auth.set({ token: data.token, user: data.user });
            dispatch('loginSuccess');
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="min-h-screen flex items-center justify-center bg-slate-50">
    <div class="max-w-md w-full p-8 bg-white rounded-2xl shadow-xl border border-slate-200">
        <h1 class="text-3xl font-black text-slate-800 mb-1 text-center tracking-tighter">GlanceFlow</h1>
        <p class="text-slate-400 text-[10px] text-center mb-8 uppercase tracking-[0.2em] font-bold">Team Timeline Dashboard</p>
        
        <form on:submit|preventDefault={handleLogin} class="space-y-4">
            <div>
                <label for="username" class="block text-sm font-medium text-slate-700 mb-1">ユーザー名</label>
                <input
                    id="username"
                    type="text"
                    bind:value={username}
                    required
                    class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-all outline-none"
                    placeholder="ユーザー名"
                />
            </div>
            
            <div>
                <label for="password" class="block text-sm font-medium text-slate-700 mb-1">パスワード</label>
                <input
                    id="password"
                    type="password"
                    bind:value={password}
                    required
                    class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-all outline-none"
                    placeholder="••••••••"
                />
            </div>
            
            {#if error}
                <p class="text-red-500 text-sm mt-2">{error}</p>
            {/if}
            
            <button
                type="submit"
                disabled={loading}
                class="w-full py-3 px-4 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-lg transition-colors shadow-lg shadow-blue-200 disabled:opacity-50"
            >
                {loading ? 'ログイン中...' : 'ログイン'}
            </button>
        </form>

        <div class="mt-6 flex flex-col gap-3 text-center">
            <a href="/register" class="text-xs text-blue-600 font-bold hover:underline">新しい組織を作成する</a>
            <a href="/forgot-password" class="text-xs text-slate-400 hover:text-slate-600">パスワードを忘れた場合</a>
        </div>
    </div>
</div>
