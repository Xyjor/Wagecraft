import type { HolidayInput } from "@/bindings/HolidayInput";
import type { DepartmentInput } from "@/bindings/DepartmentInput";
import type { PositionInput } from "@/bindings/PositionInput";
import type { WorkScheduleInput } from "@/bindings/WorkScheduleInput";
import { parsePesos } from "@/lib/money";

// Mirrors services/org.rs, so the form can point at a mistake before the round trip.
// The Rust checks are the real ones.

type Result<T> = { ok: true; value: T } | { ok: false; errors: Record<string, string> };

export function checkDepartment(v: Record<string, string>): Result<DepartmentInput> {
  const errors: Record<string, string> = {};
  const code = (v.code ?? "").trim();
  const name = (v.name ?? "").trim();
  if (!/^[A-Za-z0-9-]{2,10}$/.test(code)) errors.code = "Use 2 to 10 letters, numbers or dashes";
  if (!name || name.length > 100) errors.name = "Enter a name (up to 100 characters)";
  if (Object.keys(errors).length) return { ok: false, errors };
  return { ok: true, value: { code, name, description: v.description?.trim() || null } };
}

const AMOUNT_HINT = "Enter an amount like 18,000.00";

/** Form field names for the backend's PositionInput fields. */
export const POSITION_FIELD: Record<string, string> = {
  departmentId: "departmentId",
  title: "title",
  minRateCents: "minRate",
  maxRateCents: "maxRate",
};

export function checkPosition(v: Record<string, string>): Result<PositionInput> {
  const errors: Record<string, string> = {};
  const departmentId = Number(v.departmentId);
  const title = (v.title ?? "").trim();
  const min = v.minRate?.trim() ? parsePesos(v.minRate) : undefined;
  const max = v.maxRate?.trim() ? parsePesos(v.maxRate) : undefined;
  if (!departmentId) errors.departmentId = "Pick a department";
  if (!title || title.length > 100) errors.title = "Enter a title (up to 100 characters)";
  if (min === null) errors.minRate = AMOUNT_HINT;
  if (max === null) errors.maxRate = AMOUNT_HINT;
  else if (min != null && max != null && min > max) {
    errors.maxRate = "The maximum must not be below the minimum";
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: { departmentId, title, minRateCents: min ?? null, maxRateCents: max ?? null },
  };
}

/** Weekday codes as the backend stores them, in calendar order. */
export const WEEKDAYS = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"] as const;

/** Minutes from start to end, wrapping past midnight: 22:00 to 06:00 is 480. */
export function shiftMinutes(start: string, end: string): number {
  const toMin = (t: string) => Number(t.slice(0, 2)) * 60 + Number(t.slice(3, 5));
  const m = toMin(end) - toMin(start);
  return m <= 0 ? m + 24 * 60 : m;
}

/**
 * Checks the schedule form. Work days arrive as one field per checked day (`day-MON`),
 * because FormData keeps only one value per name.
 */
export function checkSchedule(v: Record<string, string>): Result<WorkScheduleInput> {
  const errors: Record<string, string> = {};
  const name = (v.name ?? "").trim();
  const startTime = (v.startTime ?? "").trim();
  const endTime = (v.endTime ?? "").trim();
  const breakMinutes = Number(v.breakMinutes);
  const graceMinutes = Number(v.graceMinutes);
  const workDays = WEEKDAYS.filter((d) => v[`day-${d}`]).join(",");
  const time = /^([01]\d|2[0-3]):[0-5]\d$/;
  if (!name || name.length > 60) errors.name = "Enter a name (up to 60 characters)";
  if (!time.test(startTime)) errors.startTime = "Enter a time like 08:00";
  if (!time.test(endTime)) errors.endTime = "Enter a time like 17:00";
  else if (startTime === endTime) errors.endTime = "The shift must end at a different time";
  if (
    !v.breakMinutes?.trim() ||
    !Number.isInteger(breakMinutes) ||
    breakMinutes < 0 ||
    breakMinutes > 240
  ) {
    errors.breakMinutes = "Enter 0 to 240 minutes";
  } else if (
    !errors.startTime &&
    !errors.endTime &&
    breakMinutes >= shiftMinutes(startTime, endTime)
  ) {
    errors.breakMinutes = "The break must be shorter than the shift";
  }
  if (
    !v.graceMinutes?.trim() ||
    !Number.isInteger(graceMinutes) ||
    graceMinutes < 0 ||
    graceMinutes > 60
  ) {
    errors.graceMinutes = "Enter 0 to 60 minutes";
  }
  if (!workDays) errors.workDays = "Pick at least one work day";
  if (Object.keys(errors).length) return { ok: false, errors };
  return { ok: true, value: { name, startTime, endTime, breakMinutes, graceMinutes, workDays } };
}

export const HOLIDAY_KINDS = ["REGULAR", "SPECIAL_NON_WORKING", "SPECIAL_WORKING"] as const;

/** Checks the holiday form. The date must be a real calendar day from 2000 to 2100. */
export function checkHoliday(v: Record<string, string>): Result<HolidayInput> {
  const errors: Record<string, string> = {};
  const date = (v.date ?? "").trim();
  const name = (v.name ?? "").trim().replace(/\s+/g, " ");
  const kind = HOLIDAY_KINDS.find((k) => k === v.kind);
  if (!isRealDate(date)) errors.date = "Enter a date from 2000 to 2100";
  if (name.length < 2 || name.length > 80) errors.name = "Enter a name (2 to 80 characters)";
  if (!kind) errors.kind = "Pick a holiday type";
  if (Object.keys(errors).length || !kind) return { ok: false, errors };
  return { ok: true, value: { date, name, kind } };
}

function isRealDate(iso: string): boolean {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!m) return false;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const date = new Date(y, mo - 1, d);
  return y >= 2000 && y <= 2100 && date.getMonth() === mo - 1 && date.getDate() === d;
}
