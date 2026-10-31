import { useCallback, useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  templateQueryKeys,
  useActiveTemplate,
  useTemplateActions,
} from '../../hooks/useTemplates';
import { useSettingsPatch } from '../../hooks/useSettings';
import type { BandSetting, Template } from '../../types';

/** Debounce before a change reaches `save_template`. */
const WRITE_DELAY_MS = 300;

/**
 * Write-through editing for the active template.
 *
 * The Configure tab has no Save button: every control commits. Two things make
 * that behave. The cached template is patched immediately, so a slider or a
 * number field reads back the value the user just set rather than waiting for a
 * round trip. The write itself is debounced, because dragging a slider is one
 * gesture and not forty saves of the same file.
 *
 * The debounce is flushed on unmount, which is what "nothing lost on tab
 * switch" means in practice: the tab unmounts the moment it is left, and a
 * pending timer would otherwise be cleared with the last edit still in it.
 *
 * Stock templates are read-only, so every setter is a no-op for them. The
 * backend refuses them too (`save_template` errors on `isStock`), and this
 * keeps the UI from asking.
 */
export function useTemplateEditor() {
  const template = useActiveTemplate();
  const { save, duplicate, remove } = useTemplateActions();
  const { settings, patch: patchSettings } = useSettingsPatch();
  const queryClient = useQueryClient();

  const pending = useRef<Template | null>(null);
  const timer = useRef<number | null>(null);
  const saveRef = useRef(save);
  saveRef.current = save;

  const flush = useCallback(() => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    const next = pending.current;
    pending.current = null;
    if (next) saveRef.current.mutate(next);
  }, []);

  // Through a ref so the cleanup does not re-run (and flush early) whenever a
  // new `flush` identity comes along.
  const flushRef = useRef(flush);
  flushRef.current = flush;
  useEffect(() => () => flushRef.current(), []);

  const commit = useCallback(
    (next: Template) => {
      queryClient.setQueryData<Template[]>(templateQueryKeys.all, (prev) =>
        prev ? prev.map((t) => (t.id === next.id ? next : t)) : prev,
      );
      pending.current = next;
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => flush(), WRITE_DELAY_MS);
    },
    [flush, queryClient],
  );

  const editable = Boolean(template && !template.isStock);

  const setField = useCallback(
    <K extends keyof Template>(key: K, value: Template[K]) => {
      if (!template || template.isStock) return;
      commit({ ...template, [key]: value });
    },
    [commit, template],
  );

  const setBand = useCallback(
    (index: number, patch: Partial<BandSetting>) => {
      if (!template || template.isStock) return;
      if (index < 0 || index >= template.bands.length) return;
      const bands = template.bands.map((b, i) => (i === index ? { ...b, ...patch } : b));
      commit({ ...template, bands });
    },
    [commit, template],
  );

  /**
   * Moves one crossover. The bands meet exactly — band i's `highHz` is band
   * i+1's `lowHz` — and the multiband processor relies on that, so both sides
   * move together and the value is kept clear of its neighbours.
   */
  const setCrossover = useCallback(
    (index: number, hz: number) => {
      if (!template || template.isStock) return;
      const bands = template.bands;
      if (index < 0 || index >= bands.length - 1) return;
      const floor = bands[index].lowHz + 20;
      const ceiling = bands[index + 1].highHz - 20;
      const clamped = Math.min(Math.max(hz, floor), ceiling);
      const next = bands.map((b, i) => {
        if (i === index) return { ...b, highHz: clamped };
        if (i === index + 1) return { ...b, lowHz: clamped };
        return b;
      });
      commit({ ...template, bands: next });
    },
    [commit, template],
  );

  /** Switches which template the tab shows; the choice lives in settings. */
  const selectTemplate = useCallback(
    (id: string) => {
      flush();
      if (settings?.activeTemplateId === id) return;
      patchSettings({ activeTemplateId: id });
    },
    [flush, patchSettings, settings],
  );

  /** Duplicating is the only way to edit a stock template's values. */
  const duplicateActive = useCallback(() => {
    if (!template) return;
    flush();
    duplicate.mutate(
      { id: template.id, name: `${template.name} Copy` },
      { onSuccess: (copy) => patchSettings({ activeTemplateId: copy.id }) },
    );
  }, [duplicate, flush, patchSettings, template]);

  const rename = useCallback(
    (name: string) => {
      const trimmed = name.trim();
      if (!template || template.isStock || trimmed.length === 0) return;
      if (trimmed === template.name) return;
      setField('name', trimmed);
      flush();
    },
    [flush, setField, template],
  );

  const deleteActive = useCallback(() => {
    if (!template || template.isStock) return;
    // Drop any queued write first: saving a template that is about to be
    // deleted re-adds it.
    pending.current = null;
    if (timer.current !== null) window.clearTimeout(timer.current);
    remove.mutate(template.id);
  }, [remove, template]);

  return {
    template,
    editable,
    setField,
    setBand,
    setCrossover,
    selectTemplate,
    duplicateActive,
    rename,
    deleteActive,
    isDuplicating: duplicate.isPending,
  };
}
