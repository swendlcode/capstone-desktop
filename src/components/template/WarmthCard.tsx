import { Radio } from '../ui/Radio';
import { SettingCard } from './SettingCard';
import type { Template, Warmth } from '../../types';

interface Props {
  template: Template;
  editable: boolean;
  setField: <K extends keyof Template>(key: K, value: Template[K]) => void;
}

const OPTIONS: Array<{ value: Warmth; label: string }> = [
  { value: 'none', label: 'No warmth, just regular sound' },
  { value: 'gentle', label: 'Gentle warmth' },
  { value: 'tube', label: 'Hot vacuum tube' },
];

export function WarmthCard({ template, editable, setField }: Props) {
  return (
    <SettingCard title="Add Warmth">
      <div className="flex flex-col gap-3">
        {OPTIONS.map((option) => (
          <Radio
            key={option.value}
            name="capstone-warmth"
            value={option.value}
            checked={template.warmth === option.value}
            disabled={!editable}
            onChange={() => setField('warmth', option.value)}
            label={option.label}
          />
        ))}
      </div>
    </SettingCard>
  );
}
