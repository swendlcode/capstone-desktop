import { Button } from '../ui/Button';
import { Checkbox } from '../ui/Checkbox';
import { Radio } from '../ui/Radio';
import { Pane } from './Pane';
import { usePickOutputDir, useSettingsPatch } from '../../hooks/useSettings';
import type { CollisionPolicy, Settings } from '../../types';

/**
 * Where processed files land.
 *
 * `outputDir` can be null while the location is `specific`: the backend falls
 * back to a Capstone folder under the user's audio directory, so a first run
 * writes something without being asked. The pane says so rather than showing an
 * empty path and implying the setting is broken.
 *
 * The collision choice lives here rather than in a section of its own: it only
 * means anything together with the folder above it, and it is the second half
 * of the same question — where the file goes, and what happens if something is
 * already there.
 */
const COLLISION_CHOICES: { value: CollisionPolicy; label: string }[] = [
  { value: 'overwrite', label: 'Replace it' },
  { value: 'skip', label: 'Leave it and skip the file' },
  { value: 'rename', label: 'Keep both \u2014 write \u201cname (2)\u201d' },
];

export function OutputLocationPane({ settings }: { settings: Settings }) {
  const { patch } = useSettingsPatch();
  const pickDir = usePickOutputDir();
  const specific = settings.outputLocation === 'specific';

  return (
    <Pane title="Output Location">
      <Radio
        name="capstone-output-location"
        checked={specific}
        onChange={() => patch({ outputLocation: 'specific' })}
        label="Specific folder"
      />

      {/* Dimmed by token, not by `opacity`: the Button and Checkbox below dim
          themselves when disabled, and the two would multiply. */}
      <div className="ml-6 flex flex-col gap-3">
        <div className="flex items-center gap-3">
          <span
            className={`mono min-w-0 flex-1 truncate rounded-md border border-gray-600 bg-gray-700 px-3 py-2 text-xs ${
              specific ? 'text-gray-300' : 'text-gray-500'
            }`}
          >
            {settings.outputDir ?? 'Default Capstone folder'}
          </span>
          <Button
            size="sm"
            disabled={!specific || pickDir.isPending}
            onClick={() => pickDir.mutate()}
          >
            Change…
          </Button>
        </div>
        <Checkbox
          checked={settings.mirrorInputStructure}
          disabled={!specific}
          onChange={(e) => patch({ mirrorInputStructure: e.target.checked })}
          label="Mirror structure of input folders"
        />
      </div>

      <Radio
        name="capstone-output-location"
        checked={!specific}
        onChange={() => patch({ outputLocation: 'same_as_source' })}
        label="Same folder as the original file"
      />

      <div className="mt-1 border-t border-gray-700/70 pt-4">
        <p className="text-[10px] uppercase tracking-widest text-gray-500">
          If a file is already there
        </p>
        <div className="mt-3 flex flex-col gap-2">
          {COLLISION_CHOICES.map(({ value, label }) => (
            <Radio
              key={value}
              name="capstone-collision"
              checked={settings.collisionPolicy === value}
              onChange={() => patch({ collisionPolicy: value })}
              label={label}
            />
          ))}
        </div>
        {/* Said once, here, because it is the one part of this that is not a
            choice: two files in the same run that resolve to one name are
            always kept apart, since either alternative loses a track. */}
        <p className="mt-3 text-xs leading-relaxed text-gray-500">
          Two files in one run that would land on the same name are always kept
          apart, whichever of these is set.
        </p>
      </div>
    </Pane>
  );
}
