import { forwardRef } from 'react';
import type { InputHTMLAttributes, ReactNode } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  leading?: ReactNode;
  /** A unit such as dB, %, or cents. Rendered in the mono face. */
  suffix?: ReactNode;
}

/**
 * Text or number field. Pass `className="mono"` for the numeric ones — dB,
 * cents and percentages all belong in DM Mono so columns of them line up.
 */
export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ leading, suffix, className = '', ...rest }, ref) => (
    <div
      className={`flex h-9 items-center gap-2 rounded-md border border-gray-600 bg-gray-700 px-3 text-sm text-capstone-white focus-within:border-capstone-sky ${className}`}
    >
      {leading}
      <input
        ref={ref}
        className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-gray-400"
        {...rest}
      />
      {suffix && <span className="mono shrink-0 text-xs text-gray-400">{suffix}</span>}
    </div>
  ),
);
Input.displayName = 'Input';
