import { useEffect, useRef, useState } from 'react';
import { Input } from '../ui/Input';

interface NumberFieldProps {
  value: number;
  min: number;
  max: number;
  /** Decimal places the committed value is rounded and displayed to. */
  decimals?: number;
  step?: number;
  /** dB, %, cents, Hz — rendered in the mono face by `Input`. */
  unit?: string;
  disabled?: boolean;
  label: string;
  className?: string;
  onChange: (value: number) => void;
}

function round(value: number, decimals: number): number {
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
}

/**
 * A clamped numeric field in DM Mono.
 *
 * It keeps the keystrokes in local state rather than deriving the text from the
 * number, because a controlled number input cannot hold "-" or "-1." long
 * enough to type "-12.5" — each intermediate state parses to something else and
 * the caret fights the user.
 *
 * So: typing only commits while the parse is finite and in range, and blur (or
 * Enter) clamps, rounds and rewrites the text. The prop is adopted back
 * whenever it changes from outside and the field is not focused, which is how a
 * duplicate or a template switch repaints it.
 */
export function NumberField({
  value,
  min,
  max,
  decimals = 1,
  step,
  unit,
  disabled = false,
  label,
  className = '',
  onChange,
}: NumberFieldProps) {
  const [text, setText] = useState(() => value.toFixed(decimals));
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) setText(value.toFixed(decimals));
  }, [value, decimals]);

  const commitText = (raw: string) => {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      setText(value.toFixed(decimals));
      return;
    }
    const next = round(Math.min(Math.max(parsed, min), max), decimals);
    setText(next.toFixed(decimals));
    if (next !== value) onChange(next);
  };

  return (
    <Input
      // Dimmed rather than only non-interactive: a stock template disables
      // every field on the tab, and fields that look live but refuse keystrokes
      // read as a broken window. The dimming itself belongs to `Input`, which
      // does it with an ink token: an `opacity` here multiplied with the one a
      // dependent `FieldRow` used to apply and buried the field at 1.38:1.
      className={`mono ${className}`}
      type="number"
      inputMode="decimal"
      value={text}
      min={min}
      max={max}
      step={step ?? (decimals === 0 ? 1 : 10 ** -decimals)}
      disabled={disabled}
      aria-label={label}
      suffix={unit}
      onFocus={() => {
        focused.current = true;
      }}
      onChange={(e) => {
        const raw = e.target.value;
        setText(raw);
        const parsed = Number(raw);
        if (raw !== '' && Number.isFinite(parsed) && parsed >= min && parsed <= max) {
          const next = round(parsed, decimals);
          if (next !== value) onChange(next);
        }
      }}
      onBlur={(e) => {
        focused.current = false;
        commitText(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commitText((e.target as HTMLInputElement).value);
      }}
    />
  );
}
