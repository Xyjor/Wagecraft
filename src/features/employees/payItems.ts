import type { RecurringItemInput } from "@/bindings/RecurringItemInput";
import { parsePesos } from "@/lib/money";

// Mirrors services/recurring_items.rs so the form can point at a mistake early.
// The Rust checks are the real ones.

type Result =
  { ok: true; value: RecurringItemInput } | { ok: false; errors: Record<string, string> };

export const KIND_LABELS: Record<string, string> = {
  ALLOWANCE: "Allowance",
  LOAN: "Loan",
  DEDUCTION: "Deduction",
};

export const SCHEDULE_LABELS: Record<string, string> = {
  EVERY_CUTOFF: "Every cutoff",
  FIRST_CUTOFF: "1st to 15th",
  SECOND_CUTOFF: "16th to end of month",
};

/** Form field names for the backend's RecurringItemInput fields. */
export const PAY_ITEM_FIELD: Record<string, string> = {
  amountCents: "amount",
  remainingBalanceCents: "balance",
};

export function checkPayItem(v: Record<string, string>): Result {
  const errors: Record<string, string> = {};
  const label = (v.label ?? "").trim();
  if (!label) errors.label = "Enter a name, such as Rice subsidy";
  else if (label.length > 60) errors.label = "Keep the name under 60 characters";
  const amountCents = parsePesos(v.amount ?? "");
  if (!amountCents) errors.amount = "Enter an amount like 1,000.00";
  const startDate = (v.startDate ?? "").trim();
  if (!startDate) errors.startDate = "Enter the date it starts";
  const endDate = (v.endDate ?? "").trim() || null;
  if (endDate && startDate && endDate < startDate) {
    errors.endDate = "End on or after the start date";
  }
  const loan = v.kind === "LOAN";
  const balance = loan ? parsePesos(v.balance ?? "") : null;
  if (loan && balance === null) errors.balance = "Enter what is still owed";
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: {
      kind: v.kind,
      label,
      amountCents: amountCents!,
      taxable: v.kind === "ALLOWANCE" && v.taxable === "on",
      schedule: v.schedule,
      startDate,
      endDate,
      remainingBalanceCents: balance,
    },
  };
}
