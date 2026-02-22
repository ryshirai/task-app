import { browser } from '$app/environment';
import { writable, type Writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

const THEME_STORAGE_KEY = 'glanceflow_theme';

function isTheme(value: string | null): value is Theme {
  return value === 'light' || value === 'dark';
}

function resolveInitialTheme(): Theme {
  if (!browser) {
    return 'light';
  }

  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (isTheme(stored)) {
    return stored;
  }

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyThemeClass(nextTheme: Theme): void {
  if (!browser) {
    return;
  }

  const isDark = nextTheme === 'dark';
  document.documentElement.classList.toggle('dark', isDark);
  document.documentElement.style.colorScheme = isDark ? 'dark' : 'light';

  if (document.body) {
    document.body.classList.toggle('dark', isDark);
  }
}

const initialTheme = resolveInitialTheme();

export const theme: Writable<Theme> = writable<Theme>(initialTheme);

if (browser) {
  theme.subscribe((value) => {
    applyThemeClass(value);
    localStorage.setItem(THEME_STORAGE_KEY, value);
  });
}

export function initializeTheme(): void {
  const initial = resolveInitialTheme();
  theme.set(initial);
  applyThemeClass(initial);
}

export function setTheme(nextTheme: Theme): void {
  theme.set(nextTheme);
}

export function toggleTheme(): void {
  theme.update((current) => (current === 'dark' ? 'light' : 'dark'));
}
