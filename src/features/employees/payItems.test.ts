import { describe, expect, it } from "vitest";
import { checkPayItem } from "./payItems";

const rice = {
  kind: "ALLOWANCE",
  label: " Rice subsidy ",
  amount: "1,000",
  schedule: "EVERY_CUTOFF",
  startDate: "2026-10-16",
  endDate: "",
  taxable: "on",
};

describe("checkPayItem", () => {
  it("turns the form into the backend input", () => {
    expect(checkPayItem(rice)).toEqual({
      ok: true,
      value: {
        kind: "ALLOWANCE",
        label: "Rice subsidy",
        amountCents: 100_000,
        taxable: true,
        schedule: "EVERY_CUTOFF",
        startDate: "2026-10-16",
        endDate: null,
        remainingBalanceCents: null,
      },
    });
  });

  it("keeps the balance for loans only, and the tax flag for allowances only", () => {
    const loan = checkPayItem({ ...rice, kind: "LOAN", balance: "24,000.50" });
    expect(loan.ok && [loan.value.remainingBalanceCents, loan.value.taxable]).toEqual([
      2_400_050,
      false,
    ]);
    const other = checkPayItem({ ...rice, kind: "DEDUCTION", balance: "5" });
    expect(other.ok && [other.value.remainingBalanceCents, other.value.taxable]).toEqual([
      null,
      false,
    ]);
    const untaxed = checkPayItem({ ...rice, taxable: undefined as unknown as string });
    expect(untaxed.ok && untaxed.value.taxable).toBe(false);
  });

  it("points at each mistake", () => {
    expect(
      checkPayItem({
        kind: "LOAN",
        label: " ",
        amount: "abc",
        schedule: "EVERY_CUTOFF",
        startDate: "2026-10-16",
        endDate: "2026-10-15",
        balance: "",
      }),
    ).toEqual({
      ok: false,
      errors: {
        label: "Enter a name, such as Rice subsidy",
        amount: "Enter an amount like 1,000.00",
        endDate: "End on or after the start date",
        balance: "Enter what is still owed",
      },
    });
    expect(checkPayItem({ ...rice, amount: "0", startDate: "" })).toEqual({
      ok: false,
      errors: { amount: "Enter an amount like 1,000.00", startDate: "Enter the date it starts" },
    });
  });
});
