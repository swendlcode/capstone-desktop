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
 *
 * The disabled look is a colour swap, not `opacity`. Opacity composes with
 * every dimmed ancestor, and a field that dimmed itself inside a row that also
 * dimmed landed at an effective 0.2 — 1.38:1, unreadable (see `FieldRow`). A
 * disabled field drops to `gray-400`, which is the ink token solved for this
 * one's plane: the field is a well, so `gray-400` is the 4.5:1 step there
 * (measured 4.55:1 dark, 5.11:1 light) while `gray-500`, correct on a card,
 * would only reach 3.3:1 in here. `-webkit-text-fill-color` is pinned to
 * `currentColor` because WebKit otherwise greys a disabled input's text itself
 * and ignores the token. The border does not change: dimming it as well left
 * the field a borderless slab, and the drop from white to `gray-400` is the
 * signal.
 */
export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ leading, suffix, className = '', disabled = false, ...rest }, ref) => (
    <div
      className={`flex h-9 items-center gap-2 rounded-md border border-gray-600 bg-gray-700 px-3 text-sm focus-within:border-capstone-sky ${
        disabled ? 'cursor-not-allowed text-gray-400' : 'text-capstone-white'
      } ${className}`}
    >
      {leading}
      <input
        ref={ref}
        className="min-w-0 flex-1 bg-transparent text-inherit outline-none placeholder:text-gray-400 disabled:cursor-not-allowed"
        style={{ WebkitTextFillColor: 'currentColor' }}
        disabled={disabled}
        {...rest}
      />
      {/* The unit already sits in the small-label tier at gray-400, so it does
          not dim further: the value above it dropping from white to gray-400,
          the flattened border and the cursor are the disabled signal. */}
      {suffix && <span className="mono shrink-0 text-xs text-gray-400">{suffix}</span>}
    </div>
  ),
);
Input.displayName = 'Input';
