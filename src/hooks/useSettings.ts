import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { capstoneService } from '../services/capstoneService';
import type { Settings } from '../types';

export const settingsQueryKeys = {
  all: ['settings'] as const,
};

export function useSettings() {
  return useQuery({
    queryKey: settingsQueryKeys.all,
    queryFn: () => capstoneService.getSettings(),
  });
}

/**
 * Writes the whole Settings object and seeds the cache with what came back.
 *
 * Personalize has no Save button — every control commits on change — so this
 * takes a complete object rather than a patch. Callers spread the current
 * settings and override one field; `useSettingsPatch` below does that for them.
 */
export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: Settings) => capstoneService.updateSettings(settings),
    onSuccess: (stored) => queryClient.setQueryData(settingsQueryKeys.all, stored),
    onError: () =>
      // Leave the UI showing what is actually stored rather than the rejected
      // edit.
      queryClient.invalidateQueries({ queryKey: settingsQueryKeys.all }),
  });
}

/**
 * Change one or two fields of the settings without the caller having to hold
 * the whole object. A no-op before the first load, which is also when the
 * Personalize controls are still disabled.
 */
export function useSettingsPatch() {
  const { data: settings } = useSettings();
  const update = useUpdateSettings();
  const patch = (changes: Partial<Settings>) => {
    if (!settings) return;
    update.mutate({ ...settings, ...changes });
  };
  return { settings, patch, isPending: update.isPending };
}

/** Opens the folder picker and stores the chosen directory as the output dir. */
export function usePickOutputDir() {
  const { settings, patch } = useSettingsPatch();
  return useMutation({
    mutationFn: () => capstoneService.pickFolder(),
    onSuccess: (dir) => {
      if (!dir || !settings) return;
      patch({ outputDir: dir, outputLocation: 'specific' });
    },
  });
}
