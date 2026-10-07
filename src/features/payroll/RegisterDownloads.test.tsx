import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RegisterReport } from "@/bindings/RegisterReport";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { RegisterDownloads } from "./RegisterDownloads";

const totals = {
  employeeNo: "",
  employeeName: "Total",
  grossCents: 1_250_000,
  sssCents: 62_500,
  philhealthCents: 31_250,
  pagibigCents: 10_000,
  taxCents: 15_683,
  otherDeductionsCents: 0,
  netCents: 1_130_567,
};
const report: RegisterReport = {
  period: {
    id: 7,
    periodStart: "2026-10-16",
    periodEnd: "2026-10-31",
    payDate: "2026-10-31",
    cutoffNo: 2,
    status: "POSTED",
    rulePackCode: "PH-2026",
    createdByName: "maria",
    createdAt: "2026-10-16T01:00:00Z",
  },
  company: { name: "Acme Trading", address: "", tin: "", logo: null },
  rows: [{ ...totals, employeeNo: "EMP-0001", employeeName: "Santos, Ana" }],
  totals,
};

// Braces matter: a function returned from beforeEach runs as teardown.
beforeEach(() => {
  call.mockReset();
});
afterEach(cleanup);

describe("RegisterDownloads", () => {
  it("saves the register as CSV", async () => {
    call.mockResolvedValue("C:\\Reports\\payroll-register-2026-10-16.csv");
    render(<RegisterDownloads periodId={7} />);
    fireEvent.click(screen.getByRole("button", { name: "Download CSV" }));

    expect(
      await screen.findByText("Saved to C:\\Reports\\payroll-register-2026-10-16.csv"),
    ).toBeTruthy();
    expect(call).toHaveBeenCalledWith("report_payroll_register_csv", { periodId: 7 });
  });

  it("draws the register as a PDF and saves it", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "report_payroll_register") return report;
      if (cmd === "export_save_pdf") return "C:\\Reports\\payroll-register-2026-10-16.pdf";
      throw new Error(`unexpected ${cmd}`);
    });
    render(<RegisterDownloads periodId={7} />);
    fireEvent.click(screen.getByRole("button", { name: "Download PDF" }));

    expect(
      await screen.findByText("Saved to C:\\Reports\\payroll-register-2026-10-16.pdf"),
    ).toBeTruthy();
    expect(call).toHaveBeenCalledWith("report_payroll_register", { periodId: 7 });
    const save = call.mock.calls.find(([cmd]) => cmd === "export_save_pdf")!;
    expect(save[1].fileName).toBe("payroll-register-2026-10-16.pdf");
    expect(save[1].bytes.slice(0, 5)).toEqual([37, 80, 68, 70, 45]);
  }, 20_000);

  it("shows why an export failed", async () => {
    call.mockRejectedValue({ code: "CONFLICT", message: "No payslips yet", fields: [] });
    render(<RegisterDownloads periodId={7} />);
    fireEvent.click(screen.getByRole("button", { name: "Download CSV" }));

    expect((await screen.findByRole("alert")).textContent).toBe("No payslips yet");
  });
});
