import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PayslipDetail } from "@/bindings/PayslipDetail";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { PayslipView } from "./PayslipView";

const slip: PayslipDetail = {
  id: 11,
  periodStart: "2026-10-01",
  periodEnd: "2026-10-15",
  payDate: "2026-10-15",
  company: { name: "Acme Trading", address: "", tin: "", logo: null },
  employeeNo: "EMP-0001",
  employeeName: "Santos, Ana",
  department: null,
  position: null,
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
  ],
  grossCents: 1_250_000,
  taxableCents: 1_250_000,
  statutoryEeCents: 0,
  taxCents: 0,
  otherDeductionsCents: 0,
  netCents: 1_250_000,
  warnings: [],
};

// Braces matter: a function returned from beforeEach runs as teardown, and mockReset
// returns the mock itself.
beforeEach(() => {
  call.mockReset();
});
afterEach(cleanup);

// These tests draw a real PDF. The first one also loads the PDF library, which on a cold or
// busy machine takes longer than Testing Library's default 1 second wait.
const PDF_WAIT = { timeout: 15_000 };

const download = () =>
  fireEvent.click(screen.getByRole("button", { name: "Download PDF of Oct payslip" }));

describe("PayslipView download", () => {
  it("saves the payslip as a PDF named after the employee and period", async () => {
    call.mockResolvedValue("C:\\Users\\ana\\payslip-EMP-0001-2026-10-01.pdf");
    render(<PayslipView slip={slip} label="Oct payslip" />);
    download();

    expect(
      await screen.findByText(
        "Saved to C:\\Users\\ana\\payslip-EMP-0001-2026-10-01.pdf",
        undefined,
        PDF_WAIT,
      ),
    ).toBeTruthy();
    const [cmd, args] = call.mock.calls[0];
    expect(cmd).toBe("export_save_pdf");
    expect(args.fileName).toBe("payslip-EMP-0001-2026-10-01.pdf");
    // Plain numbers, so Tauri sends them as a byte list: "%PDF-".
    expect(args.bytes.slice(0, 5)).toEqual([37, 80, 68, 70, 45]);
  }, 20_000);

  it("says nothing when the save dialog is cancelled", async () => {
    call.mockResolvedValue(null);
    render(<PayslipView slip={slip} label="Oct payslip" />);
    download();

    const button = screen.getByRole("button", { name: "Download PDF of Oct payslip" });
    await vi.waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false), PDF_WAIT);
    expect(call).toHaveBeenCalledOnce();
    expect(screen.queryByText(/Saved to/)).toBeNull();
  }, 20_000);

  it("shows why a save failed", async () => {
    call.mockRejectedValue({
      code: "CONFLICT",
      message: "That file couldn't be saved",
      fields: [],
    });
    render(<PayslipView slip={slip} label="Oct payslip" />);
    download();

    expect(
      await screen.findByText("That file couldn't be saved", undefined, PDF_WAIT),
    ).toBeTruthy();
  }, 20_000);
});
