<script lang="ts">
  import { theme, toggleTheme } from '$lib/theme';

  $: isDark = $theme === 'dark';
</script>

<button
  type="button"
  class="theme-toggle"
  aria-label={isDark ? 'ライトモードに切り替え' : 'ダークモードに切り替え'}
  aria-pressed={isDark}
  on:click={toggleTheme}
>
  <span class="theme-toggle__track"></span>

  <span class="theme-toggle__icon theme-toggle__icon--sun" aria-hidden="true">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2.5M12 19.5V22M4.93 4.93l1.77 1.77M17.3 17.3l1.77 1.77M2 12h2.5M19.5 12H22M4.93 19.07l1.77-1.77M17.3 6.7l1.77-1.77" />
    </svg>
  </span>

  <span class="theme-toggle__icon theme-toggle__icon--moon" aria-hidden="true">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z" />
    </svg>
  </span>

  <span class="theme-toggle__thumb" class:is-dark={isDark} aria-hidden="true"></span>
</button>

<style>
  .theme-toggle {
    position: relative;
    width: 56px;
    height: 32px;
    border: 1px solid color-mix(in srgb, var(--color-border) 88%, transparent);
    border-radius: 999px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    overflow: hidden;
    transition:
      border-color 260ms cubic-bezier(0.22, 1, 0.36, 1),
      transform 220ms cubic-bezier(0.22, 1, 0.36, 1),
      box-shadow 260ms cubic-bezier(0.22, 1, 0.36, 1);
    box-shadow:
      0 8px 16px -12px rgb(15 23 42 / 0.5),
      inset 0 1px 0 rgb(255 255 255 / 0.3);
  }

  .theme-toggle:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--color-border) 65%, #60a5fa 35%);
    box-shadow:
      0 12px 22px -14px rgb(15 23 42 / 0.6),
      inset 0 1px 0 rgb(255 255 255 / 0.35);
  }

  .theme-toggle:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 3px rgb(59 130 246 / 0.2),
      0 12px 22px -14px rgb(15 23 42 / 0.6),
      inset 0 1px 0 rgb(255 255 255 / 0.35);
  }

  .theme-toggle__track {
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, #dbeafe 0%, #fef3c7 100%);
    transition: background 320ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  :global(.dark) .theme-toggle__track {
    background: linear-gradient(135deg, #0f172a 0%, #1e293b 45%, #334155 100%);
  }

  .theme-toggle__icon {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    transform: translateY(-50%);
    transition:
      opacity 240ms ease,
      transform 280ms cubic-bezier(0.22, 1, 0.36, 1),
      color 240ms ease;
    z-index: 2;
  }

  .theme-toggle__icon svg {
    width: 100%;
    height: 100%;
  }

  .theme-toggle__icon--sun {
    left: 10px;
    color: #f59e0b;
    opacity: 1;
    transform: translateY(-50%) scale(1);
  }

  .theme-toggle__icon--moon {
    right: 9px;
    color: #cbd5e1;
    opacity: 0.45;
    transform: translateY(-50%) scale(0.9);
  }

  :global(.dark) .theme-toggle__icon--sun {
    opacity: 0.45;
    transform: translateY(-50%) scale(0.9);
    color: #fbbf24;
  }

  :global(.dark) .theme-toggle__icon--moon {
    opacity: 1;
    transform: translateY(-50%) scale(1);
    color: #e2e8f0;
  }

  .theme-toggle__thumb {
    position: absolute;
    left: 3px;
    top: 3px;
    width: 24px;
    height: 24px;
    border-radius: 999px;
    background: linear-gradient(160deg, #fffef5 0%, #fde68a 100%);
    border: 1px solid rgb(255 255 255 / 0.7);
    box-shadow:
      0 8px 16px -10px rgb(245 158 11 / 0.7),
      inset 0 1px 0 rgb(255 255 255 / 0.75);
    transition:
      transform 300ms cubic-bezier(0.34, 1.4, 0.64, 1),
      background 260ms cubic-bezier(0.22, 1, 0.36, 1),
      box-shadow 260ms cubic-bezier(0.22, 1, 0.36, 1);
    z-index: 3;
  }

  .theme-toggle__thumb.is-dark {
    transform: translateX(24px);
    background: linear-gradient(160deg, #e2e8f0 0%, #94a3b8 100%);
    box-shadow:
      0 10px 18px -11px rgb(15 23 42 / 0.85),
      inset 0 1px 0 rgb(255 255 255 / 0.4);
  }
</style>
