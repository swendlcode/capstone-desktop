import { useState } from 'react';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { TemplateMenu } from './TemplateMenu';
import { templateDescription } from './descriptions';
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
 * The list is grouped: shipped templates in one group, the user's own in
 * another. With one template a flat list was fine; with five shipped and
 * however many the user has made or imported, a flat list is a wall of names
 * with no way to tell what is read-only until you click it.
 *
 * Under the row is the line that says what the selected template is *for*. It
 * is there rather than in a tooltip because a native select cannot show
 * anything on hover, and the point of a template is choosing the right one
 * before processing fifty files with the wrong one.
 *
 * Stock templates are read-only, so the menu offers Duplicate and nothing else.
 * Duplicating is also a button next to the picker: it is the one action a user
 * on a stock template actually needs, and burying it behind a gear hides it.
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
  const [renaming, setRenaming] = useState(false);
  const [draftName, setDraftName] = useState(active.name);

  const stock = templates.filter((t) => t.isStock);
  const mine = templates.filter((t) => !t.isStock);
  const description = templateDescription(active);

  const startRename = () => {
    setDraftName(active.name);
    setRenaming(true);
  };

  const commitRename = () => {
    setRenaming(false);
    onRename(draftName);
  };

  return (
    <div className="flex flex-col gap-2">
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
            <optgroup label="Stock">
              {stock.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </optgroup>
            {mine.length > 0 && (
              <optgroup label="Yours">
                {mine.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        )}

        <TemplateMenu
          editable={editable}
          isDuplicating={isDuplicating}
          activeId={active.id}
          onDuplicate={onDuplicate}
          onRename={startRename}
          onDelete={onDelete}
        />

        {!editable && (
          <Button variant="primary" size="sm" onClick={onDuplicate} disabled={isDuplicating}>
            Duplicate
          </Button>
        )}
      </div>

      {/* Always rendered, so switching template does not shift the cards below. */}
      <p className="min-h-[1rem] text-xs text-gray-400">
        <span className="font-semibold uppercase tracking-wide text-capstone-sky">
          {active.isStock ? 'Stock' : 'Yours'}
        </span>
        {description && <span> · {description}</span>}
        {!editable && <span> · Duplicate to edit.</span>}
      </p>
    </div>
  );
}
