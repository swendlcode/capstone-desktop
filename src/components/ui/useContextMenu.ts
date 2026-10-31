import { useCallback, useState } from 'react';

/**
 * Holds a context menu's open position, so a consumer only writes the handler
 * and the JSX.
 *
 * Split from ContextMenu.tsx to keep that file inside the component size cap;
 * it is re-exported from there, which is where callers import it.
 */
export function useContextMenu() {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const close = useCallback(() => setAt(null), []);
  const open = useCallback((e: { clientX: number; clientY: number; preventDefault(): void }) => {
    e.preventDefault();
    setAt({ x: e.clientX, y: e.clientY });
  }, []);
  return { at, open, close };
}
