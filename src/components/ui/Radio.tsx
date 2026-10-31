import { forwardRef } from 'react';
import type { InputHTMLAttributes, ReactNode } from 'react';

interface RadioProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: ReactNode;
  /** One short line under the label, for the format descriptions. */
  hint?: ReactNode;
}

/**
 * One option in a radio group. Callers set a shared `name` per group; without
 * it the browser treats each radio as its own group and lets two be chosen at
 * once.
 *
 * Disabled dims by swapping the ink token, never with `opacity` — see the note
 * in `Input`. A group of these therefore needs no dimmed wrapper of its own.
 */
export const Radio = forwardRef<HTMLInputElement, RadioProps>(
  ({ label, hint, className = '', disabled = false, ...rest }, ref) => (
    <label
      className={`flex items-start gap-2 text-sm ${
        disabled ? 'cursor-not-allowed text-gray-500' : 'cursor-pointer text-capstone-white'
      } ${className}`}
    >
      <input
        ref={ref}
        type="radio"
        className="mt-0.5 h-4 w-4 shrink-0 disabled:cursor-not-allowed [accent-color:var(--color-accent)]"
        disabled={disabled}
        {...rest}
      />
      <span className="min-w-0">
        <span className="block leading-snug">{label}</span>
        {hint && (
          <span className={`block text-xs ${disabled ? 'text-gray-500' : 'text-gray-400'}`}>
            {hint}
          </span>
        )}
      </span>
    </label>
  ),
);
Radio.displayName = 'Radio';
