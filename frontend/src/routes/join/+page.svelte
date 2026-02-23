<script lang="ts">
  import { apiFetch } from '$lib/api';
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

    function isSecurePassword(value: string): boolean {
        return /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[^A-Za-z0-9]).{8,}$/.test(value);
    }

    onMount(async () => {
        token = page.url.searchParams.get('token') || '';
        if (!token) {
            error = '招待トークンが見つかりません。';
            loading = false;
            return;
        }

        try {
            const res = await apiFetch(`/api/invitations/${token}`);
            if (!res.ok) throw new Error('招待が無効か、期限が切れています。');
            invitation = await res.json();
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    });

    async function handleJoin() {
        if (!isSecurePassword(password)) {
            error = 'パスワードは8文字以上で、英大文字・英小文字・数字・記号をそれぞれ含めてください。';
            return;
        }

        joining = true;
        error = '';
        try {
            const res = await apiFetch('/api/auth/join', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ token, name, username, email, password })
            });

            if (!res.ok) {
                const data = await res.text();
                throw new Error(data || '参加に失敗しました');
            }

            const data = await res.json();
            auth.set({ token: data.token, user: data.user, initialized: true });
            goto('/');
        } catch (e: any) {
            error = e.message;
        } finally {
            joining = false;
        }
    }
</script>

<div class="auth-shell flex items-center justify-center p-4">
    <div class="auth-card w-full max-w-md p-8">
        {#if loading}
            <div class="py-10 text-center text-lg font-bold text-[var(--text-muted)] animate-pulse">招待を確認中...</div>
        {:else if error}
            <div class="text-center">
                <div class="mb-4 font-bold text-red-500 dark:text-red-300">{error}</div>
                <a href="/" class="text-sm font-semibold text-blue-600 hover:brightness-110">トップページへ</a>
            </div>
        {:else}
            <div class="mb-6 text-center">
                <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-500 text-xl font-black text-white shadow-lg shadow-blue-500/25">G</div>
                <h1 class="mb-1 text-2xl font-black tracking-tight text-[var(--text-primary)]">GlanceFlow</h1>
                <p class="text-xs text-[var(--text-muted)]">
                    <span class="font-bold text-blue-600">{invitation?.org_name ?? '組織'}</span> への招待
                </p>
                <p class="mt-2 text-[10px] font-bold uppercase tracking-[0.14em] text-[var(--text-muted)]">アカウントを作成して参加しましょう</p>
            </div>
            
            <form on:submit|preventDefault={handleJoin} class="space-y-4">
                <input bind:value={name} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="表示名 (例: 山田太郎)" />
                <div>
                    <input bind:value={username} required pattern="^[a-zA-Z0-9_-]+$" class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="ユーザー名 (英数字・ハイフン・アンダースコア)" />
                    <p class="ml-1 mt-1 text-[9px] text-[var(--text-muted)]">※半角英数字、_、- が使用可能です</p>
                </div>
                <input type="email" bind:value={email} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="メールアドレス" />
                <div>
                    <input type="password" bind:value={password} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="パスワード" />
                    <p class="ml-1 mt-1 text-[9px] text-[var(--text-muted)]">※8文字以上、英大文字・英小文字・数字・記号を各1文字以上含めてください</p>
                </div>

                {#if error}
                    <p class="text-xs text-red-500 dark:text-red-300">{error}</p>
                {/if}
                
                <button
                    type="submit"
                    disabled={joining}
                    class="btn-primary w-full py-3 text-sm"
                >
                    {joining ? '参加中...' : 'アカウントを作成して参加'}
                </button>
            </form>
        {/if}
    </div>
</div>
