import { useId, type ReactNode } from "react";

// The tone colours the number; the label still says what it is, so colour isn't the only cue.
const TILE_TONES = {
  danger: "[&>p:nth-child(2)]:text-danger",
  warn: "[&>p:nth-child(2)]:text-warn",
};

/**
 * A ruled band of figures, like the totals strip across the top of a ledger page. Tiles sit
 * in it as cells, divided by column rules and closed by a double rule.
 */
export function TileStrip({
  columns,
  inPanel = false,
  children,
}: {
  columns: string;
  /** Inside a Panel, which already draws the box: the cells run to its edges. */
  inPanel?: boolean;
  children: ReactNode;
}) {
  const box = inPanel
    ? "-mx-4 -mb-4 border-t border-rule"
    : "rounded-sm border border-b-[3px] border-double border-rule-strong bg-surface";
  return <div className={`grid divide-x divide-rule ${box} ${columns}`}>{children}</div>;
}

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
    <div role="group" aria-label={label} className={`px-4 py-3 ${tone ? TILE_TONES[tone] : ""}`}>
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
      className="space-y-2 rounded-sm border border-rule bg-surface p-4"
    >
      <h2 id={id} className="text-sm font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}
