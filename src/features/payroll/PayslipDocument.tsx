import { Document, Page, StyleSheet, Text, View } from "@react-pdf/renderer";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PayslipLine } from "@/bindings/PayslipLine";
import { formatDate } from "@/lib/dates";
import { CompanyBlock } from "./CompanyBlock";
import { formatAmount, payslipSections } from "./payslipPdf";
import { periodLabel, quantityText } from "./periods";

const styles = StyleSheet.create({
  page: { padding: 40, fontSize: 10, fontFamily: "Helvetica", color: "#18181b" },
  company: { fontSize: 16, fontFamily: "Helvetica-Bold" },
  title: { fontSize: 11, color: "#52525b", marginTop: 8, marginBottom: 16 },
  header: { flexDirection: "row", justifyContent: "space-between", marginBottom: 16 },
  muted: { color: "#71717a" },
  bold: { fontFamily: "Helvetica-Bold" },
  heading: {
    fontFamily: "Helvetica-Bold",
    marginTop: 12,
    paddingBottom: 3,
    borderBottom: "1pt solid #d4d4d8",
    flexDirection: "row",
  },
  row: { flexDirection: "row", paddingVertical: 2 },
  total: { flexDirection: "row", paddingVertical: 3, borderTop: "1pt solid #d4d4d8" },
  net: { flexDirection: "row", marginTop: 12, padding: 6, backgroundColor: "#f4f4f5" },
  label: { flex: 3 },
  qty: { flex: 2, color: "#71717a" },
  amount: { flex: 2, textAlign: "right" },
  footer: { position: "absolute", bottom: 30, left: 40, right: 40, color: "#71717a" },
});

/** Plan §6.6's payslip: header, earnings, deductions, totals, then the employer's share. */
export function PayslipDocument({ slip }: { slip: PayslipDetail }) {
  const s = payslipSections(slip);
  const job = [slip.position, slip.department].filter(Boolean).join(", ");
  return (
    <Document title={`Payslip ${slip.employeeNo} ${slip.periodStart}`}>
      <Page size="A4" style={styles.page}>
        <CompanyBlock company={slip.company} fallback="Payslip" nameStyle={styles.company} />
        <Text style={styles.title}>Payslip</Text>
        <View style={styles.header}>
          <View>
            <Text style={styles.bold}>{slip.employeeName}</Text>
            <Text>{slip.employeeNo}</Text>
            {job ? <Text>{job}</Text> : null}
          </View>
          <View>
            <Text>Pay period: {periodLabel(slip.periodStart, slip.periodEnd)}</Text>
            <Text>Pay date: {formatDate(slip.payDate)}</Text>
          </View>
        </View>

        <Section title="Earnings" lines={s.earnings} />
        <Row label="Gross pay" amountCents={s.grossCents} style={styles.total} />

        <Section title="Deductions" lines={s.deductions} />
        <Row label="Total deductions" amountCents={s.totalDeductionsCents} style={styles.total} />

        <View style={styles.net}>
          <Text style={[styles.label, styles.bold]}>Net pay</Text>
          <Text style={[styles.amount, styles.bold]}>PHP {formatAmount(s.netCents)}</Text>
        </View>

        {s.employer.length > 0 ? (
          <Section title="Employer contributions" lines={s.employer} />
        ) : null}

        <Text style={styles.footer} fixed>
          This is a system-generated payslip.
        </Text>
      </Page>
    </Document>
  );
}

function Section({ title, lines }: { title: string; lines: PayslipLine[] }) {
  return (
    <View>
      <View style={styles.heading}>
        <Text style={styles.label}>{title}</Text>
        <Text style={styles.qty} />
        <Text style={styles.amount}>PHP</Text>
      </View>
      {lines.map((l, i) => (
        // Several loans or deductions share a code, so the position keeps keys unique.
        <View key={`${i}-${l.code}`} style={styles.row}>
          <Text style={styles.label}>{l.label}</Text>
          <Text style={styles.qty}>{quantityText(l.quantity, l.unit)}</Text>
          <Text style={styles.amount}>{formatAmount(l.amountCents)}</Text>
        </View>
      ))}
    </View>
  );
}

function Row({
  label,
  amountCents,
  style,
}: {
  label: string;
  amountCents: number;
  style: (typeof styles)[keyof typeof styles];
}) {
  return (
    <View style={style}>
      <Text style={[styles.label, styles.bold]}>{label}</Text>
      <Text style={styles.qty} />
      <Text style={[styles.amount, styles.bold]}>{formatAmount(amountCents)}</Text>
    </View>
  );
}
