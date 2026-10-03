import type { DepartmentInput } from "@/bindings/DepartmentInput";
import type { PositionInput } from "@/bindings/PositionInput";
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
