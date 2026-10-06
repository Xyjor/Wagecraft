import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router";
import type { PayrollRegister } from "@/bindings/PayrollRegister";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { RegisterRow } from "@/bindings/RegisterRow";
import { Field, FormAlert } from "@/components/form";
import { Badge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import {
  approvePayroll,
  computePayroll,
  getPayslip,
  getRegister,
  postPayroll,
  sendBackPayroll,
} from "./api";
import { PayslipView } from "./PayslipView";
import { periodLabel, STATUS_LABELS, STATUS_TONES } from "./periods";

export function RegisterPage() {
  const { me } = useSession();
  const id = Number(useParams().id);
  if (me.role === "STAFF") {
    return <p className="text-zinc-600 dark:text-zinc-400">Only Admin and HR can run payroll.</p>;
  }
  return <Register key={id} id={id} />;
}

const MONEY_COLUMNS: { key: keyof RegisterRow; label: string }[] = [
  { key: "grossCents", label: "Gross" },
  { key: "statutoryEeCents", label: "Contributions" },
  { key: "taxCents", label: "Tax" },
  { key: "otherDeductionsCents", label: "Other deductions" },
  { key: "netCents", label: "Net pay" },
];

function Register({ id }: { id: number }) {
  const [register, setRegister] = useState<PayrollRegister | null>(null);
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState<number | null>(null);

  useEffect(() => {
    let current = true;
    getRegister(id)
      .then((r) => current && setRegister(r))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, [id]);

  /** Runs one step (compute, approve, post) and shows the register it returns. */
  async function run(step: () => Promise<PayrollRegister>) {
    setAlert(undefined);
    setBusy(true);
    try {
      setRegister(await step());
      setOpen(null);
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setBusy(false);
    }
  }

  if (!register) return <FormAlert message={alert} />;
  const { period, rows, skipped } = register;
  const label = periodLabel(period.periodStart, period.periodEnd);
  const canCompute = period.status === "DRAFT" || period.status === "COMPUTED";
  const total = (key: keyof RegisterRow) => rows.reduce((sum, r) => sum + (r[key] as number), 0);

  return (
    <div className="space-y-6">
      <Link to="/payroll" className={quietButton}>
        ← All pay periods
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Payroll for {label}</h1>
        <Badge tone={STATUS_TONES[period.status]}>{STATUS_LABELS[period.status]}</Badge>
      </div>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Pay date {formatDate(period.payDate)} · Rule pack {period.rulePackCode}
      </p>
      <FormAlert message={alert} />
      {canCompute && (
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            className={period.status === "DRAFT" ? primaryButton : quietButton}
            disabled={busy}
            onClick={() => run(() => computePayroll(id))}
          >
            {period.status === "DRAFT" ? "Compute payroll" : "Recompute"}
          </button>
          {period.status === "COMPUTED" && (
            <>
              <button
                type="button"
                className={primaryButton}
                disabled={busy || rows.length === 0}
                onClick={() => run(() => approvePayroll(id))}
              >
                Approve
              </button>
              <span className="text-sm text-zinc-600 dark:text-zinc-400">
                Fix attendance, leave or pay rates at the source, then recompute. Approve when the
                numbers are right.
              </span>
            </>
          )}
        </div>
      )}
      {period.status === "APPROVED" && (
        <ApprovedActions
          label={label}
          busy={busy}
          onPost={() => run(() => postPayroll(id))}
          onSendBack={(reason) => run(() => sendBackPayroll(id, reason))}
        />
      )}
      {period.status === "POSTED" && (
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          Posted. These payslips are final, staff can see them, and attendance for these days is
          locked.
        </p>
      )}

      {period.status === "DRAFT" ? (
        <p className="text-zinc-500">Not computed yet.</p>
      ) : (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Employee</th>
              {MONEY_COLUMNS.map((c) => (
                <th key={c.key} className="py-2 text-right font-medium">
                  {c.label}
                </th>
              ))}
              <th className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr>
                <td colSpan={7} className="py-3 text-zinc-500">
                  No payslips. Check that employees have a pay rate and a schedule.
                </td>
              </tr>
            )}
            {rows.map((r) => (
              <RegisterLine
                key={r.payslipId}
                row={r}
                open={open === r.payslipId}
                onToggle={() => setOpen(open === r.payslipId ? null : r.payslipId)}
              />
            ))}
          </tbody>
          {rows.length > 0 && (
            <tfoot className="border-t border-zinc-300 font-medium dark:border-zinc-700">
              <tr>
                <td className="py-2">Total ({rows.length})</td>
                {MONEY_COLUMNS.map((c) => (
                  <td key={c.key} className="py-2 text-right tabular-nums">
                    {formatPesos(total(c.key))}
                  </td>
                ))}
                <td />
              </tr>
            </tfoot>
          )}
        </table>
      )}

      {skipped.length > 0 && (
        <section
          aria-label="Not in this payroll"
          className="space-y-1 rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200"
        >
          <p className="font-medium">Not in this payroll</p>
          <ul className="list-disc pl-5">
            {skipped.map((s) => (
              <li key={s.employeeNo}>
                {s.employeeName} ({s.employeeNo}): {s.reason}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

function ApprovedActions({
  label,
  busy,
  onPost,
  onSendBack,
}: {
  label: string;
  busy: boolean;
  onPost: () => Promise<void>;
  onSendBack: (reason: string) => Promise<void>;
}) {
  const [step, setStep] = useState<"idle" | "post" | "back">("idle");
  const [reasonError, setReasonError] = useState<string>();

  async function sendBack(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const reason = String(new FormData(e.currentTarget).get("reason") ?? "").trim();
    if (reason.length < 3 || reason.length > 200) {
      setReasonError("Say why, in 3 to 200 characters");
      return;
    }
    await onSendBack(reason);
  }

  if (step === "post") {
    return (
      <section
        aria-label="Post this payroll"
        className="space-y-3 rounded-md bg-amber-50 px-3 py-3 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200"
      >
        <p>
          Post the payroll for {label}? A backup is taken first. Then attendance for these days is
          locked, loan balances go down, and staff can see their payslips. Posting can't be undone.
        </p>
        <div className="flex gap-2">
          <button type="button" className={primaryButton} disabled={busy} onClick={onPost}>
            Yes, post it
          </button>
          <button type="button" className={quietButton} onClick={() => setStep("idle")}>
            Not yet
          </button>
        </div>
      </section>
    );
  }
  if (step === "back") {
    return (
      <form onSubmit={sendBack} noValidate aria-label="Send back" className="max-w-md space-y-3">
        <Field name="reason" label="Reason" autoFocus error={reasonError} />
        <div className="flex gap-2">
          <button type="submit" className={primaryButton} disabled={busy}>
            Send back
          </button>
          <button type="button" className={quietButton} onClick={() => setStep("idle")}>
            Cancel
          </button>
        </div>
      </form>
    );
  }
  return (
    <div className="flex flex-wrap items-center gap-3">
      <button type="button" className={primaryButton} onClick={() => setStep("post")}>
        Post payroll
      </button>
      <button type="button" className={quietButton} onClick={() => setStep("back")}>
        Send back
      </button>
      <span className="text-sm text-zinc-600 dark:text-zinc-400">
        Approved. Send it back if something needs fixing before it's posted.
      </span>
    </div>
  );
}

function RegisterLine({
  row,
  open,
  onToggle,
}: {
  row: RegisterRow;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <>
      <tr className="border-b border-zinc-100 align-top dark:border-zinc-900">
        <td className="py-2">
          <div className="font-medium">{row.employeeName}</div>
          <div className="text-xs text-zinc-500">{row.employeeNo}</div>
          {row.warnings.map((w) => (
            <div key={w} className="text-xs text-amber-700 dark:text-amber-300">
              {w}
            </div>
          ))}
        </td>
        {MONEY_COLUMNS.map((c) => (
          <td key={c.key} className="py-2 text-right tabular-nums">
            {formatPesos(row[c.key] as number)}
          </td>
        ))}
        <td className="py-2 text-right">
          <button
            type="button"
            className={quietButton}
            aria-expanded={open}
            aria-label={`${open ? "Hide" : "View"} payslip for ${row.employeeName}`}
            onClick={onToggle}
          >
            {open ? "Hide" : "View"}
          </button>
        </td>
      </tr>
      {open && (
        <tr>
          <td colSpan={7} className="pb-4">
            <Payslip id={row.payslipId} name={row.employeeName} />
          </td>
        </tr>
      )}
    </>
  );
}

function Payslip({ id, name }: { id: number; name: string }) {
  const [slip, setSlip] = useState<PayslipDetail | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    let current = true;
    getPayslip(id)
      .then((s) => current && setSlip(s))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, [id]);

  if (!slip) return <FormAlert message={alert} />;
  return <PayslipView slip={slip} label={`Payslip for ${name}`} />;
}
