// @vitest-environment node
// Real Node Blobs, so the PDF bytes come out as a browser would make them.
import { describe, expect, it } from "vitest";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PayslipLine } from "@/bindings/PayslipLine";
import { formatAmount, payslipFileName, payslipSections, renderPayslipPdf } from "./payslipPdf";

const line = (
  kind: PayslipLine["kind"],
  code: string,
  label: string,
  amountCents: number,
  quantity = "1",
  unit: PayslipLine["unit"] = null,
): PayslipLine => ({ kind, code, label, quantity, unit, amountCents, taxable: false });

// Plan §7.7's worked example (golden case 01), worked out by hand.
const slip: PayslipDetail = {
  id: 11,
  periodStart: "2026-10-16",
  periodEnd: "2026-10-31",
  payDate: "2026-10-31",
  companyName: "Acme Trading",
  employeeNo: "EMP-0001",
  employeeName: "Santos, Ana",
  department: "Operations",
  position: "Driver",
  payBasis: "MONTHLY",
  rateCents: 2_500_000,
  lines: [
    line("EARNING", "BASIC", "Basic pay", 1_250_000),
    line("EARNING", "ABSENT", "Absences", -114_943, "1", "DAYS"),
    line("EARNING", "LATE", "Late", -7_184, "30", "MINUTES"),
    line("EARNING", "OT", "Overtime", 53_879, "3", "HOURS"),
    line("DEDUCTION", "SSS", "SSS", -62_500),
    line("DEDUCTION", "PHILHEALTH", "PhilHealth", -31_250),
    line("DEDUCTION", "PAGIBIG", "Pag-IBIG", -10_000),
    line("DEDUCTION", "TAX", "Withholding tax", -5_445),
    line("EMPLOYER_SHARE", "SSS_ER", "SSS (employer)", 125_000),
  ],
  grossCents: 1_181_752,
  taxableCents: 1_078_002,
  statutoryEeCents: 103_750,
  taxCents: 5_445,
  otherDeductionsCents: 0,
  netCents: 1_072_557,
  warnings: [],
};

const inflate = (data: Uint8Array<ArrayBuffer>) =>
  new Response(
    new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate")),
  ).arrayBuffer();

describe("payslip PDF", () => {
  it("writes amounts without the peso sign, which the PDF's built-in font lacks", () => {
    expect(formatAmount(1_250_000)).toBe("12,500.00");
    expect(formatAmount(-114_943)).toBe("-1,149.43");
    expect(formatAmount(5)).toBe("0.05");
  });

  it("names the file after the employee and the period", () => {
    expect(payslipFileName(slip)).toBe("payslip-EMP-0001-2026-10-16.pdf");
  });

  it("splits the lines and totals them so gross minus deductions is net", () => {
    const s = payslipSections(slip);
    expect(s.earnings.map((l) => l.code)).toEqual(["BASIC", "ABSENT", "LATE", "OT"]);
    // Deductions are shown as positive amounts under their own heading.
    expect(s.deductions.map((l) => [l.code, l.amountCents])).toEqual([
      ["SSS", 62_500],
      ["PHILHEALTH", 31_250],
      ["PAGIBIG", 10_000],
      ["TAX", 5_445],
    ]);
    expect(s.employer.map((l) => l.code)).toEqual(["SSS_ER"]);
    expect([s.grossCents, s.totalDeductionsCents, s.netCents]).toEqual([
      1_181_752, 109_195, 1_072_557,
    ]);
  });

  it("renders a real PDF file", async () => {
    const bytes = await renderPayslipPdf(slip);
    expect(new TextDecoder().decode(bytes.slice(0, 5))).toBe("%PDF-");
    // Every page stream must unpack. jsdom's Blob mangles binary bytes, which keeps the
    // header but breaks the compressed page, so a viewer shows a blank page.
    const text = new TextDecoder("latin1").decode(bytes);
    const streams = [...text.matchAll(/\/Length (\d+)[^]*?stream\r?\n/g)];
    expect(streams.length).toBeGreaterThan(0);
    for (const m of streams) {
      const start = m.index + m[0].length;
      await expect(inflate(bytes.slice(start, start + Number(m[1])))).resolves.toBeDefined();
    }
  }, 20_000);
});
