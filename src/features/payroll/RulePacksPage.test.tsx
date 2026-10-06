import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RulePackDetail } from "@/bindings/RulePackDetail";
import type { RulePackSummary } from "@/bindings/RulePackSummary";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { percentText } from "./periods";
import { RulePacksPage } from "./RulePacksPage";

const ph2027: RulePackSummary = {
  id: 2,
  code: "PH-2027",
  name: "Philippines 2027",
  effectiveFrom: "2027-01-01",
  effectiveTo: null,
};
const ph2026: RulePackSummary = {
  id: 1,
  code: "PH-2026",
  name: "Philippines 2026",
  effectiveFrom: "2026-01-01",
  effectiveTo: "2026-12-31",
};

function detail(summary: RulePackSummary): RulePackDetail {
  return {
    summary,
    settings: {
      factorFiveDay: 261,
      factorSixDay: 313,
      sssEmployeeBp: 500,
      sssEmployerBp: 1000,
      philhealth: { rateBp: 500, floorCents: 1_000_000, ceilingCents: 10_000_000 },
      pagibig: {
        lowPayLimitCents: 150_000,
        lowRateBp: 100,
        employeeBp: 200,
        employerBp: 200,
        maxBaseCents: 1_000_000,
      },
    },
    sssBrackets: [
      {
        fromCents: 0,
        toCents: 524_999,
        mscCents: 500_000,
        eeCents: 25_000,
        erCents: 50_000,
        ecCents: 1_000,
      },
      {
        fromCents: 3_475_000,
        toCents: null,
        mscCents: 3_500_000,
        eeCents: 175_000,
        erCents: 350_000,
        ecCents: 3_000,
      },
    ],
    taxBrackets: [
      {
        frequency: "SEMI_MONTHLY",
        overCents: 1_666_700,
        notOverCents: 3_333_300,
        baseTaxCents: 93_750,
        rateBp: 2000,
      },
      {
        frequency: "MONTHLY",
        overCents: 66_666_700,
        notOverCents: null,
        baseTaxCents: 18_354_180,
        rateBp: 3500,
      },
    ],
    premiums: [
      { dayType: "ORDINARY", workBp: 10_000, otBp: 12_500, nightDiffBp: 1000 },
      { dayType: "REGULAR", workBp: 20_000, otBp: 26_000, nightDiffBp: 1000 },
    ],
  };
}

beforeEach(() => {
  call.mockReset();
  call.mockImplementation(async (cmd: string, args?: { id: number }) => {
    if (cmd === "rule_pack_list") return [ph2027, ph2026];
    if (cmd === "rule_pack_get") return detail(args!.id === 1 ? ph2026 : ph2027);
    throw new Error(`unexpected ${cmd}`);
  });
});
afterEach(cleanup);

const show = () =>
  render(
    <MemoryRouter>
      <RulePacksPage />
    </MemoryRouter>,
  );

const row = (table: string, first: string) =>
  within(screen.getByRole("table", { name: table }))
    .getByRole("rowheader", { name: first })
    .closest("tr") as HTMLElement;

describe("percentText", () => {
  it("turns basis points into a percent people read", () => {
    expect(percentText(12_500)).toBe("125%");
    expect(percentText(250)).toBe("2.5%");
    expect(percentText(1)).toBe("0.01%");
    expect(percentText(0)).toBe("0%");
  });
});

describe("RulePacksPage", () => {
  it("opens on the newest pack and shows its rates", async () => {
    show();

    expect(await screen.findByRole("heading", { name: "Philippines 2027 (PH-2027)" })).toBeTruthy();
    expect(call).toHaveBeenCalledWith("rule_pack_get", { id: 2 });
    expect(screen.getByText("In effect from Jan 1, 2027")).toBeTruthy();

    const regular = row("Premium pay", "Regular holiday");
    expect(
      within(regular)
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["200%", "260%", "10%"]);
    const sss = row("SSS contributions", "₱0.00 – ₱5,249.99");
    expect(
      within(sss)
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["₱5,000.00", "₱250.00", "₱500.00", "₱10.00"]);
    expect(row("SSS contributions", "₱34,750.00 and up")).toBeTruthy();
    const tax = row("Withholding tax, semi-monthly", "₱16,667.00 – ₱33,333.00");
    expect(
      within(tax)
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["₱937.50 + 20% of the excess over ₱16,667.00"]);
    expect(row("Withholding tax, monthly", "Over ₱666,667.00")).toBeTruthy();
    expect(
      screen.getByText("PhilHealth: 5% of basic pay, on ₱10,000.00 to ₱100,000.00 a month"),
    ).toBeTruthy();
  });

  it("switches to another pack", async () => {
    show();
    await screen.findByRole("heading", { name: "Philippines 2027 (PH-2027)" });

    fireEvent.change(screen.getByLabelText("Rule pack"), { target: { value: "1" } });

    expect(await screen.findByRole("heading", { name: "Philippines 2026 (PH-2026)" })).toBeTruthy();
    expect(call).toHaveBeenLastCalledWith("rule_pack_get", { id: 1 });
    expect(screen.getByText("In effect from Jan 1, 2026 to Dec 31, 2026")).toBeTruthy();
  });

  it("says so when no pack is set up", async () => {
    call.mockImplementation(async () => []);
    show();

    expect(await screen.findByText("No rule packs are active.")).toBeTruthy();
  });
});
