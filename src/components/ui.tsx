// Small shared pieces so every screen's buttons and badges look the same.
// Colours come from the Ledger tokens in src/index.css (ADR-013).

/** The one focus ring, for anything that draws its own. */
export const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-canvas";

const button = `inline-flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1 text-sm whitespace-nowrap ${focusRing} disabled:cursor-not-allowed disabled:opacity-60`;

/** The main action on a screen. Use at most one per view. */
export const primaryButton = `${button} bg-accent font-medium text-accent-ink hover:bg-accent-hover`;

/** A second action that still needs to stand out, such as Export CSV. */
export const secondaryButton = `${button} border border-rule-strong bg-surface text-ink hover:bg-sunken`;

/** Row and inline actions. */
export const quietButton = `${button} text-ink-muted hover:bg-sunken hover:text-ink`;

const TONES = {
  good: "bg-accent-soft text-accent",
  bad: "bg-danger-soft text-danger",
  warn: "bg-warn-soft text-warn",
  muted: "bg-sunken text-ink-muted",
  /** A finished state that can't change, such as a posted payroll. */
  final: "bg-accent text-accent-ink",
};

export type Tone = keyof typeof TONES;

export function Badge({ tone, children }: { tone: Tone; children: string }) {
  return (
    <span className={`rounded px-1.5 py-0.5 text-xs font-medium whitespace-nowrap ${TONES[tone]}`}>
      {children}
    </span>
  );
}

export function ActiveBadge({ active }: { active: boolean }) {
  return <Badge tone={active ? "good" : "muted"}>{active ? "Active" : "Inactive"}</Badge>;
}
