import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Copy, Edit2, Setting2, Trash } from '../ui/icons';

interface Props {
  /** Rename and Delete are a user template's only; stock offers Duplicate. */
  editable: boolean;
  isDuplicating: boolean;
  /** Resets the delete confirmation when the picker switches template. */
  activeId: string;
  onDuplicate: () => void;
  onRename: () => void;
  onDelete: () => void;
}

/**
 * The gear beside the template picker.
 *
 * Delete asks twice in place rather than opening a dialog: the second click is
 * on the same item under the cursor, and a template is cheap to lose and
 * expensive to rebuild.
 */
export function TemplateMenu({
  editable,
  isDuplicating,
  activeId,
  onDuplicate,
  onRename,
  onDelete,
}: Props) {
  const [open, setOpen] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const wrapper = useRef<HTMLDivElement>(null);

  // Close on any click outside, and on Escape. Without this the menu stays
  // open behind the next thing the user does.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!wrapper.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  useEffect(() => {
    setConfirmingDelete(false);
  }, [open, activeId]);

  return (
    <div ref={wrapper} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        aria-label="Template options"
        aria-expanded={open}
        className="flex h-9 w-9 items-center justify-center rounded-md border border-gray-600 bg-gray-700 text-gray-300 transition-colors hover:text-capstone-white"
      >
        <Setting2 size={16} color="currentColor" variant="Linear" />
      </button>

      {open && (
        <div className="absolute left-0 top-10 z-20 w-52 overflow-hidden rounded-lg border border-gray-600 bg-gray-800 py-1 shadow-xl">
          <MenuItem
            icon={<Copy size={14} color="currentColor" variant="Linear" />}
            label="Duplicate"
            disabled={isDuplicating}
            onClick={() => {
              setOpen(false);
              onDuplicate();
            }}
          />
          <MenuItem
            icon={<Edit2 size={14} color="currentColor" variant="Linear" />}
            label="Rename"
            disabled={!editable}
            onClick={() => {
              setOpen(false);
              onRename();
            }}
          />
          <MenuItem
            icon={<Trash size={14} color="currentColor" variant="Linear" />}
            label={confirmingDelete ? 'Delete — confirm' : 'Delete'}
            disabled={!editable}
            onClick={() => {
              if (!confirmingDelete) {
                setConfirmingDelete(true);
                return;
              }
              setOpen(false);
              onDelete();
            }}
          />
        </div>
      )}
    </div>
  );
}

function MenuItem({
  icon,
  label,
  disabled,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      // Disabled steps down the ink ramp instead of fading: an `opacity` here
      // would multiply with any dimmed ancestor, which is the bug that made
      // the template cards' dependent rows unreadable.
      className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors disabled:cursor-not-allowed ${
        disabled ? 'text-gray-500' : 'text-gray-200 hover:bg-gray-700 hover:text-capstone-white'
      }`}
    >
      {icon}
      {label}
    </button>
  );
}
