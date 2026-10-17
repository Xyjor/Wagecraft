import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { PendingItem } from "@/bindings/PendingItem";
import type { PeriodChecks } from "@/bindings/PeriodChecks";
import type { RulePackSummary } from "@/bindings/RulePackSummary";
import { Field, FormAlert, SelectField } from "@/components/form";
import { formCardClass } from "@/components/Card";
import { Notice, RoleNote } from "@/components/Notice";
import { PageHeader } from "@/components/PageHeader";
import * as T from "@/components/table";
import { Badge, primaryButton, quietButton, secondaryButton } from "@/components/ui";
import { todayIso } from "@/features/attendance/format";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import { serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { createPeriod, deletePeriod, listPeriods, listRulePacks, periodChecks } from "./api";
import {
  cutoffOf,
  periodEnd,
  periodLabel,
  periodStart,
  STATUS_LABELS,
  STATUS_TONES,
  type Cutoff,
} from "./periods";

export function PayrollPage({ today }: { today?: string }) {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return <RoleNote>Only Admin and HR can run payroll.</RoleNote>;
  }
  return <Payroll today={today ?? todayIso()} />;
}

function Payroll({ today }: { today: string }) {
  const [periods, setPeriods] = useState<PayrollPeriod[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [creating, setCreating] = useState(false);
  const [confirming, setConfirming] = useState<number | null>(null);

  const reload = useCallback(async () => {
    setPeriods(await listPeriods());
  }, []);

  useEffect(() => {
    let current = true;
    listPeriods()
      .then((p) => current && setPeriods(p))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, []);

  async function remove(p: PayrollPeriod) {
    setAlert(undefined);
    try {
      await deletePeriod(p.id);
      setConfirming(null);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Payroll"
        description="Each pay period is one semi-monthly cutoff: the 1st to the 15th, or the 16th to the end of the month. Create the period, then compute, approve and post it."
        actions={
          <Link to="/payroll/rules" className={secondaryButton}>
            Rule packs
          </Link>
        }
      />
      <FormAlert message={alert} />

      {creating ? (
        <PeriodForm
          today={today}
          onCancel={() => setCreating(false)}
          onSaved={async () => {
            setCreating(false);
            await reload();
          }}
        />
      ) : (
        <button type="button" className={primaryButton} onClick={() => setCreating(true)}>
          New pay period
        </button>
      )}

      {periods && (
        <table className={T.table}>
          <thead className={T.thead}>
            <tr>
              <th className={T.th}>Period</th>
              <th className={T.th}>Pay date</th>
              <th className={T.th}>Rule pack</th>
              <th className={T.th}>Status</th>
              <th className={T.th}>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {periods.length === 0 && (
              <T.EmptyRow colSpan={5}>
                No pay periods yet. Create the first one with New pay period.
              </T.EmptyRow>
            )}
            {periods.map((p) => {
              const label = periodLabel(p.periodStart, p.periodEnd);
              return (
                <tr key={p.id} className={T.row}>
                  <td className={`${T.td} font-medium tabular-nums`}>
                    <Link
                      to={`/payroll/${p.id}`}
                      className="text-accent underline-offset-2 hover:underline"
                    >
                      {label}
                    </Link>
                  </td>
                  <td className={`${T.td} tabular-nums`}>{formatDate(p.payDate)}</td>
                  <td className={T.td}>{p.rulePackCode}</td>
                  <td className={T.td}>
                    <Badge tone={STATUS_TONES[p.status]}>{STATUS_LABELS[p.status]}</Badge>
                  </td>
                  <td className={T.td}>
                    <div className="flex items-center justify-end gap-1">
                      {p.status !== "DRAFT" ? null : confirming === p.id ? (
                        <>
                          <span>Delete {label}?</span>
                          <button type="button" className={quietButton} onClick={() => remove(p)}>
                            Yes, delete
                          </button>
                          <button
                            type="button"
                            className={quietButton}
                            onClick={() => setConfirming(null)}
                          >
                            Keep
                          </button>
                        </>
                      ) : (
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`Delete ${label}`}
                          onClick={() => setConfirming(p.id)}
                        >
                          Delete
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

const CUTOFF_OPTIONS = [
  { value: "1", label: "1st to 15th" },
  { value: "2", label: "16th to end of month" },
];

/** The pack in effect for the whole period, if there is one. */
function packFor(packs: RulePackSummary[], start: string): RulePackSummary | undefined {
  const end = periodEnd(start);
  return packs.find((p) => p.effectiveFrom <= start && (!p.effectiveTo || p.effectiveTo >= end));
}

function PeriodForm({
  today,
  onSaved,
  onCancel,
}: {
  today: string;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const initial = cutoffOf(today);
  const [month, setMonth] = useState(initial.month);
  const [cutoff, setCutoff] = useState<Cutoff>(initial.cutoff);
  const [packs, setPacks] = useState<RulePackSummary[] | null>(null);
  const [checks, setChecks] = useState<PeriodChecks | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);
  const start = /^\d{4}-\d{2}$/.test(month) ? periodStart(month, cutoff) : null;

  useEffect(() => {
    let current = true;
    listRulePacks()
      .then((p) => current && setPacks(p))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, []);

  useEffect(() => {
    if (!start) return;
    let current = true;
    periodChecks(start)
      .then((c) => current && setChecks(c))
      .catch(() => current && setChecks(null));
    return () => {
      current = false;
    };
  }, [start]);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    if (!start) {
      setErrors({ periodStart: "Pick a month" });
      return;
    }
    const data = new FormData(e.currentTarget);
    setBusy(true);
    try {
      await createPeriod({
        periodStart: start,
        payDate: String(data.get("payDate") ?? ""),
        rulePackId: Number(data.get("rulePackId")),
      });
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate aria-label="New pay period" className={formCardClass}>
      <FormAlert message={alert} />
      <Field
        name="month"
        label="Month"
        type="month"
        autoFocus
        defaultValue={month}
        onChange={setMonth}
        error={errors.periodStart}
      />
      <SelectField
        name="cutoff"
        label="Cutoff"
        options={CUTOFF_OPTIONS}
        defaultValue={String(cutoff)}
        onChange={(v) => setCutoff(v === "1" ? 1 : 2)}
      />
      <Field
        key={`pay-${start}`}
        name="payDate"
        label="Pay date"
        type="date"
        defaultValue={start ? periodEnd(start) : ""}
        error={errors.payDate}
      />
      {packs && (
        <SelectField
          key={`pack-${start}`}
          name="rulePackId"
          label="Rule pack"
          options={packs.map((p) => ({ value: String(p.id), label: `${p.code} · ${p.name}` }))}
          defaultValue={String((start && packFor(packs, start)?.id) ?? packs[0]?.id ?? "")}
          error={errors.rulePackId}
        />
      )}
      {checks && <Warnings checks={checks} />}
      <div className="flex gap-2">
        <button type="submit" disabled={busy || !packs} className={primaryButton}>
          Create pay period
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function plural(n: number, one: string, many: string) {
  return `${n} ${n === 1 ? one : many}`;
}

function Warnings({ checks }: { checks: PeriodChecks }) {
  const groups = [
    {
      title: plural(checks.pendingLeave.length, "pending leave request", "pending leave requests"),
      items: checks.pendingLeave,
      from: true,
    },
    {
      title: plural(
        checks.pendingOvertime.length,
        "pending overtime request",
        "pending overtime requests",
      ),
      items: checks.pendingOvertime,
      from: false,
    },
    {
      title: plural(checks.missingTimeOuts.length, "missing time-out", "missing time-outs"),
      items: checks.missingTimeOuts,
      from: false,
    },
  ].filter((g) => g.items.length > 0);

  return (
    <Notice label="Unfinished work in this period" tone={groups.length === 0 ? "info" : "warn"}>
      {groups.length === 0 ? (
        <p>No pending leave, overtime or missing time-outs in this period.</p>
      ) : (
        <>
          <p>
            These would change the payslips. You can still create the period, but sort them out
            before you compute.
          </p>
          {groups.map((g) => (
            <div key={g.title}>
              <p className="font-medium">{g.title}</p>
              <ul className="list-disc pl-5">
                {g.items.map((i: PendingItem) => (
                  <li key={`${i.employeeNo}-${i.date}`}>
                    {`${i.employeeName} (${i.employeeNo}), ${g.from ? "from " : ""}${formatDate(i.date)}`}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </>
      )}
    </Notice>
  );
}
