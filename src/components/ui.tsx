// Small shared pieces so every screen's buttons and badges look the same.

const button =
  "rounded-md px-2.5 py-1 text-sm focus-visible:ring-2 focus-visible:ring-sky-500 disabled:opacity-60";

export const primaryButton = `${button} bg-zinc-900 font-medium text-white hover:bg-zinc-700 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300`;

export const quietButton = `${button} text-zinc-700 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800`;

const TONES = {
  good: "bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300",
  bad: "bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-300",
  muted: "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300",
};

export function Badge({ tone, children }: { tone: keyof typeof TONES; children: string }) {
  return <span className={`rounded px-1.5 py-0.5 text-xs ${TONES[tone]}`}>{children}</span>;
}

export function ActiveBadge({ active }: { active: boolean }) {
  return <Badge tone={active ? "good" : "muted"}>{active ? "Active" : "Inactive"}</Badge>;
}
