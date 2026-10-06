import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { PayrollRegister } from "@/bindings/PayrollRegister";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { RegisterPage } from "./RegisterPage";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const period: PayrollRegister["period"] = {
  id: 4,
  periodStart: "2026-10-16",
  periodEnd: "2026-10-31",
  payDate: "2026-10-31",
  cutoffNo: 2,
  status: "DRAFT",
  rulePackCode: "PH-2026",
  createdByName: "maria",
  createdAt: "2026-10-16T01:00:00Z",
};

const computed: PayrollRegister = {
  period: { ...period, status: "COMPUTED" },
  rows: [
    {
      payslipId: 11,
      employeeNo: "EMP-0001",
      employeeName: "Santos, Ana",
      grossCents: 1_181_752,
      statutoryEeCents: 103_750,
      taxCents: 5_445,
      otherDeductionsCents: 0,
      netCents: 1_072_557,
      warnings: [],
    },
    {
      payslipId: 12,
      employeeNo: "EMP-0002",
      employeeName: "Cruz, Ben",
      grossCents: 645_000,
      statutoryEeCents: 50_000,
      taxCents: 0,
      otherDeductionsCents: 0,
      netCents: 595_000,
      warnings: ["No time-out on Oct 19, 2026, so no hours were counted that day."],
    },
  ],
  skipped: [
    {
      employeeNo: "EMP-0003",
      employeeName: "Reyes, Cy",
      reason: "No pay rate in effect for this period",
    },
  ],
};

const slip: PayslipDetail = {
  id: 11,
  periodStart: "2026-10-16",
  periodEnd: "2026-10-31",
  payDate: "2026-10-31",
  employeeNo: "EMP-0001",
  employeeName: "Santos, Ana",
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
      kind: "DEDUCTION",
      code: "SSS",
      label: "SSS",
      quantity: "1",
      unit: null,
      amountCents: -62_500,
      taxable: false,
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

let current: PayrollRegister;

function renderPage() {
  render(
    <SessionContext.Provider value={{ me: hr, signOut: async () => {} }}>
      <MemoryRouter initialEntries={["/payroll/4"]}>
        <Routes>
          <Route path="/payroll/:id" element={<RegisterPage />} />
        </Routes>
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

describe("RegisterPage", () => {
  beforeEach(() => {
    current = { period, rows: [], skipped: [] };
    call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "payroll_register") return args?.periodId === 4 ? current : undefined;
      if (cmd === "payroll_compute") {
        current = computed;
        return computed;
      }
      if (cmd === "payslip_get") return slip;
      const to = (status: PayrollRegister["period"]["status"]) => {
        current = { ...current, period: { ...current.period, status } };
        return current;
      };
      if (cmd === "payroll_approve") return to("APPROVED");
      if (cmd === "payroll_send_back") return to("COMPUTED");
      if (cmd === "payroll_post") return to("POSTED");
      return undefined;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("computes a draft and shows one row per payslip with totals", async () => {
    renderPage();
    expect(
      await screen.findByRole("heading", { name: "Payroll for Oct 16 – 31, 2026" }),
    ).toBeTruthy();
    expect(screen.getByText("Not computed yet.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Compute payroll" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("payroll_compute", { periodId: 4 }));

    const ana = (await screen.findByText("Santos, Ana")).closest("tr")!;
    expect(within(ana).getByText("₱11,817.52")).toBeTruthy();
    expect(within(ana).getByText("₱10,725.57")).toBeTruthy();
    const totals = screen.getByText("Total (2)").closest("tr")!;
    expect(within(totals).getByText("₱18,267.52")).toBeTruthy();
    expect(within(totals).getByText("₱16,675.57")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Recompute" })).toBeTruthy();
  });

  it("shows warnings and who was left out", async () => {
    current = computed;
    renderPage();
    expect(
      await screen.findByText("No time-out on Oct 19, 2026, so no hours were counted that day."),
    ).toBeTruthy();
    const skipped = screen.getByRole("region", { name: "Not in this payroll" });
    expect(within(skipped).getByText(/Reyes, Cy \(EMP-0003\)/)).toBeTruthy();
    expect(within(skipped).getByText(/No pay rate in effect for this period/)).toBeTruthy();
  });

  it("opens a payslip line by line", async () => {
    current = computed;
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "View payslip for Santos, Ana" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("payslip_get", { id: 11 }));
    const detail = await screen.findByRole("region", { name: "Payslip for Santos, Ana" });
    expect(within(detail).getByText("1 day")).toBeTruthy();
    expect(within(detail).getByText("-₱1,149.43")).toBeTruthy();
    expect(within(detail).getByText("SSS (employer)")).toBeTruthy();
    expect(within(detail).getByText("₱10,725.57")).toBeTruthy();
  });

  it("does not offer to recompute an approved period", async () => {
    current = { ...computed, period: { ...period, status: "APPROVED" } };
    renderPage();
    await screen.findByText("Santos, Ana");
    expect(screen.queryByRole("button", { name: "Recompute" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Compute payroll" })).toBeNull();
  });

  it("shows the server's message when compute fails", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "payroll_register") return current;
      throw {
        code: "CONFLICT",
        message: "This pay period is approved or posted, so it can't be recomputed.",
        fields: [],
      };
    });
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Compute payroll" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "This pay period is approved or posted, so it can't be recomputed.",
    );
  });

  it("approves a computed period, then posts it after confirming", async () => {
    current = computed;
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Approve" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("payroll_approve", { periodId: 4 }));
    expect(await screen.findByText("Approved")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Post payroll" }));
    const confirm = screen.getByRole("region", { name: "Post this payroll" });
    expect(within(confirm).getByText(/can't be undone/)).toBeTruthy();
    fireEvent.click(within(confirm).getByRole("button", { name: "Not yet" }));
    expect(call).not.toHaveBeenCalledWith("payroll_post", expect.anything());

    fireEvent.click(screen.getByRole("button", { name: "Post payroll" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, post it" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("payroll_post", { periodId: 4 }));
    expect(await screen.findByText(/Posted\. These payslips are final/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Approve|Post payroll|Send back/ })).toBeNull();
  });

  it("sends an approved period back with a reason", async () => {
    current = { ...computed, period: { ...period, status: "APPROVED" } };
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Send back" }));
    const form = screen.getByRole("form", { name: "Send back" });
    fireEvent.click(within(form).getByRole("button", { name: "Send back" }));
    expect(await within(form).findByText("Say why, in 3 to 200 characters")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("payroll_send_back", expect.anything());

    fireEvent.change(within(form).getByLabelText("Reason"), {
      target: { value: "Wrong OT for Santos" },
    });
    fireEvent.click(within(form).getByRole("button", { name: "Send back" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("payroll_send_back", {
        periodId: 4,
        reason: "Wrong OT for Santos",
      }),
    );
    expect(await screen.findByRole("button", { name: "Recompute" })).toBeTruthy();
  });

  it("shows why posting was refused", async () => {
    current = { ...computed, period: { ...period, status: "APPROVED" } };
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "payroll_register") return current;
      throw { code: "CONFLICT", message: "Post the earlier pay periods first.", fields: [] };
    });
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Post payroll" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, post it" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Post the earlier pay periods first.",
    );
  });
});
