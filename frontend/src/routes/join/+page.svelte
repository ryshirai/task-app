<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import { auth } from '../../lib/auth';
    import { goto } from '$app/navigation';
    import type { Invitation } from '../../lib/types';

    let token = '';
    let invitation: Invitation | null = null;
    let loading = true;
    let error = '';

    let name = '';
    let username = '';
    let email = '';
    let password = '';
    let joining = false;

    onMount(async () => {
        token = page.url.searchParams.get('token') || '';
        if (!token) {
            error = '招待トークンが見つかりません。';
            loading = false;
            return;
        }

        try {
            const res = await fetch(`http://localhost:3000/api/invitations/${token}`);
            if (!res.ok) throw new Error('招待が無効か、期限が切れています。');
            invitation = await res.json();
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    });

    async function handleJoin() {
        joining = true;
        error = '';
        try {
            const res = await fetch('http://localhost:3000/api/auth/join', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ token, name, username, email, password })
            });

            if (!res.ok) {
                const data = await res.text();
                throw new Error(data || '参加に失敗しました');
            }

            const data = await res.json();
            auth.set({ token: data.token, user: data.user });
            goto('/');
        } catch (e: any) {
            error = e.message;
        } finally {
            joining = false;
        }
    }
</script>

<div class="min-h-screen flex items-center justify-center bg-slate-50 p-4">
    <div class="max-w-md w-full p-8 bg-white rounded-2xl shadow-xl border border-slate-200">
        {#if loading}
            <div class="text-center text-slate-400 py-10 animate-pulse font-bold text-lg">招待を確認中...</div>
        {:else if error}
            <div class="text-center">
                <div class="text-red-500 font-bold mb-4">{error}</div>
                <a href="/" class="text-blue-600 hover:underline text-sm">トップページへ</a>
            </div>
        {:else}
            <h1 class="text-2xl font-bold text-slate-800 mb-1 text-center">GlanceFlow</h1>
            <p class="text-slate-500 text-xs text-center mb-4">
                <span class="font-bold text-blue-600">{invitation?.org_name}</span> への招待
            </p>
            <p class="text-slate-400 text-[10px] text-center mb-6 uppercase tracking-widest font-bold">アカウントを作成して参加しましょう</p>
            
            <form on:submit|preventDefault={handleJoin} class="space-y-4">
                <input bind:value={name} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="表示名 (例: 山田太郎)" />
                <div>
                    <input bind:value={username} required pattern="^[a-zA-Z0-9_-]+$" class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="ユーザー名 (英数字・ハイフン・アンダースコア)" />
                    <p class="text-[9px] text-slate-400 mt-1 ml-1">※半角英数字、_、- が使用可能です</p>
                </div>
                <input type="email" bind:value={email} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="メールアドレス" />
                <input type="password" bind:value={password} required class="w-full px-4 py-2 border border-slate-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 text-sm" placeholder="パスワード" />
                
                <button
                    type="submit"
                    disabled={joining}
                    class="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-lg transition-colors shadow-lg shadow-blue-100 disabled:opacity-50"
                >
                    {joining ? '参加中...' : 'アカウントを作成して参加'}
                </button>
            </form>
        {/if}
    </div>
</div>
