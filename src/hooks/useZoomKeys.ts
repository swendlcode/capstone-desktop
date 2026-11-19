import { useEffect } from 'react';
import { zoomIn, zoomOut, zoomReset } from '../lib/appZoom';

/**
 * Cmd/Ctrl +, -, 0 anywhere in the window.
 *
 * Mounted at the root and bound with no field guard on purpose: zoom has to
 * work while a template name is being typed and while a modal is open, the
 * same way it does in a browser. The capture phase keeps it ahead of the
 * dialogs that take the keyboard for themselves (Modal, ContextMenu,
 * TemplateMenu, MultibandEditor all bind keydown of their own).
 */
export function useZoomKeys() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey && !e.ctrlKey) return;
      // `=` is the unshifted key that carries `+`; `code` covers the numpad and
      // keeps the shortcut working on layouts that put `+` somewhere else.
      const bigger = e.key === '+' || e.key === '=' || e.code === 'NumpadAdd';
      const smaller = e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract';
      const reset = e.key === '0' || e.code === 'Numpad0';
      if (!bigger && !smaller && !reset) return;
      e.preventDefault();
      if (bigger) zoomIn();
      else if (smaller) zoomOut();
      else zoomReset();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, []);
}
