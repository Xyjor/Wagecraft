import { useId, type ReactNode } from "react";

// The tone colours the number; the label still says what it is, so colour isn't the only cue.
const TILE_TONES = {
  danger: "[&>p:nth-child(2)]:text-danger",
  warn: "[&>p:nth-child(2)]:text-warn",
};

/** One number with its label; a group, so tests and screen readers find it by name. */
export function Tile({
  label,
  value,
  tone,
  children,
}: {
  label: string;
  value: ReactNode;
  /** Colours the number when it needs attention, such as late arrivals. */
  tone?: keyof typeof TILE_TONES;
  children?: ReactNode;
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className={`rounded-lg border border-rule bg-surface p-4 ${tone ? TILE_TONES[tone] : ""}`}
    >
      <p className="text-sm text-ink-muted">{label}</p>
      <p className="mt-1 text-3xl font-semibold tabular-nums">{value}</p>
      {children && <p className="mt-0.5 text-xs text-ink-faint">{children}</p>}
    </div>
  );
}

/** A titled box, named by its title so it reads as a landmark. */
export function Panel({ title, children }: { title: string; children: ReactNode }) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className="space-y-2 rounded-lg border border-rule bg-surface p-4"
    >
      <h2 id={id} className="text-sm font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}
