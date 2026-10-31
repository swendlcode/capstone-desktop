import { Checkbox } from '../ui/Checkbox';
import { Radio } from '../ui/Radio';
import { FieldRow, SettingCard } from './SettingCard';
import { NumberField } from './NumberField';
import type { PitchMode, Template } from '../../types';

interface Props {
  template: Template;
  editable: boolean;
  setField: <K extends keyof Template>(key: K, value: Template[K]) => void;
}

const MODES: Array<{ value: PitchMode; label: string }> = [
  { value: 'keep_tempo', label: 'Fix the pitch, keep the tempo' },
  { value: 'change_tempo', label: 'Change tempo to change pitch (turntable)' },
];

/** Pitch correction. Off by default: most modern files are already on pitch. */
export function PitchCard({ template, editable, setField }: Props) {
  const on = template.correctPitch;
  const dependentsDisabled = !editable || !on;

  return (
    <SettingCard title="Correct Pitch" hint="Good for vinyl rips and older music.">
      <Checkbox
        checked={on}
        disabled={!editable}
        onChange={(e) => setField('correctPitch', e.target.checked)}
        label="Correct pitch"
      />
      <FieldRow
        label="When off key by at least"
        disabled={!on}
        control={
          <NumberField
            label="Pitch tolerance"
            value={template.pitchToleranceCents}
            min={1}
            max={50}
            decimals={0}
            unit="cents"
            disabled={dependentsDisabled}
            onChange={(v) => setField('pitchToleranceCents', v)}
          />
        }
      />
      <div className={`flex flex-col gap-3 ${on ? '' : 'opacity-40'}`}>
        {MODES.map((mode) => (
          <Radio
            key={mode.value}
            name="capstone-pitch-mode"
            value={mode.value}
            checked={template.pitchMode === mode.value}
            disabled={dependentsDisabled}
            onChange={() => setField('pitchMode', mode.value)}
            label={mode.label}
          />
        ))}
      </div>
    </SettingCard>
  );
}
