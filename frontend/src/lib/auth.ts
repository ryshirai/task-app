import { writable } from 'svelte/store';
import type { AuthState, User } from './types';

const isBrowser = typeof window !== 'undefined';

const initialToken = isBrowser ? localStorage.getItem('auth_token') : null;
const initialUser = isBrowser ? JSON.parse(localStorage.getItem('auth_user') || 'null') : null;

export const auth = writable<AuthState>({
    token: initialToken,
    user: initialUser
});

if (isBrowser) {
    auth.subscribe(value => {
        if (value.token) {
            localStorage.setItem('auth_token', value.token);
            localStorage.setItem('auth_user', JSON.stringify(value.user));
        } else {
            localStorage.removeItem('auth_token');
            localStorage.removeItem('auth_user');
        }
    });
}

export function logout() {
    auth.set({ token: null, user: null });
}
