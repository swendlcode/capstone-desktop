import type { ReactNode } from 'react';

interface PaneProps {
  title: string;
  children: ReactNode;
}

/** One pane of the Personalize tab, to the right of the rail. */
export function Pane({ title, children }: PaneProps) {
  return (
    <section className="min-w-0 flex-1">
      <h2 className="text-xs font-bold uppercase tracking-wider text-capstone-sky">
        {title}
      </h2>
      <div className="mt-4 flex flex-col gap-4">{children}</div>
    </section>
  );
}
