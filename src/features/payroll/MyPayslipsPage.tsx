import { useEffect, useState } from "react";
import type { MyPayslip } from "@/bindings/MyPayslip";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import { FormAlert } from "@/components/form";
import { quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { getMyPayslip, myPayslips } from "./api";
import { PayslipView } from "./PayslipView";
import { periodLabel } from "./periods";

/** The signed-in person's payslips (`/my-payslips`), once HR has posted each payroll. */
export function MyPayslipsPage() {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn&apos;t linked to an employee record. HR can link it from your employee
        profile.
      </p>
    );
  }
  return <MyPayslips />;
}

function MyPayslips() {
  const [slips, setSlips] = useState<MyPayslip[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [open, setOpen] = useState<number | null>(null);

  useEffect(() => {
    let live = true;
    myPayslips()
      .then((s) => live && setSlips(s))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, []);

  return (
    <div className="max-w-4xl space-y-6">
      <h1 className="text-xl font-semibold">My payslips</h1>
      <FormAlert message={alert} />
      {slips?.length === 0 && (
        <p className="text-sm text-zinc-500">
          No payslips yet. Each one shows up here once HR posts that payroll.
        </p>
      )}
      {slips && slips.length > 0 && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Pay period</th>
              <th className="py-2 font-medium">Pay date</th>
              <th className="py-2 text-right font-medium">Gross</th>
              <th className="py-2 text-right font-medium">Net pay</th>
              <th className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {slips.map((s) => {
              const period = periodLabel(s.periodStart, s.periodEnd);
              const isOpen = open === s.id;
              return [
                <tr key={s.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2">{period}</td>
                  <td className="py-2">{formatDate(s.payDate)}</td>
                  <td className="py-2 text-right tabular-nums">{formatPesos(s.grossCents)}</td>
                  <td className="py-2 text-right font-medium tabular-nums">
                    {formatPesos(s.netCents)}
                  </td>
                  <td className="py-2 text-right">
                    <button
                      type="button"
                      className={quietButton}
                      aria-expanded={isOpen}
                      aria-label={`${isOpen ? "Hide" : "View"} payslip for ${period}`}
                      onClick={() => setOpen(isOpen ? null : s.id)}
                    >
                      {isOpen ? "Hide" : "View"}
                    </button>
                  </td>
                </tr>,
                isOpen && (
                  <tr key={`${s.id}-slip`}>
                    <td colSpan={5} className="pb-4">
                      <Payslip id={s.id} label={`Payslip for ${period}`} />
                    </td>
                  </tr>
                ),
              ];
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

function Payslip({ id, label }: { id: number; label: string }) {
  const [slip, setSlip] = useState<PayslipDetail | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    let live = true;
    getMyPayslip(id)
      .then((s) => live && setSlip(s))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [id]);

  if (!slip) return <FormAlert message={alert} />;
  return <PayslipView slip={slip} label={label} />;
}
