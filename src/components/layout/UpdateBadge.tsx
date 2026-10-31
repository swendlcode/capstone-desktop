import { useState } from 'react';
import { ArrowUp2, CloseCircle, Refresh, TickCircle } from '../ui/icons';
import { useAppUpdate } from '../../hooks/useAppUpdate';

/**
 * In-app update control, in the title bar.
 *
 * Downloading happens in the background and the restart is always the user's
 * call: Capstone's whole job is long batch runs, and relaunching under one
 * would lose the queue.
 */
export function UpdateBadge() {
  const { stage, version, progress, error, install, restart } = useAppUpdate();
  const [dismissed, setDismissed] = useState(false);

  if (stage === 'idle' || dismissed) return null;

  const shell =
    'flex items-center gap-1 rounded-md border border-capstone-sky/30 bg-capstone-sky/10 py-1 pl-2 pr-1';

  if (stage === 'downloading') {
    const pct = progress === null ? null : Math.round(progress * 100);
    return (
      <div className={shell} title={`Downloading Capstone v${version}`}>
        <Refresh size={11} color="currentColor" variant="Linear" className="animate-spin" />
        <span className="mono text-xs font-medium text-capstone-sky">
          {pct === null ? 'Updating…' : `${pct}%`}
        </span>
      </div>
    );
  }

  if (stage === 'ready') {
    return (
      <div className={shell}>
        <button
          onClick={restart}
          className="flex items-center gap-1 text-xs font-medium text-capstone-sky transition-colors hover:text-capstone-sky/80"
          title={`Capstone v${version} is installed — restart to finish`}
        >
          <TickCircle size={11} color="currentColor" variant="Bold" />
          Restart to update
        </button>
      </div>
    );
  }

  if (stage === 'error') {
    return (
      <div className="flex items-center gap-1 rounded-md border border-capstone-red/40 bg-capstone-red/10 py-1 pl-2 pr-1">
        <span className="text-xs font-medium text-capstone-red" title={error ?? undefined}>
          Update failed
        </span>
        <button
          onClick={() => setDismissed(true)}
          className="ml-0.5 text-capstone-red/60 transition-colors hover:text-capstone-red"
          aria-label="Dismiss"
        >
          <CloseCircle size={13} color="currentColor" variant="Linear" />
        </button>
      </div>
    );
  }

  return (
    <div className={shell}>
      <button
        onClick={install}
        className="flex items-center gap-1 text-xs font-medium text-capstone-sky transition-colors hover:text-capstone-sky/80"
        title={`Capstone v${version} is available — click to update`}
      >
        <ArrowUp2 size={11} color="currentColor" variant="Bold" />
        Update to v{version}
      </button>
      <button
        onClick={() => setDismissed(true)}
        className="ml-0.5 text-capstone-sky/60 transition-colors hover:text-capstone-sky"
        aria-label="Dismiss update"
      >
        <CloseCircle size={13} color="currentColor" variant="Linear" />
      </button>
    </div>
  );
}
