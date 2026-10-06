import type { ReactNode } from "react";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PayslipLine } from "@/bindings/PayslipLine";
import { formatPesos } from "@/lib/money";
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
      className="grid gap-4 rounded-lg border border-zinc-200 p-4 sm:grid-cols-2 dark:border-zinc-800"
    >
      <LineTable title="Earnings and deductions" lines={employee}>
        <tr className="border-t border-zinc-300 font-medium dark:border-zinc-700">
          <td className="py-1">Net pay</td>
          <td />
          <td className="py-1 text-right tabular-nums">{formatPesos(slip.netCents)}</td>
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
      <caption className="pb-1 text-left font-medium">{title}</caption>
      <tbody>
        {lines.map((l, i) => (
          // Several loans or deductions share a code, so the position keeps keys unique.
          <tr key={`${i}-${l.code}`}>
            <td className="py-1">{l.label}</td>
            <td className="py-1 text-zinc-500">{quantityText(l.quantity, l.unit)}</td>
            <td className="py-1 text-right tabular-nums">{formatPesos(l.amountCents)}</td>
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
