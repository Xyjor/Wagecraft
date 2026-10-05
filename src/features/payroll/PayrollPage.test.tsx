import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { PeriodChecks } from "@/bindings/PeriodChecks";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { PayrollPage } from "./PayrollPage";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function period(over: Partial<PayrollPeriod>): PayrollPeriod {
  return {
    id: 1,
    periodStart: "2026-10-01",
    periodEnd: "2026-10-15",
    payDate: "2026-10-15",
    cutoffNo: 1,
    status: "POSTED",
    rulePackCode: "PH-2026",
    createdByName: "maria",
    createdAt: "2026-10-01T01:00:00Z",
    ...over,
  };
}

const NO_CHECKS: PeriodChecks = { pendingLeave: [], pendingOvertime: [], missingTimeOuts: [] };

let periods: PayrollPeriod[];
let checks: Record<string, PeriodChecks>;

function renderAs(who: Me = hr) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <PayrollPage today="2026-10-20" />
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("PayrollPage", () => {
  beforeEach(() => {
    periods = [
      period({
        id: 2,
        periodStart: "2026-10-16",
        periodEnd: "2026-10-31",
        payDate: "2026-11-05",
        cutoffNo: 2,
        status: "DRAFT",
      }),
      period({}),
    ];
    checks = {};
    call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "payroll_period_list") return periods;
      if (cmd === "rule_pack_list")
        return [
          {
            id: 7,
            code: "PH-2026",
            name: "Philippines 2026",
            effectiveFrom: "2026-01-01",
            effectiveTo: null,
          },
        ];
      if (cmd === "payroll_period_checks") return checks[args?.periodStart as string] ?? NO_CHECKS;
      return undefined;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("tells Staff that payroll is for Admin and HR", () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(screen.getByText("Only Admin and HR can run payroll.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("lists the pay periods with their status", async () => {
    renderAs();
    const draft = (await screen.findByText("Oct 16 – 31, 2026")).closest("tr")!;
    expect(within(draft).getByText("Nov 5, 2026")).toBeTruthy();
    expect(within(draft).getByText("PH-2026")).toBeTruthy();
    expect(within(draft).getByText("Draft")).toBeTruthy();
    const posted = screen.getByText("Oct 1 – 15, 2026").closest("tr")!;
    expect(within(posted).getByText("Posted")).toBeTruthy();
    expect(within(posted).queryByRole("button", { name: /Delete/ })).toBeNull();
  });

  it("creates a period, suggesting the cutoff's last day as the pay date", async () => {
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "New pay period" }));
    type("Month", "2026-11");
    type("Cutoff", "1");
    await waitFor(() =>
      expect((screen.getByLabelText("Pay date") as HTMLInputElement).value).toBe("2026-11-15"),
    );
    expect((screen.getByLabelText("Rule pack") as HTMLSelectElement).value).toBe("7");

    fireEvent.click(screen.getByRole("button", { name: "Create pay period" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("payroll_period_create", {
        input: { periodStart: "2026-11-01", payDate: "2026-11-15", rulePackId: 7 },
      }),
    );
    await waitFor(() => expect(screen.queryByLabelText("Month")).toBeNull());
    expect(call.mock.calls.filter(([c]) => c === "payroll_period_list")).toHaveLength(2);
  });

  it("warns about unfinished work inside the period", async () => {
    checks["2026-10-16"] = {
      pendingLeave: [{ employeeNo: "EMP-0001", employeeName: "Reyes, Ana", date: "2026-10-14" }],
      pendingOvertime: [],
      missingTimeOuts: [
        { employeeNo: "EMP-0002", employeeName: "Cruz, Ben", date: "2026-10-19" },
        { employeeNo: "EMP-0003", employeeName: "Lim, Cy", date: "2026-10-20" },
      ],
    };
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "New pay period" }));
    const warnings = await screen.findByRole("region", { name: "Unfinished work in this period" });
    expect(within(warnings).getByText("1 pending leave request")).toBeTruthy();
    expect(within(warnings).getByText("2 missing time-outs")).toBeTruthy();
    expect(within(warnings).queryByText(/overtime/)).toBeNull();
    expect(within(warnings).getByText("Reyes, Ana (EMP-0001), from Oct 14, 2026")).toBeTruthy();
    expect(within(warnings).getByText("Lim, Cy (EMP-0003), Oct 20, 2026")).toBeTruthy();

    type("Cutoff", "1");
    await waitFor(() =>
      expect(
        screen.getByText("No pending leave, overtime or missing time-outs in this period."),
      ).toBeTruthy(),
    );
  });

  it("shows the server's message under the field it is about", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "payroll_period_create")
        throw {
          code: "VALIDATION",
          message: "Some fields are invalid",
          fields: [
            {
              field: "periodStart",
              message: "There is already a pay period for Oct 16 to 31, 2026",
            },
          ],
        };
      if (cmd === "payroll_period_list") return periods;
      if (cmd === "rule_pack_list")
        return [
          {
            id: 7,
            code: "PH-2026",
            name: "Philippines 2026",
            effectiveFrom: "2026-01-01",
            effectiveTo: null,
          },
        ];
      return NO_CHECKS;
    });
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "New pay period" }));
    await screen.findByLabelText("Rule pack");
    fireEvent.click(screen.getByRole("button", { name: "Create pay period" }));
    expect(
      await screen.findByText("There is already a pay period for Oct 16 to 31, 2026"),
    ).toBeTruthy();
    expect(screen.getByLabelText("Month").getAttribute("aria-invalid")).toBe("true");
  });

  it("deletes a draft after asking", async () => {
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "Delete Oct 16 – 31, 2026" }));
    expect(call).not.toHaveBeenCalledWith("payroll_period_delete", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("payroll_period_delete", { id: 2 }));
  });
});
