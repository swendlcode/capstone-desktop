import { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Files dropped onto the window, as filesystem paths.
 *
 * This listens to Tauri's own drag-drop event and not the DOM's. Two reasons,
 * and the second is the one that matters: with `dragDropEnabled` set in
 * tauri.conf.json the webview never receives an HTML `drop` at all, and even
 * where it does, a DOM `DataTransfer` carries a `File` handle rather than a
 * path — nothing the Rust side can open.
 *
 * Returns whether something is currently hovering, for the drop highlight.
 */
export function useFileDrop(onDrop: (paths: string[]) => void): boolean {
  const [isOver, setIsOver] = useState(false);

  // The callback is read through a ref so the listener is registered once. A
  // listener keyed on the callback identity re-registers on every render of the
  // page, and a drop landing mid-swap is delivered to nobody.
  const handler = useRef(onDrop);
  handler.current = onDrop;

  useEffect(() => {
    const unlisten = getCurrentWindow().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === 'enter' || payload.type === 'over') {
        setIsOver(true);
        return;
      }
      if (payload.type === 'leave') {
        setIsOver(false);
        return;
      }
      // 'drop'. Folders arrive as paths too; the backend expands them.
      setIsOver(false);
      if (payload.paths.length > 0) handler.current(payload.paths);
    });

    return () => {
      // Awaited rather than fire-and-forget: the handle only exists once the
      // registration has resolved, and a cleanup that gives up early leaves a
      // second listener behind on the next mount.
      unlisten.then((fn) => fn()).catch(() => {
        /* the window went away first; nothing to detach */
      });
    };
  }, []);

  return isOver;
}
