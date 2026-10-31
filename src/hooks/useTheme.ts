import { useCallback, useSyncExternalStore } from 'react';
import { hexToTriplet, lighten } from '../theme/color';

export type Theme = 'dark' | 'light';

const THEME_KEY = 'capstone:theme';
const ACCENT_KEY = 'capstone:accent';

/**
 * Capstone opens dark on a machine that has never run it. Stack is dark-first
 * and this is its sibling, and dark is also the only theme that cannot flash
 * on the first paint: the stylesheet's `:root` block is the dark set, so the
 * paint that happens before this module runs is already correct for it.
 */
const DEFAULT_THEME: Theme = 'dark';

/**
 * Theme is client-only state. The IPC `Settings` contract carries no theme or
 * accent field, so it lives in localStorage and is applied to the DOM, never
 * sent to the backend. Both values are also applied before React mounts (see
 * main.tsx) so the first paint is not the default theme.
 */
export function applyTheme(theme: Theme, accent?: string | null) {
  const el = document.documentElement;
  el.setAttribute('data-theme', theme);
  applyAccent(accent);
  try {
    localStorage.setItem(THEME_KEY, theme);
    if (accent) localStorage.setItem(ACCENT_KEY, accent);
    else localStorage.removeItem(ACCENT_KEY);
  } catch {
    /* private mode / quota — the DOM is already correct for this session */
  }
  publish(theme);
}

/**
 * Overrides the accent vars inline on <html>. A falsy accent removes the
 * overrides so the stylesheet's own Capstone cyan applies again, including the
 * light theme's darker variant — which is why this sets the triplet rather than
 * a finished colour.
 *
 * One override serves both themes, so a hex picked while the app is dark is
 * the hex the light theme gets as well. That is a limit of the accent contract
 * — there is nowhere to keep a per-theme pair — not an oversight here.
 */
function applyAccent(accent?: string | null) {
  const el = document.documentElement;
  const triplet = accent ? hexToTriplet(accent) : null;
  if (accent && triplet) {
    el.style.setProperty('--capstone-sky', triplet);
    el.style.setProperty('--color-accent', `rgb(${triplet})`);
    el.style.setProperty('--color-accent-hover', lighten(accent, 0.18));
  } else {
    el.style.removeProperty('--capstone-sky');
    el.style.removeProperty('--color-accent');
    el.style.removeProperty('--color-accent-hover');
  }
}

export function readPersistedTheme(): Theme {
  try {
    return localStorage.getItem(THEME_KEY) === 'light' ? 'light' : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

export function readPersistedAccent(): string | null {
  try {
    return localStorage.getItem(ACCENT_KEY);
  } catch {
    return null;
  }
}

/*
 * One theme for the whole app, held outside React.
 *
 * This used to be `useState` inside the hook, which hands every caller its own
 * copy. The title bar's toggle repainted <html> for everyone, but any second
 * consumer — a status bar, an onboarding step, a settings pane — went on
 * rendering the icon and label of whichever theme the app was in when that
 * component mounted, and nothing recovered it short of a remount. There is one
 * consumer today; the next one is the bug.
 */
let current: Theme = readPersistedTheme();
const listeners = new Set<() => void>();

function publish(next: Theme) {
  if (next === current) return;
  current = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Current theme plus a toggle, for the title bar control. */
export function useTheme() {
  const theme = useSyncExternalStore(
    subscribe,
    () => current,
    () => DEFAULT_THEME,
  );

  const setTheme = useCallback((next: Theme) => {
    applyTheme(next, readPersistedAccent());
  }, []);

  const toggleTheme = useCallback(() => {
    applyTheme(current === 'dark' ? 'light' : 'dark', readPersistedAccent());
  }, []);

  return { theme, setTheme, toggleTheme };
}
