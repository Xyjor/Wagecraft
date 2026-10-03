import type { Compensation } from "@/bindings/Compensation";
import type { CompensationInput } from "@/bindings/CompensationInput";
import type { Position } from "@/bindings/Position";
import { formatPesos, parsePesos } from "@/lib/money";

// Mirrors services/compensation.rs so the form can point at a mistake early.
// The Rust checks are the real ones, including "after the current rate".

type Result =
  { ok: true; value: CompensationInput } | { ok: false; errors: Record<string, string> };

export const PAY_BASIS_LABELS: Record<string, string> = { MONTHLY: "Monthly", DAILY: "Daily" };

/** Form field names for the backend's CompensationInput fields. */
export const COMPENSATION_FIELD: Record<string, string> = {
  payBasis: "payBasis",
  rateCents: "rate",
  effectiveFrom: "effectiveFrom",
  reason: "reason",
};

export function checkCompensation(v: Record<string, string>): Result {
  const errors: Record<string, string> = {};
  const rateCents = parsePesos(v.rate ?? "");
  if (!rateCents) errors.rate = "Enter an amount like 18,000.00";
  const effectiveFrom = (v.effectiveFrom ?? "").trim();
  const day = Number(effectiveFrom.slice(8, 10));
  if (!effectiveFrom) errors.effectiveFrom = "Enter the date the rate starts";
  else if (day !== 1 && day !== 16) {
    errors.effectiveFrom = "Start on the 1st or 16th, the first day of a pay period";
  }
  const reason = v.reason?.trim() || null;
  if (reason && reason.length > 200) errors.reason = "Keep the reason under 200 characters";
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: { payBasis: v.payBasis, rateCents: rateCents!, effectiveFrom, reason },
  };
}

/** A note when a monthly rate falls outside its position's range. Never blocks saving. */
export function rangeWarning(rate: Compensation, position: Position | undefined): string | null {
  if (!position || rate.payBasis !== "MONTHLY") return null;
  const { minRateCents: min, maxRateCents: max, title } = position;
  if (min !== null && rate.rateCents < min) {
    return `${formatPesos(rate.rateCents)} is below the ${title} range, which starts at ${formatPesos(min)}.`;
  }
  if (max !== null && rate.rateCents > max) {
    return `${formatPesos(rate.rateCents)} is above the ${title} range, which tops out at ${formatPesos(max)}.`;
  }
  return null;
}
