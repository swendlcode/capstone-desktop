import { useEffect } from 'react';
import { Button } from '../ui/Button';
import { CloseCircle } from '../ui/icons';
import { BAND_GRID, BandRow } from './BandRow';
import type { BandSetting, Template } from '../../types';

interface Props {
  template: Template;
  editable: boolean;
  onClose: () => void;
  onBandChange: (index: number, patch: Partial<BandSetting>) => void;
  onCrossover: (index: number, hz: number) => void;
}

const HEADINGS = ['Band', 'Range', 'Threshold', 'Ratio', 'Attack', 'Release', 'Makeup'];

/**
 * The multiband compressor, as a modal over the Configure tab.
 *
 * Modal rather than a fifth card because it is a table of 28 numbers and would
 * dominate a tab whose job is four decisions. Closing it does not need a Save:
 * every field in here writes through the same way the cards do.
 */
export function MultibandEditor({
  template,
  editable,
  onClose,
  onBandChange,
  onCrossover,
}: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-30 flex items-center justify-center bg-black/70 p-6"
      role="dialog"
      aria-modal="true"
      aria-label="Multiband dynamics"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="flex max-h-full w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-gray-700 bg-gray-800 shadow-2xl">
        <header className="flex shrink-0 items-center justify-between gap-3 border-b border-gray-700 px-5 py-4">
          <div>
            <h2 className="text-sm font-bold uppercase tracking-wider text-capstone-sky">
              Multiband Dynamics
            </h2>
            <p className="mt-1 text-xs text-gray-400">
              {editable
                ? `${template.bands.length} bands · ${template.name}`
                : `${template.name} is a stock template — duplicate it to edit.`}
            </p>
          </div>
          <button
            onClick={onClose}
            aria-label="Close multiband dynamics"
            className="flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition-colors hover:bg-gray-700 hover:text-capstone-white"
          >
            <CloseCircle size={18} color="currentColor" variant="Linear" />
          </button>
        </header>

        <div className="min-h-0 flex-1 overflow-auto px-5 pb-2">
          <div className="min-w-[780px]">
            <div className={`${BAND_GRID} pb-2 pt-4`}>
              {HEADINGS.map((h) => (
                <span
                  key={h}
                  className="text-[10px] font-semibold uppercase tracking-wider text-gray-400"
                >
                  {h}
                </span>
              ))}
            </div>
            {template.bands.map((band, i) => (
              <BandRow
                key={i}
                band={band}
                index={i}
                isLast={i === template.bands.length - 1}
                maxCrossoverHz={template.bands[i + 1]?.highHz ?? band.highHz}
                editable={editable}
                onChange={(patch) => onBandChange(i, patch)}
                onCrossover={(hz) => onCrossover(i, hz)}
              />
            ))}
          </div>
        </div>

        <footer className="flex shrink-0 justify-end border-t border-gray-700 px-5 py-3">
          <Button variant="primary" size="sm" onClick={onClose}>
            Done
          </Button>
        </footer>
      </div>
    </div>
  );
}
