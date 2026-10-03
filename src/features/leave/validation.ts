import type { BalanceAdjustment } from "@/bindings/BalanceAdjustment";
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
