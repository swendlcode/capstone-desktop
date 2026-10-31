interface SliderFieldProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  disabled?: boolean;
  label: string;
  /** What the mono readout to the right of the track shows. */
  readout: string;
  onChange: (value: number) => void;
}

/**
 * Range input plus a mono readout.
 *
 * The filled portion of the track is driven by `--slider-pct` (see the
 * `.capstone-slider` rules in index.css), which has to be set here because the
 * stylesheet cannot know the value.
 *
 * Lives in components/template because that is where it is used most; the
 * Processor Load pane imports it from here. It is a generic primitive and
 * belongs in components/ui alongside Input and Checkbox.
 */
export function SliderField({
  value,
  min,
  max,
  step = 1,
  disabled = false,
  label,
  readout,
  onChange,
}: SliderFieldProps) {
  const span = max - min;
  const pct = span > 0 ? ((value - min) / span) * 100 : 0;

  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        className="capstone-slider min-w-0 flex-1 disabled:cursor-not-allowed disabled:opacity-50"
        style={{ '--slider-pct': `${pct}%` } as React.CSSProperties}
        value={value}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        aria-label={label}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span className="mono w-16 shrink-0 text-right text-xs text-gray-300">{readout}</span>
    </div>
  );
}
