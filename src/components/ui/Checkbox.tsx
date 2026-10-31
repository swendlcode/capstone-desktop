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
 */
export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, className = '', ...rest }, ref) => (
    <label
      className={`inline-flex cursor-pointer items-center gap-2 text-sm text-capstone-white ${className}`}
    >
      <input
        ref={ref}
        type="checkbox"
        className="h-4 w-4 shrink-0 cursor-pointer rounded border-gray-600 bg-gray-700 [accent-color:var(--color-accent)]"
        {...rest}
      />
      {label}
    </label>
  ),
);
Checkbox.displayName = 'Checkbox';
