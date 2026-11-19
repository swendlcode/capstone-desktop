import { useEffect, useState } from 'react';
import { Pane } from './Pane';
import {
  currentZoom,
  DEFAULT_ZOOM,
  readZoom,
  ZOOM_EVENT,
  zoomIn,
  zoomOut,
  zoomReset,
} from '../../lib/appZoom';

/**
 * Interface scale — the only pane here that is not backend state.
 *
 * It listens for the zoom event rather than owning the value, so the readout
 * stays right when the keyboard is used while this pane is open.
 */
export function InterfaceScalePane() {
  const [zoom, setZoom] = useState(() => currentZoom() || readZoom());

  useEffect(() => {
    const onChange = (e: Event) => setZoom((e as CustomEvent<{ zoom: number }>).detail.zoom);
    window.addEventListener(ZOOM_EVENT, onChange);
    return () => window.removeEventListener(ZOOM_EVENT, onChange);
  }, []);

  const btn =
    'flex h-8 w-8 items-center justify-center rounded-md border border-gray-700 text-gray-300 ' +
    'transition-colors hover:border-capstone-sky/60 hover:text-capstone-white ' +
    'disabled:opacity-40 disabled:hover:border-gray-700 disabled:hover:text-gray-300';

  return (
    <Pane title="Interface Scale">
      <div className="flex max-w-md flex-col gap-3">
        <span className="text-sm text-gray-300">Size of everything in the window</span>
        <div className="flex items-center gap-2">
          <button className={btn} onClick={() => setZoom(zoomOut())} aria-label="Smaller">
            <span className="text-base leading-none">−</span>
          </button>
          <span className="mono w-16 text-center text-sm text-gray-300">
            {Math.round(zoom * 100)}%
          </span>
          <button className={btn} onClick={() => setZoom(zoomIn())} aria-label="Bigger">
            <span className="text-base leading-none">+</span>
          </button>
          {/* The percentage is deliberately not the reset control: nothing
              about a number says it can be clicked. */}
          <button
            className="ml-1 flex h-8 items-center rounded-md border border-gray-700 px-2.5
              text-[10px] uppercase tracking-widest text-gray-400 transition-colors
              hover:border-capstone-sky/60 hover:text-capstone-white disabled:opacity-40
              disabled:hover:border-gray-700 disabled:hover:text-gray-400"
            onClick={() => setZoom(zoomReset())}
            disabled={zoom === DEFAULT_ZOOM}
            title="Back to 100%"
          >
            Reset
          </button>
        </div>
        <p className="text-xs text-gray-400">
          Scales the whole interface, not just the text. Cmd / Ctrl with + and − do the same from
          anywhere in the app, and Cmd / Ctrl 0 goes back to 100%. The size is remembered for this
          display only.
        </p>
      </div>
    </Pane>
  );
}
