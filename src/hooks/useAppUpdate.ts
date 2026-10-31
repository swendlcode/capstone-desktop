import { useCallback, useEffect, useState } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { isTauri } from '../lib/tauri-core';

export type UpdateStage = 'idle' | 'available' | 'downloading' | 'ready' | 'error';

interface UpdateState {
  stage: UpdateStage;
  version: string | null;
  /** 0–1 while downloading, null when the server did not send a length. */
  progress: number | null;
  error: string | null;
}

const INITIAL: UpdateState = { stage: 'idle', version: null, progress: null, error: null };

/**
 * In-app updates. Checks the signed manifest on the latest release, downloads
 * in the background while the app stays usable, and relaunches only when the
 * user asks — a batch that is mid-run must never be interrupted by an update.
 *
 * The check runs once per launch. That is also the only telemetry this app has:
 * the manifest's download count is a count of launches, not of installs, and
 * should never be read as a user count.
 */
export function useAppUpdate() {
  const [state, setState] = useState<UpdateState>(INITIAL);
  const [update, setUpdate] = useState<Update | null>(null);

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    check()
      .then((found) => {
        if (cancelled || !found) return;
        setUpdate(found);
        setState({ stage: 'available', version: found.version, progress: null, error: null });
      })
      // No network, no release yet, or an unsigned local build — stay quiet.
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const install = useCallback(async () => {
    if (!update) return;
    setState((s) => ({ ...s, stage: 'downloading', progress: 0, error: null }));
    let downloaded = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          setState((s) => ({
            ...s,
            stage: 'downloading',
            progress: total > 0 ? Math.min(1, downloaded / total) : null,
          }));
        } else if (event.event === 'Finished') {
          setState((s) => ({ ...s, stage: 'ready', progress: 1 }));
        }
      });
      setState((s) => ({ ...s, stage: 'ready', progress: 1 }));
    } catch (e) {
      setState((s) => ({ ...s, stage: 'error', error: String(e) }));
    }
  }, [update]);

  const restart = useCallback(async () => {
    try {
      await relaunch();
    } catch (e) {
      setState((s) => ({ ...s, stage: 'error', error: String(e) }));
    }
  }, []);

  return { ...state, install, restart };
}
