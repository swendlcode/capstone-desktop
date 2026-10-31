import { forwardRef } from 'react';
import type { InputHTMLAttributes, ReactNode } from 'react';

interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  /** Inline label. Sentence case, short — the settings copy stays terse. */
  label?: ReactNode;
}

/**
 * Native checkbox tinted with the accent. `accent-color` is used rather than a
 * hand-built box so the control keeps its platform keyboard and focus
 * behaviour, and it reads the CSS var so a runtime accent change follows.
 *
 * Disabled dims by swapping the ink token, never with `opacity` — see the note
 * in `Input`.
 */
export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, className = '', disabled = false, ...rest }, ref) => (
    <label
      className={`inline-flex items-center gap-2 text-sm ${
        disabled ? 'cursor-not-allowed text-gray-500' : 'cursor-pointer text-capstone-white'
      } ${className}`}
    >
      <input
        ref={ref}
        type="checkbox"
        className="h-4 w-4 shrink-0 rounded border-gray-600 bg-gray-700 disabled:cursor-not-allowed [accent-color:var(--color-accent)]"
        disabled={disabled}
        {...rest}
      />
      {label}
    </label>
  ),
);
Checkbox.displayName = 'Checkbox';
