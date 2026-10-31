import { useEffect, useRef, useState } from 'react';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Copy, Edit2, Setting2, Trash } from '../ui/icons';
import type { Template } from '../../types';

interface Props {
  templates: Template[];
  active: Template;
  editable: boolean;
  isDuplicating: boolean;
  onSelect: (id: string) => void;
  onDuplicate: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
}

/**
 * The named template at the top of the tab, with its gear menu.
 *
 * Stock templates are read-only, so the menu offers Duplicate and nothing else
 * and the hint says why. Duplicating is also surfaced as a button next to the
 * picker: it is the one action a user on a stock template actually needs, and
 * burying it behind a gear is how the original hides it.
 */
export function TemplatePicker({
  templates,
  active,
  editable,
  isDuplicating,
  onSelect,
  onDuplicate,
  onRename,
  onDelete,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [draftName, setDraftName] = useState(active.name);
  const wrapper = useRef<HTMLDivElement>(null);

  // Close the menu on any click outside it, and on Escape. Without this the
  // menu stays open behind the next thing the user does.
  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!wrapper.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenuOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [menuOpen]);

  useEffect(() => {
    setConfirmingDelete(false);
  }, [menuOpen, active.id]);

  const startRename = () => {
    setDraftName(active.name);
    setRenaming(true);
    setMenuOpen(false);
  };

  const commitRename = () => {
    setRenaming(false);
    onRename(draftName);
  };

  return (
    <div className="flex flex-wrap items-center gap-3">
      {renaming ? (
        <Input
          className="w-56"
          autoFocus
          value={draftName}
          aria-label="Template name"
          onChange={(e) => setDraftName(e.target.value)}
          onBlur={commitRename}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commitRename();
            if (e.key === 'Escape') setRenaming(false);
          }}
        />
      ) : (
        <select
          value={active.id}
          aria-label="Template"
          onChange={(e) => onSelect(e.target.value)}
          className="h-9 w-56 rounded-md border border-gray-600 bg-gray-700 px-3 text-sm font-semibold text-capstone-white outline-none focus:border-capstone-sky"
        >
          {templates.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      )}

      <div ref={wrapper} className="relative">
        <button
          onClick={() => setMenuOpen((v) => !v)}
          aria-label="Template options"
          aria-expanded={menuOpen}
          className="flex h-9 w-9 items-center justify-center rounded-md border border-gray-600 bg-gray-700 text-gray-300 transition-colors hover:text-capstone-white"
        >
          <Setting2 size={16} color="currentColor" variant="Linear" />
        </button>

        {menuOpen && (
          <div className="absolute left-0 top-10 z-20 w-52 overflow-hidden rounded-lg border border-gray-600 bg-gray-800 py-1 shadow-xl">
            <MenuItem
              icon={<Copy size={14} color="currentColor" variant="Linear" />}
              label="Duplicate"
              disabled={isDuplicating}
              onClick={() => {
                setMenuOpen(false);
                onDuplicate();
              }}
            />
            <MenuItem
              icon={<Edit2 size={14} color="currentColor" variant="Linear" />}
              label="Rename"
              disabled={!editable}
              onClick={startRename}
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
                setMenuOpen(false);
                onDelete();
              }}
            />
          </div>
        )}
      </div>

      {!editable && (
        <>
          <Button variant="primary" size="sm" onClick={onDuplicate} disabled={isDuplicating}>
            Duplicate
          </Button>
          <span className="text-xs text-gray-400">Duplicate to change these settings.</span>
        </>
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
  icon: React.ReactNode;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-gray-200 transition-colors hover:bg-gray-700 hover:text-capstone-white disabled:cursor-not-allowed disabled:opacity-40"
    >
      {icon}
      {label}
    </button>
  );
}
