import { NumberField } from './NumberField';
import type { BandSetting } from '../../types';

/** Shared by the header and every row, so the columns line up. */
export const BAND_GRID =
  'grid items-center gap-3 grid-cols-[56px_minmax(190px,1.4fr)_repeat(5,minmax(78px,1fr))]';

const BAND_NAMES = ['Low', 'Low-mid', 'High-mid', 'High'];

interface Props {
  band: BandSetting;
  index: number;
  /** The top of the band above this one's ceiling; the crossover stays below it. */
  maxCrossoverHz: number;
  /** The last band's ceiling is the top of the spectrum and does not move. */
  isLast: boolean;
  editable: boolean;
  onChange: (patch: Partial<BandSetting>) => void;
  onCrossover: (hz: number) => void;
}

/**
 * One band: where it sits in the spectrum, and the five numbers that shape it.
 *
 * Only the upper edge is editable. The bands meet exactly — this band's
 * `highHz` is the next one's `lowHz` — so a band's lower edge is the crossover
 * the row above already owns, and showing it twice invites two fields to
 * disagree.
 */
export function BandRow({
  band,
  index,
  maxCrossoverHz,
  isLast,
  editable,
  onChange,
  onCrossover,
}: Props) {
  const name = BAND_NAMES[index] ?? `Band ${index + 1}`;

  return (
    <div className={`${BAND_GRID} border-t border-gray-700 py-3`}>
      <span className="text-xs font-semibold uppercase tracking-wide text-gray-300">
        {name}
      </span>

      <div className="flex min-w-0 items-center gap-2">
        <span className="mono shrink-0 text-xs text-gray-400">
          {Math.round(band.lowHz)}
        </span>
        <span className="shrink-0 text-gray-500">–</span>
        {isLast ? (
          <span className="mono text-xs text-gray-400">{Math.round(band.highHz)} Hz</span>
        ) : (
          <NumberField
            label={`${name} crossover`}
            value={band.highHz}
            min={band.lowHz + 20}
            max={maxCrossoverHz}
            decimals={0}
            step={10}
            unit="Hz"
            disabled={!editable}
            onChange={onCrossover}
          />
        )}
      </div>

      <NumberField
        label={`${name} threshold`}
        value={band.thresholdDb}
        min={-60}
        max={0}
        decimals={1}
        unit="dB"
        disabled={!editable}
        onChange={(v) => onChange({ thresholdDb: v })}
      />
      <NumberField
        label={`${name} ratio`}
        value={band.ratio}
        min={1}
        max={20}
        decimals={1}
        unit=":1"
        disabled={!editable}
        onChange={(v) => onChange({ ratio: v })}
      />
      <NumberField
        label={`${name} attack`}
        value={band.attackMs}
        min={0.1}
        max={200}
        decimals={1}
        unit="ms"
        disabled={!editable}
        onChange={(v) => onChange({ attackMs: v })}
      />
      <NumberField
        label={`${name} release`}
        value={band.releaseMs}
        min={10}
        max={2000}
        decimals={0}
        step={5}
        unit="ms"
        disabled={!editable}
        onChange={(v) => onChange({ releaseMs: v })}
      />
      <NumberField
        label={`${name} makeup`}
        value={band.makeupDb}
        min={-12}
        max={12}
        decimals={1}
        unit="dB"
        disabled={!editable}
        onChange={(v) => onChange({ makeupDb: v })}
      />
    </div>
  );
}
