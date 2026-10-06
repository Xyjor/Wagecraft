import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { MyPayslip } from "@/bindings/MyPayslip";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { MyPayslipsPage } from "./MyPayslipsPage";

const ana: Me = {
  userId: 5,
  username: "ana",
  role: "STAFF",
  employeeId: 1,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const mine: MyPayslip[] = [
  {
    id: 21,
    periodStart: "2026-10-16",
    periodEnd: "2026-10-31",
    payDate: "2026-10-31",
    grossCents: 1_250_000,
    netCents: 1_133_445,
  },
  {
    id: 11,
    periodStart: "2026-10-01",
    periodEnd: "2026-10-15",
    payDate: "2026-10-15",
    grossCents: 1_181_752,
    netCents: 1_072_557,
  },
];

const slip: PayslipDetail = {
  id: 11,
  periodStart: "2026-10-01",
  periodEnd: "2026-10-15",
  payDate: "2026-10-15",
  employeeNo: "EMP-0001",
  companyName: "Acme Trading",
  employeeName: "Santos, Ana",
  department: "Operations",
  position: "Driver",
  payBasis: "MONTHLY",
  rateCents: 2_500_000,
  lines: [
    {
      kind: "EARNING",
      code: "BASIC",
      label: "Basic pay",
      quantity: "1",
      unit: null,
      amountCents: 1_250_000,
      taxable: true,
    },
    {
      kind: "EARNING",
      code: "ABSENT",
      label: "Absences",
      quantity: "1",
      unit: "DAYS",
      amountCents: -114_943,
      taxable: true,
    },
    {
      kind: "EMPLOYER_SHARE",
      code: "SSS_ER",
      label: "SSS (employer)",
      quantity: "1",
      unit: null,
      amountCents: 125_000,
      taxable: false,
    },
  ],
  grossCents: 1_181_752,
  taxableCents: 1_078_002,
  statutoryEeCents: 103_750,
  taxCents: 5_445,
  otherDeductionsCents: 0,
  netCents: 1_072_557,
  warnings: [],
};

function renderPage(me: Me = ana) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MyPayslipsPage />
    </SessionContext.Provider>,
  );
}

describe("MyPayslipsPage", () => {
  beforeEach(() => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "payslip_my_list") return mine;
      if (cmd === "payslip_my_get") return slip;
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists posted payslips with their pay date and net pay", async () => {
    renderPage();
    const rows = (await screen.findAllByRole("row")).slice(1);
    expect(rows.map((r) => within(r).getAllByRole("cell")[0].textContent)).toEqual([
      "Oct 16 – 31, 2026",
      "Oct 1 – 15, 2026",
    ]);
    expect(within(rows[1]).getByText("Oct 15, 2026")).toBeTruthy();
    expect(within(rows[1]).getByText("₱10,725.57")).toBeTruthy();
  });

  it("opens a payslip line by line", async () => {
    renderPage();
    fireEvent.click(
      await screen.findByRole("button", { name: "View payslip for Oct 1 – 15, 2026" }),
    );
    await waitFor(() => expect(call).toHaveBeenCalledWith("payslip_my_get", { id: 11 }));
    const view = await screen.findByRole("region", { name: "Payslip for Oct 1 – 15, 2026" });
    expect(within(view).getByText("Absences")).toBeTruthy();
    expect(within(view).getByText("-₱1,149.43")).toBeTruthy();
    expect(within(view).getByText("Net pay")).toBeTruthy();
    expect(within(view).getByText("SSS (employer)")).toBeTruthy();
  });

  it("says when nothing has been posted yet", async () => {
    call.mockImplementation(async () => []);
    renderPage();
    expect(await screen.findByText(/No payslips yet/)).toBeTruthy();
  });

  it("explains an account with no employee record and asks for nothing", () => {
    renderPage({ ...ana, employeeId: null });
    expect(screen.getByText(/isn.t linked to an employee record/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});
