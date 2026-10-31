import { Pane } from './Pane';
// A generic primitive that currently lives next to its heaviest user; it
// belongs in components/ui alongside Input and Checkbox.
import { SliderField } from '../template/SliderField';
import { useSettingsPatch } from '../../hooks/useSettings';
import type { Settings } from '../../types';

/**
 * How many cores a run may use.
 *
 * The ceiling comes from `navigator.hardwareConcurrency`, because the IPC
 * contract carries no command that reports the core count. A stored value from
 * a machine with more cores would otherwise sit above the slider's maximum, so
 * the ceiling is raised to whatever is stored and the backend's own floor of 1
 * does the rest.
 */
export function ProcessorLoadPane({ settings }: { settings: Settings }) {
  const { patch } = useSettingsPatch();
  const detected = typeof navigator !== 'undefined' ? navigator.hardwareConcurrency : 0;
  const cores = Math.max(detected || 1, settings.maxCores, 1);
  const all = settings.maxCores >= cores;

  return (
    <Pane title="Processor Load">
      <div className="flex max-w-md flex-col gap-2">
        <span className="text-sm text-gray-300">Maximum processor cores</span>
        <SliderField
          label="Maximum processor cores"
          value={Math.min(settings.maxCores, cores)}
          min={1}
          max={cores}
          step={1}
          readout={all ? `All ${cores}` : `${settings.maxCores} / ${cores}`}
          onChange={(v) => patch({ maxCores: Math.round(v) })}
        />
        <p className="text-xs text-gray-400">
          Fewer cores leaves the machine responsive while a batch runs.
        </p>
      </div>
    </Pane>
  );
}
