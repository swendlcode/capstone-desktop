import { useCallback, useState } from 'react';
import { hexToTriplet, lighten } from '../theme/color';

export type Theme = 'dark' | 'light';

const THEME_KEY = 'capstone:theme';
const ACCENT_KEY = 'capstone:accent';

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
}

/**
 * Overrides the accent vars inline on <html>. A falsy accent removes the
 * overrides so the stylesheet's own Capstone cyan applies again, including the
 * light theme's darker variant — which is why this sets the triplet rather than
 * a finished colour.
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
    return localStorage.getItem(THEME_KEY) === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

export function readPersistedAccent(): string | null {
  try {
    return localStorage.getItem(ACCENT_KEY);
  } catch {
    return null;
  }
}

/** Current theme plus a toggle, for the title bar control. */
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(readPersistedTheme);

  const setAndApply = useCallback((next: Theme) => {
    applyTheme(next, readPersistedAccent());
    setTheme(next);
  }, []);

  const toggleTheme = useCallback(() => {
    setAndApply(readPersistedTheme() === 'dark' ? 'light' : 'dark');
  }, [setAndApply]);

  return { theme, setTheme: setAndApply, toggleTheme };
}
