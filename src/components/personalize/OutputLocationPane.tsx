import { Button } from '../ui/Button';
import { Checkbox } from '../ui/Checkbox';
import { Radio } from '../ui/Radio';
import { Pane } from './Pane';
import { usePickOutputDir, useSettingsPatch } from '../../hooks/useSettings';
import type { Settings } from '../../types';

/**
 * Where processed files land.
 *
 * `outputDir` can be null while the location is `specific`: the backend falls
 * back to a Capstone folder under the user's audio directory, so a first run
 * writes something without being asked. The pane says so rather than showing an
 * empty path and implying the setting is broken.
 */
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

      <div className={`ml-6 flex flex-col gap-3 ${specific ? '' : 'opacity-40'}`}>
        <div className="flex items-center gap-3">
          <span className="mono min-w-0 flex-1 truncate rounded-md border border-gray-600 bg-gray-700 px-3 py-2 text-xs text-gray-300">
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
    </Pane>
  );
}
