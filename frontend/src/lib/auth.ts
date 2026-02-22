import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import type { AuthState, User } from './types';

const isBrowser = typeof window !== 'undefined';

/**
 * Safely parses the persisted auth user value.
 * Returns `null` when the value is missing or invalid.
 */
function parseStoredUser(rawValue: string | null): User | null {
  if (!rawValue) {
    return null;
  }

  try {
    return JSON.parse(rawValue) as User;
  } catch {
    return null;
  }
}

const initialToken = isBrowser ? localStorage.getItem('auth_token') : null;
const initialUser = isBrowser
  ? parseStoredUser(localStorage.getItem('auth_user'))
  : null;

/**
 * Global authentication store.
 *
 * Mechanism:
 * 1. Initialize from `localStorage` in the browser.
 * 2. Keep auth state in a Svelte writable store for reactive UI updates.
 * 3. Subscribe to changes and mirror them back to `localStorage`.
 */
export const auth: Writable<AuthState> = writable<AuthState>({
  token: initialToken,
  user: initialUser,
});

if (isBrowser) {
  auth.subscribe((value) => {
    if (value.token && value.user) {
      localStorage.setItem('auth_token', value.token);
      localStorage.setItem('auth_user', JSON.stringify(value.user));
    } else {
      localStorage.removeItem('auth_token');
      localStorage.removeItem('auth_user');
    }
  });
}

/**
 * Clears current authentication state for sign-out.
 */
export function logout() {
  auth.set({ token: null, user: null });
}
