import type { ReactNode } from "react";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PayslipLine } from "@/bindings/PayslipLine";
import { Money } from "@/components/Money";
import { savePdf } from "./api";
import { payslipFileName, renderPayslipPdf } from "./payslipPdf";
import { SaveButton } from "./SaveButton";
import { quantityText } from "./periods";

/** One payslip, line by line: what the employee earned and paid, then the employer's share. */
export function PayslipView({ slip, label }: { slip: PayslipDetail; label: string }) {
  const employee = slip.lines.filter((l) => l.kind !== "EMPLOYER_SHARE");
  const employer = slip.lines.filter((l) => l.kind === "EMPLOYER_SHARE");

  return (
    <section
      aria-label={label}
      className="grid gap-6 rounded-lg border border-rule bg-surface p-4 sm:grid-cols-2"
    >
      <LineTable title="Earnings and deductions" lines={employee}>
        <tr className="border-t-[3px] border-double border-rule-strong font-semibold">
          <td className="py-1.5">Net pay</td>
          <td />
          <td className="py-1.5 text-right tabular-nums">
            <Money cents={slip.netCents} />
          </td>
        </tr>
      </LineTable>
      <LineTable title="Employer contributions" lines={employer} />
      <DownloadPdf slip={slip} label={label} />
    </section>
  );
}

function LineTable({
  title,
  lines,
  children,
}: {
  title: string;
  lines: PayslipLine[];
  children?: ReactNode;
}) {
  return (
    <table className="w-full self-start text-sm">
      <caption className="border-b border-rule-strong pb-1 text-left font-medium">{title}</caption>
      <tbody>
        {lines.map((l, i) => (
          // Several loans or deductions share a code, so the position keeps keys unique.
          <tr key={`${i}-${l.code}`} className="border-b border-rule">
            <td className="py-1">{l.label}</td>
            <td className="py-1 text-ink-faint">{quantityText(l.quantity, l.unit)}</td>
            <td className="py-1 text-right tabular-nums">
              <Money cents={l.amountCents} />
            </td>
          </tr>
        ))}
        {children}
      </tbody>
    </table>
  );
}

function DownloadPdf({ slip, label }: { slip: PayslipDetail; label: string }) {
  return (
    <div className="sm:col-span-2">
      <SaveButton
        label="Download PDF"
        busyLabel="Making PDF…"
        ariaLabel={`Download PDF of ${label}`}
        save={async () => savePdf(payslipFileName(slip), await renderPayslipPdf(slip))}
      />
    </div>
  );
}
