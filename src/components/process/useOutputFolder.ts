import { useMutation } from '@tanstack/react-query';
import { capstoneService } from '../../services/capstoneService';

/**
 * Reveals the configured output folder in Finder / Explorer.
 *
 * A mutation rather than a bare call so the footer button can disable itself
 * while the shell is being asked, and so a failure (an output folder the user
 * has since deleted) is a rejected mutation rather than an unhandled rejection.
 */
export function useOpenOutputFolder() {
  return useMutation({
    mutationFn: () => capstoneService.openOutputFolder(),
  });
}
