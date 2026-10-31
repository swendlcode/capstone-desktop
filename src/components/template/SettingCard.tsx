import type { ReactNode } from 'react';

interface SettingCardProps {
  title: string;
  /** One short line under the title. The copy stays terse on purpose. */
  hint?: ReactNode;
  children: ReactNode;
}

/** One of the four setting cards. Shell only: no state, no layout opinions. */
export function SettingCard({ title, hint, children }: SettingCardProps) {
  return (
    <section className="flex flex-col gap-4 rounded-xl border border-gray-700 bg-gray-800 p-5">
      <header>
        <h2 className="text-xs font-bold uppercase tracking-wider text-capstone-sky">
          {title}
        </h2>
        {hint && <p className="mt-1 text-xs text-gray-400">{hint}</p>}
      </header>
      {children}
    </section>
  );
}

interface FieldRowProps {
  label: string;
  /** Width of the control column; the label takes the rest. */
  control: ReactNode;
  disabled?: boolean;
}

/**
 * A label on the left, a control on the right, aligned across cards.
 *
 * A dependent row dims its LABEL by swapping the ink token, and nothing else.
 * It used to put `opacity-40` on the whole row while the field inside it
 * carried its own `opacity-50`: opacity multiplies down the tree, so every
 * dependent row sat at an effective 0.2 and "When off key by at least" with
 * its cents field measured 1.38:1 in dark (1.28:1 in light) — and that is the
 * default state whenever pitch correction is off, on stock and user templates
 * alike. Each control now owns its own disabled look (see `Input`), so the two
 * can never compound again. `gray-500` is 4.0:1 on a card in dark and 4.5:1 in
 * light, and still a visible step down from the `gray-300` of a live row.
 */
export function FieldRow({ label, control, disabled = false }: FieldRowProps) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className={`min-w-0 text-sm ${disabled ? 'text-gray-500' : 'text-gray-300'}`}>
        {label}
      </span>
      <div className="w-28 shrink-0">{control}</div>
    </div>
  );
}
