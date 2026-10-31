import { useCallback, useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { capstoneService } from '../../services/capstoneService';
import { templateQueryKeys } from '../../hooks/useTemplates';
import { useSettingsPatch } from '../../hooks/useSettings';
import type { ShareNotice } from './TemplateShare';

/** An `invoke` rejection arrives as a plain string from Rust (see error.rs). */
function message(error: unknown): string {
  return typeof error === 'string' ? error : (error as Error)?.message ?? 'That did not work.';
}

/**
 * Export and import for the Configure tab's share row.
 *
 * Both halves are one native dialog each, driven from Rust, so there is nothing
 * to hold here but which of the two is open and the one line that reports what
 * happened. A cancelled picker resolves to null and says nothing: the user
 * closed the dialog, they do not need telling.
 *
 * An import switches the tab to what it just added. Storing a template the user
 * then has to go and find in the picker is how an import looks like it failed.
 */
export function useTemplateShare() {
  const queryClient = useQueryClient();
  const { patch: patchSettings } = useSettingsPatch();
  const [notice, setNotice] = useState<ShareNotice | null>(null);

  const exportTemplate = useMutation({
    mutationFn: (id: string) => capstoneService.exportTemplate(id),
    onSuccess: (path) => setNotice(path ? { kind: 'ok', text: 'Saved.' } : null),
    onError: (error) => setNotice({ kind: 'error', text: message(error) }),
  });

  const importTemplate = useMutation({
    mutationFn: () => capstoneService.importTemplate(),
    onSuccess: (imported) => {
      if (!imported) {
        setNotice(null);
        return;
      }
      queryClient.invalidateQueries({ queryKey: templateQueryKeys.all });
      patchSettings({ activeTemplateId: imported.template.id });
      setNotice({ kind: 'ok', text: `Added ${imported.template.name}.` });
    },
    onError: (error) => setNotice({ kind: 'error', text: message(error) }),
  });

  const busy = exportTemplate.isPending
    ? ('export' as const)
    : importTemplate.isPending
      ? ('import' as const)
      : null;

  return {
    busy,
    notice,
    exportActive: useCallback((id: string) => exportTemplate.mutate(id), [exportTemplate]),
    importFile: useCallback(() => importTemplate.mutate(), [importTemplate]),
  };
}
