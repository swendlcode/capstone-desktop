import { FieldRow, SettingCard } from './SettingCard';
import { NumberField } from './NumberField';
import { SliderField } from './SliderField';
import type { Template } from '../../types';

interface Props {
  template: Template;
  editable: boolean;
  setField: <K extends keyof Template>(key: K, value: Template[K]) => void;
}

/**
 * The line under the title, read off the number in the field below it.
 *
 * It used to be the fixed string "-12.5 dB keeps the contrast between loud and
 * quiet parts." — true of Festival and of nothing else shipped: Club is -11,
 * Stream -14, Vinyl Rip and Archive -16, and a user template is whatever was
 * typed. A card that states a figure the field beneath it contradicts is worse
 * than a card with no hint at all, so the figure comes from the template.
 *
 * The bands are the four things this number means in practice; the boundaries
 * sit between the shipped values rather than on them, and the copy describes
 * the level rather than naming a venue — a template is free to be at any of
 * these numbers.
 */
export function volumeHint(targetLufs: number): string {
  const shown = `${targetLufs.toFixed(1)} dB`;
  if (targetLufs > -10) return `${shown} is very loud: the limiter does most of the work.`;
  if (targetLufs > -13) return `${shown} is loud, with some contrast left.`;
  if (targetLufs > -15) return `${shown} is the streaming and radio standard.`;
  return `${shown} keeps the contrast between loud and quiet parts.`;
}

/**
 * Target loudness and how hard the limiter works to reach it.
 *
 * The range stops at -6 LUFS: louder than that and the limiter is doing all the
 * work, which is the opposite of what this tool is for.
 */
export function OutputVolumeCard({ template, editable, setField }: Props) {
  return (
    <SettingCard title="Output Volume" hint={volumeHint(template.targetLufs)}>
      <FieldRow
        label="Average volume"
        control={
          <NumberField
            label="Average volume"
            value={template.targetLufs}
            min={-24}
            max={-6}
            decimals={1}
            unit="dB"
            disabled={!editable}
            onChange={(v) => setField('targetLufs', v)}
          />
        }
      />
      <div className="flex flex-col gap-2">
        <span className="text-sm text-gray-300">Limiter character</span>
        <SliderField
          label="Limiter character"
          value={template.limiterCharacter}
          min={0}
          max={10}
          step={1}
          disabled={!editable}
          readout={`${template.limiterCharacter} / 10`}
          onChange={(v) => setField('limiterCharacter', Math.round(v))}
        />
      </div>
    </SettingCard>
  );
}
