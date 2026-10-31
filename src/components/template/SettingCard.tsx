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

/** A label on the left, a control on the right, aligned across cards. */
export function FieldRow({ label, control, disabled = false }: FieldRowProps) {
  return (
    <div
      className={`flex items-center justify-between gap-3 ${disabled ? 'opacity-40' : ''}`}
    >
      <span className="min-w-0 text-sm text-gray-300">{label}</span>
      <div className="w-28 shrink-0">{control}</div>
    </div>
  );
}
