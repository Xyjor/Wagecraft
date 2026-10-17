import { Document, Page, StyleSheet, Text, View } from "@react-pdf/renderer";
import type { RegisterReport } from "@/bindings/RegisterReport";
import type { RegisterReportRow } from "@/bindings/RegisterReportRow";
import { formatDate } from "@/lib/dates";
import { CompanyBlock } from "./CompanyBlock";
import { formatAmount } from "./payslipPdf";
import { amountInk, doubleRule, PDF } from "./pdfTheme";
import { periodLabel } from "./periods";
import { registerStatusNote } from "./registerPdf";

const COLUMNS: { key: keyof RegisterReportRow; label: string }[] = [
  { key: "grossCents", label: "Gross pay" },
  { key: "sssCents", label: "SSS" },
  { key: "philhealthCents", label: "PhilHealth" },
  { key: "pagibigCents", label: "Pag-IBIG" },
  { key: "taxCents", label: "Withholding tax" },
  { key: "otherDeductionsCents", label: "Other deductions" },
  { key: "netCents", label: "Net pay" },
];

const styles = StyleSheet.create({
  page: { padding: 32, fontSize: 8, fontFamily: "Helvetica", color: PDF.ink },
  company: { fontSize: 14, fontFamily: "Helvetica-Bold" },
  title: { fontSize: 10, color: PDF.muted, marginTop: 6, marginBottom: 4 },
  note: { color: PDF.danger, fontFamily: "Helvetica-Bold", marginBottom: 4 },
  meta: { color: PDF.muted, marginBottom: 12 },
  head: {
    flexDirection: "row",
    fontFamily: "Helvetica-Bold",
    borderBottom: `1pt solid ${PDF.ruleStrong}`,
    paddingBottom: 3,
  },
  row: { flexDirection: "row", paddingVertical: 2, borderBottom: `0.5pt solid ${PDF.rule}` },
  total: { flexDirection: "row", paddingVertical: 3, fontFamily: "Helvetica-Bold" },
  no: { width: 60 },
  name: { flex: 2 },
  amount: { flex: 1, textAlign: "right" },
  footer: { position: "absolute", bottom: 20, left: 32, right: 32, color: PDF.faint },
});

/** One row per employee, a column per deduction, and the totals (plan §6.8). Amounts in PHP. */
export function RegisterDocument({ report }: { report: RegisterReport }) {
  const { period } = report;
  const note = registerStatusNote(period.status);
  return (
    <Document title={`Payroll register ${period.periodStart}`}>
      <Page size="A4" orientation="landscape" style={styles.page}>
        <CompanyBlock
          company={report.company}
          fallback="Payroll register"
          nameStyle={styles.company}
        />
        <Text style={styles.title}>
          Payroll register, {periodLabel(period.periodStart, period.periodEnd)}
        </Text>
        {note ? <Text style={styles.note}>{note}</Text> : null}
        <Text style={styles.meta}>
          Pay date {formatDate(period.payDate)} · Rule pack {period.rulePackCode} · Amounts in PHP
        </Text>

        <View style={styles.head} fixed>
          <Text style={styles.no}>Employee no.</Text>
          <Text style={styles.name}>Employee</Text>
          {COLUMNS.map((c) => (
            <Text key={c.key} style={styles.amount}>
              {c.label}
            </Text>
          ))}
        </View>
        {report.rows.map((r) => (
          <Row key={r.employeeNo} row={r} style={styles.row} />
        ))}
        <View style={doubleRule.above}>
          <View style={doubleRule.aboveInner}>
            <Row row={report.totals} style={styles.total} />
          </View>
        </View>

        <Text
          style={styles.footer}
          fixed
          render={({ pageNumber, totalPages }) =>
            `${report.rows.length} employees · Page ${pageNumber} of ${totalPages}`
          }
        />
      </Page>
    </Document>
  );
}

function Row({
  row,
  style,
}: {
  row: RegisterReportRow;
  style: (typeof styles)[keyof typeof styles];
}) {
  return (
    <View style={style} wrap={false}>
      <Text style={styles.no}>{row.employeeNo}</Text>
      <Text style={styles.name}>{row.employeeName}</Text>
      {COLUMNS.map((c) => (
        <Text key={c.key} style={[styles.amount, amountInk(row[c.key] as number)]}>
          {formatAmount(row[c.key] as number)}
        </Text>
      ))}
    </View>
  );
}
