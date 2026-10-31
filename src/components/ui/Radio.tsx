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
 */
export const Radio = forwardRef<HTMLInputElement, RadioProps>(
  ({ label, hint, className = '', ...rest }, ref) => (
    <label
      className={`flex cursor-pointer items-start gap-2 text-sm text-capstone-white ${className}`}
    >
      <input
        ref={ref}
        type="radio"
        className="mt-0.5 h-4 w-4 shrink-0 cursor-pointer [accent-color:var(--color-accent)]"
        {...rest}
      />
      <span className="min-w-0">
        <span className="block leading-snug">{label}</span>
        {hint && <span className="block text-xs text-gray-400">{hint}</span>}
      </span>
    </label>
  ),
);
Radio.displayName = 'Radio';
