<script lang="ts">
    import { auth } from '../../lib/auth';
    import { goto } from '$app/navigation';

    let organization_name = '';
    let admin_name = '';
    let username = '';
    let email = '';
    let password = '';
    let showPassword = false;
    let error = '';
    let loading = false;

    function isSecurePassword(value: string): boolean {
        return /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[^A-Za-z0-9]).{8,}$/.test(value);
    }

    async function handleRegister() {
        if (!isSecurePassword(password)) {
            error = 'パスワードは8文字以上で、英大文字・英小文字・数字・記号をそれぞれ含めてください。';
            return;
        }

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
            auth.set({ token: data.token, user: data.user, initialized: true });
            goto('/');
        } catch (e: any) {
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<div class="auth-shell flex items-center justify-center p-4">
    <div class="auth-card w-full max-w-md p-8">
        <div class="mb-6 text-center">
            <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-500 text-xl font-black text-white shadow-lg shadow-blue-500/25">G</div>
            <h1 class="mb-1 text-3xl font-black tracking-tight text-[var(--text-primary)]">GlanceFlow</h1>
            <p class="text-xs font-semibold uppercase tracking-[0.18em] text-[var(--text-muted)]">New Workspace</p>
        </div>
        
        <form on:submit|preventDefault={handleRegister} class="space-y-4">
            <div>
                <label for="organization-name" class="mb-1.5 ml-1 block text-[10px] font-bold uppercase tracking-wide text-[var(--text-muted)]">組織名</label>
                <input id="organization-name" bind:value={organization_name} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="組織名を入力してください" />
            </div>
            
            <div class="border-t border-[var(--border-base)] pt-4">
                <h2 class="mb-3 ml-1 text-[10px] font-bold uppercase tracking-[0.14em] text-[var(--text-muted)]">管理者アカウント設定</h2>
                <div class="space-y-3">
                    <input bind:value={admin_name} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="お名前" />
                    <div>
                        <input bind:value={username} required pattern="^[a-zA-Z0-9_-]+$" class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="ユーザー名 (ログイン用)" />
                        <p class="ml-1 mt-1 text-[9px] text-[var(--text-muted)]">※半角英数字、_、- が使用可能です</p>
                    </div>
                    <input type="email" bind:value={email} required class="form-control px-4 py-2.5 text-sm focus:ring-2 transition-all" placeholder="メールアドレス" />
                    <div class="relative">
                        <input type={showPassword ? 'text' : 'password'} bind:value={password} required class="form-control px-4 py-2.5 pr-10 text-sm focus:ring-2 transition-all" placeholder="パスワード" />
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
                    <p class="ml-1 mt-1 text-[9px] text-[var(--text-muted)]">※8文字以上、英大文字・英小文字・数字・記号を各1文字以上含めてください</p>
                </div>
            </div>
            
            {#if error}
                <p class="mt-2 text-xs text-red-500 dark:text-red-300">{error}</p>
            {/if}
            
            <button
                type="submit"
                disabled={loading}
                class="btn-primary w-full py-3 text-sm"
            >
                {loading ? '作成中...' : '組織を作成して開始'}
            </button>
        </form>

        <div class="mt-6 text-center">
            <a href="/" class="text-xs font-semibold text-blue-600 hover:brightness-110">ログイン画面に戻る</a>
        </div>
    </div>
</div>
