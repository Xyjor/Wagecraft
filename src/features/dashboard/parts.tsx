import { useId, type ReactNode } from "react";

/** One number with its label; a group, so tests and screen readers find it by name. */
export function Tile({
  label,
  value,
  children,
}: {
  label: string;
  value: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className="rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <p className="text-sm text-zinc-600 dark:text-zinc-400">{label}</p>
      <p className="text-3xl font-semibold tabular-nums">{value}</p>
      {children && <p className="text-xs text-zinc-500">{children}</p>}
    </div>
  );
}

/** A titled box, named by its title so it reads as a landmark. */
export function Panel({ title, children }: { title: string; children: ReactNode }) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className="space-y-2 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <h2 id={id} className="text-sm font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}
