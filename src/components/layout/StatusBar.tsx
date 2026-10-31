import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { CapstoneMark, Coffee } from '../ui/icons';
import { isTauri } from '../../lib/tauri-core';

const SUPPORT_URL = 'https://www.buymeacoffee.com/swendl';

/**
 * The signature strip along the bottom of the window: mark, copyright, support.
 *
 * It exists because the mark used to sit in the title bar beside the traffic
 * lights, where it crowded the tabs and read as chrome rather than as a
 * byline. One short muted row is the whole brief — a second toolbar down here
 * would cost the queue table the height it needs.
 */
export function StatusBar() {
  const [version, setVersion] = useState<string | null>(null);

  // Read from the bundle, never written as a literal: a hardcoded string drifts
  // the moment a release ships and would then disagree with the version the
  // updater reports in the title bar. Absent in a plain browser, hence the
  // nullable render below rather than a placeholder.
  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    getVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  // The opener plugin, not an <a href>: a bare anchor navigates the webview
  // itself, replacing the app with a web page and no way back.
  const openSupport = () => {
    openUrl(SUPPORT_URL).catch(() => {});
  };

  return (
    <footer className="flex h-7 shrink-0 items-center justify-between gap-3 border-t border-gray-700 bg-gray-900 px-4 text-[11px] text-gray-500">
      <div className="flex min-w-0 items-center gap-2">
        <CapstoneMark size={11} className="shrink-0 text-capstone-sky/70" />
        <span className="font-semibold tracking-tight text-gray-300">Capstone</span>
        {version && <span className="mono text-[10px]">v{version}</span>}
        <span aria-hidden className="text-gray-600">
          ·
        </span>
        <span className="truncate">© 2026 Swendl</span>
      </div>

      <button
        onClick={openSupport}
        className="flex shrink-0 items-center gap-1.5 text-gray-400 transition-colors hover:text-capstone-sky"
        title={SUPPORT_URL}
      >
        <Coffee size={12} color="currentColor" variant="Bulk" aria-hidden />
        <span>Buy me a coffee</span>
      </button>
    </footer>
  );
}
