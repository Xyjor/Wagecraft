import type { BalanceAdjustment } from "@/bindings/BalanceAdjustment";
import type { LeaveRequestInput } from "@/bindings/LeaveRequestInput";
import type { LeaveTypeInput } from "@/bindings/LeaveTypeInput";
import { parseDays } from "./format";

// Mirrors services/leave.rs, so the form can point at a mistake before the round trip.
// The Rust checks are the real ones.

type Result<T> = { ok: true; value: T } | { ok: false; errors: Record<string, string> };

const DAYS_HINT = "Enter whole or half days, like 5 or 2.5";

export function checkLeaveType(v: Record<string, string>): Result<LeaveTypeInput> {
  const errors: Record<string, string> = {};
  const code = (v.code ?? "").trim().toUpperCase();
  const name = (v.name ?? "").trim().replace(/\s+/g, " ");
  const isPaid = v.isPaid === "on";
  const halfdays = parseDays(v.defaultDays ?? "");
  const months = Number(v.minServiceMonths);
  if (!/^[A-Z0-9]{2,10}$/.test(code)) errors.code = "Use 2 to 10 letters or numbers";
  if (!name || name.length > 60) errors.name = "Enter a name (up to 60 characters)";
  if (halfdays === null) errors.defaultDays = DAYS_HINT;
  else if (!isPaid && halfdays > 0) {
    errors.defaultDays = "Unpaid leave uses no balance, so leave this at 0";
  }
  if (!v.minServiceMonths?.trim() || !Number.isInteger(months) || months < 0 || months > 120) {
    errors.minServiceMonths = "Enter 0 to 120 months";
  }
  if (Object.keys(errors).length || halfdays === null) return { ok: false, errors };
  return {
    ok: true,
    value: {
      code,
      name,
      isPaid,
      defaultHalfdaysPerYear: halfdays,
      minServiceMonths: months,
    },
  };
}

export function checkAdjustment(
  v: Record<string, string>,
  used: number,
): Result<BalanceAdjustment> {
  const errors: Record<string, string> = {};
  const halfdays = parseDays(v.days ?? "");
  const reason = (v.reason ?? "").trim();
  if (halfdays === null) errors.days = DAYS_HINT;
  else if (halfdays < used) errors.days = "That's less than what's already used";
  if (reason.length < 3 || reason.length > 200) errors.reason = "Say why, in 3 to 200 characters";
  if (Object.keys(errors).length || halfdays === null) return { ok: false, errors };
  return { ok: true, value: { entitledHalfdays: halfdays, reason } };
}

const DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A leave request. An empty end date means a one-day leave. */
export function checkLeaveRequest(
  v: Record<string, string>,
  forSomeone: boolean,
): Result<LeaveRequestInput> {
  const errors: Record<string, string> = {};
  const employeeNo = (v.employeeNo ?? "").trim();
  const leaveTypeId = Number(v.leaveTypeId);
  const startDate = (v.startDate ?? "").trim();
  const endDate = (v.endDate ?? "").trim() || startDate;
  const halfDay = v.halfDay === "on";
  const reason = (v.reason ?? "").trim();
  if (forSomeone && !employeeNo) errors.employeeNo = "Enter the employee number";
  if (!Number.isInteger(leaveTypeId) || leaveTypeId <= 0) errors.leaveTypeId = "Pick a leave type";
  if (!DATE.test(startDate)) errors.startDate = "Pick a start date";
  else if (!DATE.test(endDate)) errors.endDate = "Pick an end date";
  else if (endDate < startDate) errors.endDate = "The end date is before the start date";
  else if (endDate.slice(0, 4) !== startDate.slice(0, 4)) {
    errors.endDate = "File leave across New Year as two requests, one for each year";
  } else if (halfDay && endDate !== startDate) {
    errors.halfDay = "A half day starts and ends on the same date";
  }
  if (reason.length < 3 || reason.length > 200) {
    errors.reason = "Say what the leave is for, in 3 to 200 characters";
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: {
      employeeNo: forSomeone ? employeeNo : null,
      leaveTypeId,
      startDate,
      endDate,
      halfDay,
      reason,
    },
  };
}
