// @vitest-environment node
// Real Node Blobs, so the PDF bytes come out as a browser would make them.
import { describe, expect, it } from "vitest";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { RegisterReport } from "@/bindings/RegisterReport";
import type { RegisterReportRow } from "@/bindings/RegisterReportRow";
import { BROKEN_LOGO, PNG_LOGO, hasImage } from "@/test/logo";
import { expectReadablePdf } from "@/test/pdf";
import { registerFileName, registerStatusNote, renderRegisterPdf } from "./registerPdf";

const period: PayrollPeriod = {
  id: 7,
  periodStart: "2026-10-16",
  periodEnd: "2026-10-31",
  payDate: "2026-10-31",
  cutoffNo: 2,
  status: "POSTED",
  rulePackCode: "PH-2026",
  createdByName: "maria",
  createdAt: "2026-10-16T01:00:00Z",
};

const row = (no: string, name: string, gross: number, net: number): RegisterReportRow => ({
  employeeNo: no,
  employeeName: name,
  grossCents: gross,
  sssCents: 62_500,
  philhealthCents: 31_250,
  pagibigCents: 10_000,
  taxCents: 5_445,
  otherDeductionsCents: 0,
  netCents: net,
});

const report: RegisterReport = {
  period,
  company: {
    name: "Acme Trading",
    address: "12 Rizal St, Makati",
    tin: "123456789000",
    logo: null,
  },
  rows: [
    row("EMP-0002", "Reyes, Ana", 1_250_000, 1_140_805),
    row("EMP-0001", "Santos, Ana", 1_181_752, 1_072_557),
  ],
  totals: {
    ...row("", "Total", 2_431_752, 2_213_362),
    sssCents: 125_000,
    philhealthCents: 62_500,
    pagibigCents: 20_000,
    taxCents: 10_890,
  },
};

describe("register PDF", () => {
  it("names the file after the period", () => {
    expect(registerFileName(report)).toBe("payroll-register-2026-10-16.pdf");
  });

  it("marks a register that isn't posted yet as not final", () => {
    expect(registerStatusNote("POSTED")).toBe("");
    expect(registerStatusNote("COMPUTED")).toBe("Not final: this payroll is computed, not posted.");
    expect(registerStatusNote("APPROVED")).toBe("Not final: this payroll is approved, not posted.");
  });

  it("renders a real PDF file", async () => {
    await expectReadablePdf(await renderRegisterPdf(report));
  }, 20_000);

  it("prints the company logo", async () => {
    const bytes = await renderRegisterPdf({
      ...report,
      company: { ...report.company, logo: PNG_LOGO },
    });
    await expectReadablePdf(bytes);
    expect(hasImage(bytes)).toBe(true);
  }, 20_000);

  it("still makes the PDF, without the logo, if the logo is damaged", async () => {
    const bytes = await renderRegisterPdf({
      ...report,
      company: { ...report.company, logo: BROKEN_LOGO },
    });
    await expectReadablePdf(bytes);
    expect(hasImage(bytes)).toBe(false);
  }, 20_000);
});
