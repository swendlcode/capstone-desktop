import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { MusicSquare } from '../ui/icons';
import { coverArtQueryKeys, useCoverArt } from '../../hooks/useCoverArt';
import type { CoverArt } from '../../types';

interface CoverCellProps {
  path: string;
}

/**
 * Settle time before a row asks for its artwork.
 *
 * Short enough that it never registers while reading the table, long enough
 * that a flick past a few hundred rows reaches none of their files.
 */
const ARM_DELAY_MS = 120;

/**
 * The track's embedded cover, or a placeholder when the file carries none.
 *
 * Two things keep this cheap on a library-sized queue. The table is
 * virtualized, so only the rows in view mount a cell at all — a 10,000-track
 * drop does not turn into 10,000 file reads. And a cell that mounts only
 * because the user scrolled past it waits out ARM_DELAY_MS before it asks,
 * so a fast scroll costs nothing.
 *
 * A row scrolled back into view skips the wait: react-query already holds the
 * answer, and making it wait again is a placeholder flashing over artwork the
 * app has in hand.
 */
export function CoverCell({ path }: CoverCellProps) {
  const queryClient = useQueryClient();
  const [armed, setArmed] = useState(
    () => queryClient.getQueryData<CoverArt>(coverArtQueryKeys.byPath(path)) !== undefined,
  );

  useEffect(() => {
    if (armed) return;
    const timer = window.setTimeout(() => setArmed(true), ARM_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [armed]);

  const { data } = useCoverArt(armed ? path : null);

  return (
    <div className="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-[5px] border border-gray-700 bg-gray-800">
      {data ? (
        // Decorative: the filename sits in the very next cell.
        <img src={data} alt="" draggable={false} className="h-full w-full object-cover" />
      ) : (
        <MusicSquare size={14} color="currentColor" variant="Linear" className="text-gray-600" />
      )}
    </div>
  );
}
