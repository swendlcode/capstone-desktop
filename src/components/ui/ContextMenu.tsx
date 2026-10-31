import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { ReactNode } from 'react';
import { ArrowRight2 } from './icons';

export interface ContextMenuItem {
  label: string;
  /** Not called when `items` is present: a parent row only opens its submenu. */
  onSelect?: () => void;
  icon?: ReactNode;
  /** Right-aligned hint, for a shortcut or a short value. Keep it to a word. */
  hint?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Turns the row into a submenu parent. */
  items?: ContextMenuEntry[];
}

export interface ContextMenuSeparator {
  separator: true;
}

export type ContextMenuEntry = ContextMenuItem | ContextMenuSeparator;

const isSeparator = (e: ContextMenuEntry): e is ContextMenuSeparator =>
  (e as ContextMenuSeparator).separator === true;

interface ContextMenuProps {
  /** Viewport coordinates, straight from the event. */
  x: number;
  y: number;
  items: ContextMenuEntry[];
  onClose: () => void;
}

const PANEL =
  'min-w-[190px] overflow-visible rounded-lg border border-gray-600 bg-gray-800 py-1 ' +
  'shadow-[0_12px_36px_rgba(0,0,0,0.45)] outline-none';

/**
 * Right-click menu, portalled to `document.body` so no ancestor's `overflow`
 * or `transform` can clip it, and clamped to the viewport after measuring —
 * a menu opened near the bottom edge would otherwise run off-screen with no
 * way to scroll to it.
 */
export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y, ready: false });

  // Layout effect, not effect: the menu is painted once at the clamped
  // position rather than flashing at the raw one first.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.max(8, Math.min(x, window.innerWidth - width - 8)),
      top: Math.max(8, Math.min(y, window.innerHeight - height - 8)),
      ready: true,
    });
  }, [x, y, items]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    // Capture phase, so a row that stops propagation cannot trap the menu open.
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('resize', onClose);
    window.addEventListener('blur', onClose);
    // Scroll closes it, in capture because a scroll event does not bubble. The
    // menu is positioned once, in viewport coordinates, and the row it belongs
    // to is in a virtualized list: scrolling moves that row out from under it
    // and leaves the menu acting on a file nobody can see.
    window.addEventListener('scroll', onClose, true);
    window.addEventListener('wheel', onClose, true);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('resize', onClose);
      window.removeEventListener('blur', onClose);
      window.removeEventListener('scroll', onClose, true);
      window.removeEventListener('wheel', onClose, true);
    };
  }, [onClose]);

  return createPortal(
    <div
      ref={ref}
      role="menu"
      style={{ left: pos.left, top: pos.top, visibility: pos.ready ? 'visible' : 'hidden' }}
      className={`fixed z-[9999] ${PANEL}`}
      onContextMenu={(e) => e.preventDefault()}
    >
      <MenuRows entries={items} onClose={onClose} />
    </div>,
    document.body,
  );
}

function MenuRows({
  entries,
  onClose,
}: {
  entries: ContextMenuEntry[];
  onClose: () => void;
}) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <>
      {entries.map((entry, i) =>
        isSeparator(entry) ? (
          <div key={`sep-${i}`} className="mx-2 my-1 border-t border-gray-700" />
        ) : (
          <MenuRow
            key={`${entry.label}-${i}`}
            item={entry}
            open={openIndex === i}
            onHover={() => setOpenIndex(entry.items ? i : null)}
            onClose={onClose}
          />
        ),
      )}
    </>
  );
}

function MenuRow({
  item,
  open,
  onHover,
  onClose,
}: {
  item: ContextMenuItem;
  open: boolean;
  onHover: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const [flip, setFlip] = useState(false);

  // A submenu near the right edge opens leftwards instead of off-screen.
  useLayoutEffect(() => {
    if (!open || !ref.current) return;
    setFlip(ref.current.getBoundingClientRect().right + 200 > window.innerWidth);
  }, [open]);

  // The palette's danger token, not Tailwind's stock red: the stock shades do
  // not follow the light theme and read as a different red beside the queue's
  // own error rows.
  // A disabled row (Open output before a file has one) steps down to the
  // subtle ink token rather than fading with `opacity`: 4.0:1 on the panel in
  // dark instead of the 2.5:1 that `opacity-35` bought, and it cannot compound
  // with a dimmed ancestor the way the template cards' rows once did.
  const tone = item.disabled
    ? 'text-gray-500'
    : item.danger
      ? 'text-capstone-red hover:bg-capstone-red/10'
      : 'text-capstone-white hover:bg-gray-700';

  return (
    <div className="relative" onMouseEnter={onHover}>
      <button
        ref={ref}
        role="menuitem"
        type="button"
        disabled={item.disabled}
        onClick={(e) => {
          e.stopPropagation();
          if (item.items) return;
          item.onSelect?.();
          onClose();
        }}
        className={`flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-sm transition-colors disabled:cursor-not-allowed ${tone}`}
      >
        {item.icon ? <span className="flex w-4 shrink-0 justify-center">{item.icon}</span> : null}
        <span className="flex-1 truncate">{item.label}</span>
        {item.hint ? <span className="font-mono text-xs text-gray-400">{item.hint}</span> : null}
        {item.items ? <ArrowRight2 size={14} className="shrink-0 text-gray-400" /> : null}
      </button>
      {open && item.items ? (
        <div className={`absolute top-0 z-10 ${flip ? 'right-full mr-1' : 'left-full ml-1'} ${PANEL}`}>
          <MenuRows entries={item.items} onClose={onClose} />
        </div>
      ) : null}
    </div>
  );
}

// Lives next door, so this file stays inside the component size cap. Re-exported
// here because the hook and the menu are one feature at the call site.
export { useContextMenu } from './useContextMenu';
