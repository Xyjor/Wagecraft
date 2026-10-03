import type { OvertimeInput } from "@/bindings/OvertimeInput";

// Mirrors services/overtime.rs, so the form can point at a mistake before the round trip.
// The Rust checks are the real ones: the schedule, holidays and overlaps live there.

type Result<T> = { ok: true; value: T } | { ok: false; errors: Record<string, string> };

const TIME = /^([01]\d|2[0-3]):[0-5]\d$/;

export function checkOvertime(
  v: Record<string, string>,
  forSomeone: boolean,
): Result<OvertimeInput> {
  const errors: Record<string, string> = {};
  const employeeNo = (v.employeeNo ?? "").trim();
  const workDate = (v.workDate ?? "").trim();
  const startTime = (v.startTime ?? "").trim();
  const endTime = (v.endTime ?? "").trim();
  const reason = (v.reason ?? "").trim();
  if (forSomeone && !employeeNo) errors.employeeNo = "Enter the employee number";
  if (!/^\d{4}-\d{2}-\d{2}$/.test(workDate)) errors.workDate = "Pick a date";
  if (!TIME.test(startTime)) errors.startTime = "Enter a time like 17:00";
  if (!TIME.test(endTime)) errors.endTime = "Enter a time like 20:00";
  else if (startTime === endTime) errors.endTime = "Overtime can be at most 12 hours";
  if (reason.length < 3 || reason.length > 200) {
    errors.reason = "Say what the overtime is for, in 3 to 200 characters";
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: { employeeNo: forSomeone ? employeeNo : null, workDate, startTime, endTime, reason },
  };
}
