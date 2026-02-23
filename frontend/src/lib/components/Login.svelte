<script lang="ts">
  import { apiFetch } from '$lib/api';
    import { auth } from '../auth';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    let username = '';
    let password = '';
    let showPassword = false;
    let error = '';
    let loading = false;

    async function handleLogin() {
        loading = true;
        error = '';
        try {
            const res = await apiFetch('/api/auth/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username, password })
            });

            if (!res.ok) {
                const data = await res.json();
                throw new Error(data.error || 'ログインに失敗しました');
            }

            const data = await res.json();
            auth.set({ token: data.token, user: data.user, initialized: true });
            dispatch('loginSuccess');
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="auth-shell flex items-center justify-center p-4">
    <div class="auth-card w-full max-w-md p-8">
        <div class="mb-8 text-center">
            <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-500 text-xl font-black text-white shadow-lg shadow-blue-500/25">G</div>
            <h1 class="mb-1 text-3xl font-black tracking-tight text-[var(--text-primary)]">GlanceFlow</h1>
            <p class="text-[10px] font-bold uppercase tracking-[0.18em] text-[var(--text-muted)]">Team Timeline Dashboard</p>
        </div>
        
        <form on:submit|preventDefault={handleLogin} class="space-y-4">
            <div>
                <label for="username" class="mb-1.5 block text-xs font-semibold text-[var(--text-muted)]">ユーザー名 または メールアドレス</label>
                <input
                    id="username"
                    type="text"
                    bind:value={username}
                    required
                    class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all"
                    placeholder="ユーザー名 または メールアドレス"
                />
            </div>
            
            <div>
                <label for="password" class="mb-1.5 block text-xs font-semibold text-[var(--text-muted)]">パスワード</label>
                <div class="relative">
                    <input
                        id="password"
                        type={showPassword ? 'text' : 'password'}
                        bind:value={password}
                        required
                        class="form-control px-4 py-2.5 pr-10 text-sm focus:ring-2 transition-all"
                        placeholder="••••••••"
                    />
                    <button
                        type="button"
                        class="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-[var(--text-muted)] transition-colors hover:text-[var(--text-primary)]"
                        aria-label={showPassword ? 'パスワードを隠す' : 'パスワードを表示'}
                        on:click={() => (showPassword = !showPassword)}
                    >
                        {#if showPassword}
                            <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.58 10.58a2 2 0 1 0 2.83 2.83"></path><path d="M9.88 4.24A10.94 10.94 0 0 1 12 4c7 0 10 8 10 8a17.22 17.22 0 0 1-2.17 3.19"></path><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 8 10 8a10.94 10.94 0 0 0 5.76-1.62"></path><line x1="2" y1="2" x2="22" y2="22"></line></svg>
                        {:else}
                            <svg xmlns="http://www.w3.org/2000/svg" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.06 12a11 11 0 0 1 19.88 0 11 11 0 0 1-19.88 0z"></path><circle cx="12" cy="12" r="3"></circle></svg>
                        {/if}
                    </button>
                </div>
            </div>
            
            {#if error}
                <p class="mt-2 text-sm text-red-500 dark:text-red-300">{error}</p>
            {/if}
            
            <button
                type="submit"
                disabled={loading}
                class="btn-primary w-full px-4 py-3 text-sm"
            >
                {loading ? 'ログイン中...' : 'ログイン'}
            </button>
        </form>

        <div class="mt-6 flex flex-col gap-3 text-center">
            <a href="/register" class="text-xs font-bold text-blue-600 hover:brightness-110">新しい組織を作成する</a>
            <a href="/forgot-password" class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]">パスワードを忘れた場合</a>
        </div>
    </div>
</div>
