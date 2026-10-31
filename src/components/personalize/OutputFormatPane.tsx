import { Radio } from '../ui/Radio';
import { Pane } from './Pane';
import { useSettingsPatch } from '../../hooks/useSettings';
import type { Mp3Rate, OutputFormat, Settings } from '../../types';

const FORMATS: Array<{ value: OutputFormat; label: string; hint: string }> = [
  { value: 'aiff', label: 'AIFF (16 bit / 44 kHz)', hint: 'Uncompressed. Largest files, opens everywhere.' },
  { value: 'alac', label: 'Apple Lossless', hint: 'Lossless, about half the size. Mac and iOS.' },
  { value: 'flac', label: 'FLAC', hint: 'Lossless, about half the size. Open standard.' },
  { value: 'mp3', label: 'MP3', hint: 'Lossy, smallest files. Plays on anything.' },
  { value: 'wav', label: 'WAV (16 bit / 44 kHz)', hint: 'Uncompressed. Largest files, the Windows standard.' },
];

const RATES: Array<{ value: Mp3Rate; label: string }> = [
  { value: 'cbr320', label: 'CBR 320 kbps' },
  { value: 'vbr', label: 'VBR' },
];

/** The five output formats, with MP3's encoding picker nested under it. */
export function OutputFormatPane({ settings }: { settings: Settings }) {
  const { patch } = useSettingsPatch();

  return (
    <Pane title="Output File Format">
      {FORMATS.map((format) => (
        <div key={format.value} className="flex flex-col gap-3">
          <Radio
            name="capstone-output-format"
            checked={settings.format === format.value}
            onChange={() => patch({ format: format.value })}
            label={format.label}
            hint={format.hint}
          />
          {format.value === 'mp3' && settings.format === 'mp3' && (
            <div className="ml-6 flex items-center gap-5">
              {RATES.map((rate) => (
                <Radio
                  key={rate.value}
                  name="capstone-mp3-rate"
                  checked={settings.mp3Rate === rate.value}
                  onChange={() => patch({ mp3Rate: rate.value })}
                  label={<span className="mono text-xs">{rate.label}</span>}
                />
              ))}
            </div>
          )}
        </div>
      ))}
    </Pane>
  );
}
