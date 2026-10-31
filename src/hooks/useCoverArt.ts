import { useQuery } from '@tanstack/react-query';
import { capstoneService } from '../services/capstoneService';
import type { CoverArt } from '../types';

export const coverArtQueryKeys = {
  all: ['cover-art'] as const,
  byPath: (path: string) => [...coverArtQueryKeys.all, path] as const,
};

/**
 * The embedded artwork for one file.
 *
 * Keyed on the path rather than the queue id, matching the backend's own cache:
 * the same file queued twice then hits one entry on both sides.
 *
 * Artwork is baked into the file, so it cannot change while the app is open
 * without the file being replaced underneath us. That makes it the rare thing
 * that genuinely never goes stale, hence `staleTime: Infinity` and no refetch
 * on focus or remount — scrolling the queue would otherwise refire the query
 * for every row that leaves and re-enters the viewport.
 */
export function useCoverArt(path: string | null | undefined) {
  return useQuery<CoverArt>({
    queryKey: coverArtQueryKeys.byPath(path ?? ''),
    queryFn: () => capstoneService.getCoverArt(path as string),
    enabled: Boolean(path),
    staleTime: Infinity,
    gcTime: Infinity,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    // A file with no art answers null, which is the answer, not a failure.
    retry: false,
  });
}
