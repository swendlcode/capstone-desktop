import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { capstoneService } from '../services/capstoneService';
import { settingsQueryKeys, useSettings } from './useSettings';
import type { Template } from '../types';

export const templateQueryKeys = {
  all: ['templates'] as const,
  byId: (id: string) => [...templateQueryKeys.all, 'detail', id] as const,
};

export function useTemplates() {
  return useQuery({
    queryKey: templateQueryKeys.all,
    queryFn: () => capstoneService.getTemplates(),
  });
}

/**
 * The template the Configure tab shows: whichever one `settings.activeTemplateId`
 * points at, falling back to the first in the list so the tab is never blank
 * after a template is deleted.
 */
export function useActiveTemplate(): Template | undefined {
  const { data: settings } = useSettings();
  const { data: templates } = useTemplates();
  if (!templates || templates.length === 0) return undefined;
  return templates.find((t) => t.id === settings?.activeTemplateId) ?? templates[0];
}

export function useTemplateActions() {
  const queryClient = useQueryClient();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: templateQueryKeys.all });

  const save = useMutation({
    mutationFn: (template: Template) => capstoneService.saveTemplate(template),
    // The stored version wins: `save_template` can correct what it was sent,
    // and the cache is reseeded from the reply rather than from the edit.
    onSuccess: (stored) => {
      queryClient.setQueryData<Template[]>(templateQueryKeys.all, (prev) =>
        prev ? prev.map((t) => (t.id === stored.id ? stored : t)) : prev,
      );
      invalidate();
    },
  });

  const duplicate = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      capstoneService.duplicateTemplate(id, name),
    // Which template the Configure tab shows lives in settings, so opening the
    // copy is the caller's job (see useTemplateEditor.duplicateActive).
    onSuccess: invalidate,
  });

  const remove = useMutation({
    mutationFn: (id: string) => capstoneService.deleteTemplate(id),
    onSuccess: () => {
      invalidate();
      // Deleting the active template changes which one Configure shows, and
      // that choice lives in settings.
      queryClient.invalidateQueries({ queryKey: settingsQueryKeys.all });
    },
  });

  return { save, duplicate, remove };
}
