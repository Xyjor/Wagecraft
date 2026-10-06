import type { DocumentProps } from "@react-pdf/renderer";
import { createElement, type ReactElement } from "react";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PayslipLine } from "@/bindings/PayslipLine";

// The payslip PDF (plan §6.6, ADR-010). The layout lives in PayslipDocument.tsx and is
// loaded only when someone downloads a payslip, so the PDF library stays out of startup.

/**
 * 1250000 → "12,500.00". The PDF's built-in Helvetica has no "₱", so amounts are plain
 * numbers under a "PHP" column heading instead.
 */
export function formatAmount(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(cents);
  const pesos = Math.trunc(abs / 100).toLocaleString("en-US");
  return `${sign}${pesos}.${String(abs % 100).padStart(2, "0")}`;
}

/** "payslip-EMP-0001-2026-10-16.pdf" */
export function payslipFileName(slip: PayslipDetail): string {
  return `payslip-${slip.employeeNo}-${slip.periodStart}.pdf`;
}

/** The payslip split into its printed sections. Deductions read as positive amounts. */
export function payslipSections(slip: PayslipDetail) {
  const earnings = slip.lines.filter((l) => l.kind === "EARNING");
  const deductions: PayslipLine[] = slip.lines
    .filter((l) => l.kind === "DEDUCTION")
    .map((l) => ({ ...l, amountCents: -l.amountCents }));
  const employer = slip.lines.filter((l) => l.kind === "EMPLOYER_SHARE");
  return {
    earnings,
    deductions,
    employer,
    grossCents: slip.grossCents,
    totalDeductionsCents: deductions.reduce((sum, l) => sum + l.amountCents, 0),
    netCents: slip.netCents,
  };
}

export type PayslipSections = ReturnType<typeof payslipSections>;

/** The payslip as PDF bytes. */
export async function renderPayslipPdf(slip: PayslipDetail): Promise<Uint8Array<ArrayBuffer>> {
  const [{ pdf }, { PayslipDocument }] = await Promise.all([
    import("@react-pdf/renderer"),
    import("./PayslipDocument"),
  ]);
  // pdf() is typed to take a bare <Document>; PayslipDocument renders one.
  const doc = createElement(PayslipDocument, { slip }) as unknown as ReactElement<DocumentProps>;
  const blob = await pdf(doc).toBlob();
  return new Uint8Array(await blob.arrayBuffer());
}
