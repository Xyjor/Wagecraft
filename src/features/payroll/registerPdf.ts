import type { DocumentProps } from "@react-pdf/renderer";
import { createElement, type ReactElement } from "react";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { RegisterReport } from "@/bindings/RegisterReport";

// The payroll register PDF (plan §6.8). The layout is in RegisterDocument.tsx and, like the
// payslip, loads only when someone downloads one.

/** "payroll-register-2026-10-16.pdf" */
export function registerFileName(report: RegisterReport): string {
  return `payroll-register-${report.period.periodStart}.pdf`;
}

/** A warning printed on a register whose numbers can still change. Empty once posted. */
export function registerStatusNote(status: PayrollPeriod["status"]): string {
  if (status === "POSTED") return "";
  return `Not final: this payroll is ${status.toLowerCase()}, not posted.`;
}

/** The register as PDF bytes. */
export async function renderRegisterPdf(report: RegisterReport): Promise<Uint8Array<ArrayBuffer>> {
  const [{ pdf }, { RegisterDocument }] = await Promise.all([
    import("@react-pdf/renderer"),
    import("./RegisterDocument"),
  ]);
  // pdf() is typed to take a bare <Document>; RegisterDocument renders one.
  const doc = createElement(RegisterDocument, { report }) as unknown as ReactElement<DocumentProps>;
  const blob = await pdf(doc).toBlob();
  return new Uint8Array(await blob.arrayBuffer());
}
