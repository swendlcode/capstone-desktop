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
 * Target loudness and how hard the limiter works to reach it.
 *
 * The range stops at -6 LUFS: louder than that and the limiter is doing all the
 * work, which is the opposite of what this tool is for.
 */
export function OutputVolumeCard({ template, editable, setField }: Props) {
  return (
    <SettingCard
      title="Output Volume"
      hint="-12.5 dB keeps the contrast between loud and quiet parts."
    >
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
