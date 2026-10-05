import type { PayrollPeriod } from "@/bindings/PayrollPeriod";

// Semi-monthly cutoffs (plan §6.5): the 1st to the 15th, and the 16th to the month's end.
// Dates are `YYYY-MM-DD` text built from parts, so no time zone can shift them.

export type Cutoff = 1 | 2;

/** "2026-10", 2 → "2026-10-16" */
export function periodStart(month: string, cutoff: Cutoff): string {
  return `${month}-${cutoff === 1 ? "01" : "16"}`;
}

/** "2026-10-16" → "2026-10-31" */
export function periodEnd(start: string): string {
  const [y, m, d] = start.split("-").map(Number);
  const last = d === 1 ? 15 : new Date(y, m, 0).getDate();
  return `${start.slice(0, 8)}${String(last).padStart(2, "0")}`;
}

/** The cutoff a day falls in. */
export function cutoffOf(day: string): { month: string; cutoff: Cutoff } {
  return { month: day.slice(0, 7), cutoff: Number(day.slice(8, 10)) <= 15 ? 1 : 2 };
}

/** "Oct 16 – 31, 2026" */
export function periodLabel(start: string, end: string): string {
  const [y, m, d] = start.split("-").map(Number);
  const month = new Date(y, m - 1, d).toLocaleDateString("en-US", { month: "short" });
  return `${month} ${d} – ${Number(end.slice(8, 10))}, ${y}`;
}

export const STATUS_LABELS: Record<PayrollPeriod["status"], string> = {
  DRAFT: "Draft",
  COMPUTED: "Computed",
  APPROVED: "Approved",
  POSTED: "Posted",
};

export const STATUS_TONES = {
  DRAFT: "muted",
  COMPUTED: "muted",
  APPROVED: "good",
  POSTED: "good",
} as const;
