import { useEffect, useState, type FormEvent } from "react";
import { useParams } from "react-router";
import type { PayrollRegister } from "@/bindings/PayrollRegister";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { RegisterRow } from "@/bindings/RegisterRow";
import { Field, FormAlert } from "@/components/form";
import { Money } from "@/components/Money";
import { Notice, RoleNote } from "@/components/Notice";
import { PageHeader } from "@/components/PageHeader";
import { EmptyState, Loading } from "@/components/states";
import * as T from "@/components/table";
import { primaryButton, quietButton, secondaryButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { todayIso } from "@/features/attendance/format";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import {
  approvePayroll,
  computePayroll,
  getPayslip,
  getRegister,
  postPayroll,
  sendBackPayroll,
} from "./api";
import { PayslipView } from "./PayslipView";
import { RegisterDownloads } from "./RegisterDownloads";
import { PayrollSteps } from "./PayrollSteps";
import { cutoffStillRunning, periodLabel } from "./periods";

export function RegisterPage({ today = todayIso() }: { today?: string }) {
  const { me } = useSession();
  const id = Number(useParams().id);
  if (me.role === "STAFF") {
    return <RoleNote>Only Admin and HR can run payroll.</RoleNote>;
  }
  return <Register key={id} id={id} today={today} />;
}

// Deductions are written in red ink, as on a paper register.
// Net pay is set off by a double rule, the way a columnar pad sets off its totals column.
const MONEY_COLUMNS: { key: keyof RegisterRow; label: string; ink?: string; rule: string }[] = [
  { key: "grossCents", label: "Gross", rule: T.num },
  { key: "statutoryEeCents", label: "Contributions", ink: "text-danger", rule: T.num },
  { key: "taxCents", label: "Tax", ink: "text-danger", rule: T.num },
  {
    key: "otherDeductionsCents",
    label: "Other deductions",
    ink: "text-danger",
    rule: T.num,
  },
  {
    key: "netCents",
    label: "Net pay",
    ink: "font-semibold",
    rule: `${T.totalCol} text-right tabular-nums`,
  },
];

/** Zeros stay faint so the red ink only marks real deductions. */
function inkFor(c: (typeof MONEY_COLUMNS)[number], cents: number) {
  return cents === 0 ? "text-ink-faint" : (c.ink ?? "");
}

function Register({ id, today }: { id: number; today: string }) {
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

  if (!register)
    return alert ? <FormAlert message={alert} /> : <Loading label="Loading the payroll…" />;
  const { period, rows, skipped } = register;
  const label = periodLabel(period.periodStart, period.periodEnd);
  const canCompute = period.status === "DRAFT" || period.status === "COMPUTED";
  const total = (key: keyof RegisterRow) => rows.reduce((sum, r) => sum + (r[key] as number), 0);

  return (
    <div className="space-y-6">
      <PageHeader
        title={`Payroll for ${label}`}
        crumbs={[{ label: "Payroll", to: "/payroll" }]}
        description={`Pay date ${formatDate(period.payDate)}. Rule pack ${period.rulePackCode}.`}
        actions={
          period.status !== "DRAFT" && rows.length > 0 ? (
            <RegisterDownloads periodId={id} />
          ) : undefined
        }
      />
      <PayrollSteps status={period.status}>
        <FormAlert message={alert} />
        {canCompute && (
          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              className={period.status === "DRAFT" ? primaryButton : secondaryButton}
              disabled={busy}
              onClick={() => run(() => computePayroll(id))}
            >
              {period.status === "DRAFT" ? "Compute payroll" : "Recompute"}
            </button>
            {period.status === "COMPUTED" && (
              <button
                type="button"
                className={primaryButton}
                disabled={busy || rows.length === 0}
                onClick={() => run(() => approvePayroll(id))}
              >
                Approve
              </button>
            )}
          </div>
        )}
        {canCompute && cutoffStillRunning(period.periodEnd, today) && (
          <p className="text-sm text-warn">
            The cutoff runs until {formatDate(period.periodEnd)}. A payroll computed now only has
            the attendance recorded so far, so recompute after that day.
          </p>
        )}
        {period.status === "APPROVED" && (
          <ApprovedActions
            label={label}
            busy={busy}
            onPost={() => run(() => postPayroll(id))}
            onSendBack={(reason) => run(() => sendBackPayroll(id, reason))}
          />
        )}
      </PayrollSteps>

      {period.status === "DRAFT" ? (
        <EmptyState message="Not computed yet." />
      ) : (
        <table className={T.table}>
          <thead className={T.thead}>
            <tr>
              <th className={T.th}>Employee</th>
              {MONEY_COLUMNS.map((c) => (
                <th key={c.key} className={`${T.th} ${c.rule}`}>
                  {c.label}
                </th>
              ))}
              <th className={T.th}>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <T.EmptyRow colSpan={7}>
                No payslips. Check that employees have a pay rate and a schedule.
              </T.EmptyRow>
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
            <tfoot>
              <tr className={T.totalRow}>
                <td className={T.td}>Total ({rows.length})</td>
                {MONEY_COLUMNS.map((c) => (
                  <td key={c.key} className={`${T.td} ${c.rule} ${inkFor(c, total(c.key))}`}>
                    <Money cents={total(c.key)} />
                  </td>
                ))}
                <td />
              </tr>
            </tfoot>
          )}
        </table>
      )}

      {skipped.length > 0 && (
        <Notice tone="warn" label="Not in this payroll">
          <p className="font-medium">Not in this payroll</p>
          <ul className="list-disc pl-5">
            {skipped.map((s) => (
              <li key={s.employeeNo}>
                {s.employeeName} ({s.employeeNo}): {s.reason}
              </li>
            ))}
          </ul>
        </Notice>
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
      <Notice tone="warn" label="Post this payroll">
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
      </Notice>
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
      <button type="button" className={secondaryButton} onClick={() => setStep("back")}>
        Send back
      </button>
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
      <tr className={`${T.row} align-top`}>
        <td className={T.td}>
          <div className="font-medium">{row.employeeName}</div>
          <div className="text-xs text-ink-faint">{row.employeeNo}</div>
          {row.warnings.map((w) => (
            <div key={w} className="text-xs text-warn">
              {w}
            </div>
          ))}
        </td>
        {MONEY_COLUMNS.map((c) => (
          <td key={c.key} className={`${T.td} ${c.rule} ${inkFor(c, row[c.key] as number)}`}>
            <Money cents={row[c.key] as number} />
          </td>
        ))}
        <td className={`${T.td} text-right`}>
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
          <td colSpan={7} className="bg-sunken/50 px-3 pt-1 pb-4">
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
  return <PayslipView slip={slip} label={`Payslip for ${name}`} inline />;
}
