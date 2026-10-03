import type { WorkSchedule } from "@/bindings/WorkSchedule";
import { WEEKDAYS } from "./validation";

export const DAY_NAMES: Record<string, string> = {
  MON: "Mon",
  TUE: "Tue",
  WED: "Wed",
  THU: "Thu",
  FRI: "Fri",
  SAT: "Sat",
  SUN: "Sun",
};

/** `MON,TUE,WED,THU,FRI` reads as "Mon–Fri"; anything else is listed day by day. */
export function workDaysLabel(workDays: string): string {
  const days = workDays.split(",").filter(Boolean);
  const idx = days.map((d) => WEEKDAYS.indexOf(d as (typeof WEEKDAYS)[number]));
  const consecutive = idx.length > 2 && idx.every((n, i) => i === 0 || n === idx[i - 1] + 1);
  if (consecutive) return `${DAY_NAMES[days[0]]}–${DAY_NAMES[days[days.length - 1]]}`;
  return days.map((d) => DAY_NAMES[d] ?? d).join(", ");
}

/** "08:00–17:00", with "(next day)" when the shift ends after midnight. */
export function scheduleHours(s: WorkSchedule): string {
  const nextDay = s.endTime <= s.startTime ? " (next day)" : "";
  return `${s.startTime}–${s.endTime}${nextDay}`;
}
