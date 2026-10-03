import type { LeaveRequest } from "@/bindings/LeaveRequest";
import { formatDate } from "@/lib/dates";

/** 10 → "5 days", 1 → "½ day", 3 → "1½ days", 0 → "0 days". */
export function formatDays(halfdays: number): string {
  const whole = Math.floor(halfdays / 2);
  const half = halfdays % 2 === 1 ? "½" : "";
  if (whole === 0 && half) return "½ day";
  const label = `${whole}${half}`;
  return halfdays === 2 ? "1 day" : `${label} days`;
}

/** "2.5" → 5 half days; null when it isn't a whole or half number from 0 to 365. */
export function parseDays(text: string): number | null {
  const t = text.trim();
  if (!/^\d{1,3}(\.\d+)?$/.test(t)) return null;
  const halfdays = Number(t) * 2;
  return Number.isInteger(halfdays) && halfdays <= 730 ? halfdays : null;
}

export const STATUS_LABELS: Record<LeaveRequest["status"], string> = {
  PENDING: "Waiting for HR",
  APPROVED: "Approved",
  REJECTED: "Rejected",
  CANCELLED: "Cancelled",
};

export const STATUS_TONES = {
  PENDING: "muted",
  APPROVED: "good",
  REJECTED: "bad",
  CANCELLED: "muted",
} as const;

/** "Oct 12, 2026 – Oct 16, 2026", one date for a single day, and "(half day)" when it is. */
export function leaveDates(r: Pick<LeaveRequest, "startDate" | "endDate" | "halfDay">): string {
  const dates =
    r.startDate === r.endDate
      ? formatDate(r.startDate)
      : `${formatDate(r.startDate)} – ${formatDate(r.endDate)}`;
  return r.halfDay ? `${dates} (half day)` : dates;
}
