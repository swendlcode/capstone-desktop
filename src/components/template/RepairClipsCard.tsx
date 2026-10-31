import { Checkbox } from '../ui/Checkbox';
import { FieldRow, SettingCard } from './SettingCard';
import { NumberField } from './NumberField';
import type { Template } from '../../types';

interface Props {
  template: Template;
  editable: boolean;
  setField: <K extends keyof Template>(key: K, value: Template[K]) => void;
}

/**
 * Clip repair, and the three numbers that decide when it runs.
 *
 * The three fields disable with the checkbox rather than disappearing: a
 * threshold the user set is still worth seeing while the feature is off, and a
 * row that vanishes takes the layout with it.
 */
export function RepairClipsCard({ template, editable, setField }: Props) {
  const on = template.repairClips;
  const fieldsDisabled = !editable || !on;

  return (
    <SettingCard title="Repair Clipped Peaks">
      <Checkbox
        checked={on}
        disabled={!editable}
        onChange={(e) => setField('repairClips', e.target.checked)}
        label="Repair clipped peaks"
      />
      <FieldRow
        label="Only when louder than"
        disabled={!on}
        control={
          <NumberField
            label="Repair above"
            value={template.repairAboveLufs}
            min={-30}
            max={0}
            decimals={1}
            unit="dB"
            disabled={fieldsDisabled}
            onChange={(v) => setField('repairAboveLufs', v)}
          />
        }
      />
      <FieldRow
        label="Clip threshold"
        disabled={!on}
        control={
          <NumberField
            label="Clip threshold"
            value={template.clipThresholdPct}
            min={90}
            max={100}
            decimals={1}
            unit="%"
            disabled={fieldsDisabled}
            onChange={(v) => setField('clipThresholdPct', v)}
          />
        }
      />
      <FieldRow
        label="Headroom"
        disabled={!on}
        control={
          <NumberField
            label="Headroom"
            value={template.headroomDb}
            min={0}
            max={6}
            decimals={1}
            unit="dB"
            disabled={fieldsDisabled}
            onChange={(v) => setField('headroomDb', v)}
          />
        }
      />
    </SettingCard>
  );
}
