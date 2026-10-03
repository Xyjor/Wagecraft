import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { formatClock, shiftDate, todayIso } from "@/features/attendance/format";

export const STATUS_LABELS: Record<OvertimeRequest["status"], string> = {
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

/** "5:00 PM – 8:00 PM", with "(next day)" when the block ends after the work date. */
export function overtimeHours(r: Pick<OvertimeRequest, "workDate" | "startAt" | "endAt">): string {
  const nextDay = r.endAt.slice(0, 10) !== r.workDate ? " (next day)" : "";
  return `${formatClock(r.startAt)} – ${formatClock(r.endAt)}${nextDay}`;
}

/** The month before and after today: 62 days, the longest range the backend lists. */
export function recentRange(today = todayIso()) {
  return { from: shiftDate(today, -31), to: shiftDate(today, 31) };
}
