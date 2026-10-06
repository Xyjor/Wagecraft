import { useEffect, useState, type FormEvent } from "react";
import type { Compensation } from "@/bindings/Compensation";
import type { Employee } from "@/bindings/Employee";
import type { Position } from "@/bindings/Position";
import { Field, FormAlert, SelectField } from "@/components/form";
import { Badge, primaryButton } from "@/components/ui";
import { listPositions } from "@/features/org/api";
import { formatDate } from "@/lib/dates";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { addCompensation, compensationHistory } from "./api";
import {
  checkCompensation,
  COMPENSATION_FIELD,
  PAY_BASIS_LABELS,
  rangeWarning,
} from "./compensation";

type Loaded = { history: Compensation[]; position?: Position };

/** Salary history, newest first, with a form to add the next rate. */
export function CompensationTab({ employee }: { employee: Employee }) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    Promise.all([compensationHistory(employee.id), listPositions()])
      .then(([history, positions]) =>
        setLoaded({ history, position: positions.find((p) => p.id === employee.positionId) }),
      )
      .catch((e: AppError) => setAlert(e.message));
  }, [employee.id, employee.positionId]);

  if (!loaded) return <FormAlert message={alert} />;
  const { history, position } = loaded;
  const current = history[0];
  const warning = current && rangeWarning(current, position);

  return (
    <div className="space-y-6">
      {history.length === 0 ? (
        <p className="text-sm text-zinc-500">
          No pay rate yet. Add one before this employee's first payroll.
        </p>
      ) : (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Starts</th>
              <th className="py-2 font-medium">Ends</th>
              <th className="py-2 font-medium">Basis</th>
              <th className="py-2 text-right font-medium">Rate</th>
              <th className="py-2 pl-6 font-medium">Reason</th>
              <th className="py-2 font-medium">Changed by</th>
            </tr>
          </thead>
          <tbody>
            {history.map((c) => (
              <tr key={c.id} className="border-b border-zinc-100 dark:border-zinc-900">
                <td className="py-2">{formatDate(c.effectiveFrom)}</td>
                <td className="py-2">
                  {c.effectiveTo ? formatDate(c.effectiveTo) : <Badge tone="good">Current</Badge>}
                </td>
                <td className="py-2">
                  {PAY_BASIS_LABELS[c.payBasis] ?? c.payBasis}
                  {c.minimumWageEarner && (
                    <div className="pt-0.5">
                      <Badge tone="muted">Minimum wage earner</Badge>
                    </div>
                  )}
                </td>
                <td className="py-2 text-right tabular-nums">{formatPesos(c.rateCents)}</td>
                <td className="py-2 pl-6">{c.reason}</td>
                <td className="py-2">{c.createdByName}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {warning && <p className="text-sm text-amber-700 dark:text-amber-300">{warning}</p>}

      {employee.archivedAt === null && (
        <AddRate
          key={history.length}
          employee={employee}
          current={current}
          onAdded={async () =>
            setLoaded({ history: await compensationHistory(employee.id), position })
          }
        />
      )}
    </div>
  );
}

function AddRate({
  employee,
  current,
  onAdded,
}: {
  employee: Employee;
  current?: Compensation;
  onAdded: () => Promise<void>;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const checked = checkCompensation(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      await addCompensation(employee.id, checked.value);
      await onAdded();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(
        Object.fromEntries(
          Object.entries(s.fields).map(([k, v]) => [COMPENSATION_FIELD[k] ?? k, v]),
        ),
      );
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate className="space-y-3">
      <h2 className="text-lg font-semibold">{current ? "Change pay" : "Set pay"}</h2>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        A rate starts on the 1st or 16th, the first day of a pay period.
        {current &&
          " The current rate ends the day before, and past payslips keep the rate they used."}
      </p>
      <FormAlert message={alert} />
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField
          name="payBasis"
          label="Pay basis"
          options={Object.entries(PAY_BASIS_LABELS).map(([value, label]) => ({ value, label }))}
          defaultValue={current?.payBasis ?? "MONTHLY"}
          error={errors.payBasis}
        />
        <Field name="rate" label="Rate (₱)" error={errors.rate} />
        <Field name="effectiveFrom" label="Starts on" type="date" error={errors.effectiveFrom} />
        <Field name="reason" label="Reason" error={errors.reason} />
        <label className="flex items-start gap-2 text-sm sm:col-span-2">
          <input
            type="checkbox"
            name="minimumWageEarner"
            defaultChecked={current?.minimumWageEarner ?? false}
            className="mt-0.5"
          />
          <span>
            Minimum wage earner
            <span className="block text-zinc-600 dark:text-zinc-400">
              No tax is withheld while this rate applies. Untick it when a raise takes them above
              the minimum wage.
            </span>
          </span>
        </label>
      </div>
      <button type="submit" disabled={busy} className={primaryButton}>
        Save rate
      </button>
    </form>
  );
}
