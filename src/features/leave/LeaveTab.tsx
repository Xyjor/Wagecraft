import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { Employee } from "@/bindings/Employee";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import { Field, FormAlert } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { adjustBalance, balancesFor } from "./api";
import { BalanceTable } from "./BalanceTable";
import { formatDays } from "./format";
import { checkAdjustment } from "./validation";

/** HR's view of one employee's leave balances, a year at a time, with adjustments. */
export function LeaveTab({
  employee,
  initialYear = new Date().getFullYear(),
}: {
  employee: Employee;
  initialYear?: number;
}) {
  const [year, setYear] = useState(initialYear);
  const [balances, setBalances] = useState<LeaveBalance[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [adjusting, setAdjusting] = useState<LeaveBalance | null>(null);

  const reload = useCallback(async () => {
    setBalances(await balancesFor(employee.id, year));
  }, [employee.id, year]);

  useEffect(() => {
    let live = true;
    balancesFor(employee.id, year)
      .then((b) => live && setBalances(b))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [employee.id, year]);

  function changeYear(by: number) {
    setAdjusting(null);
    setAlert(undefined);
    setBalances(null);
    setYear((y) => y + by);
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-1">
        <button
          type="button"
          className={quietButton}
          aria-label="Previous year"
          onClick={() => changeYear(-1)}
        >
          ←
        </button>
        <span className="w-14 text-center font-medium tabular-nums" aria-live="polite">
          {year}
        </span>
        <button
          type="button"
          className={quietButton}
          aria-label="Next year"
          onClick={() => changeYear(1)}
        >
          →
        </button>
      </div>
      <FormAlert message={alert} />
      {adjusting && (
        <AdjustForm
          key={adjusting.id}
          balance={adjusting}
          onCancel={() => setAdjusting(null)}
          onSaved={async () => {
            setAdjusting(null);
            await reload();
          }}
        />
      )}
      {balances && (
        <BalanceTable
          balances={balances}
          empty={`No leave balances for ${year}. They're added when HR grants the year's leave.`}
          actions={(b) => (
            <div className="flex justify-end">
              <button
                type="button"
                className={quietButton}
                aria-label={`Adjust ${b.leaveTypeName}`}
                onClick={() => setAdjusting(b)}
              >
                Adjust
              </button>
            </div>
          )}
        />
      )}
    </div>
  );
}

function AdjustForm({
  balance,
  onSaved,
  onCancel,
}: {
  balance: LeaveBalance;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkAdjustment(formValues(e.currentTarget), balance.usedHalfdays);
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      await adjustBalance(balance.id, checked.value);
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      noValidate
      aria-label={`Adjust ${balance.leaveTypeName}`}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <Field
        name="days"
        label={`${balance.leaveTypeName} days for ${balance.year}`}
        autoFocus
        defaultValue={String(balance.entitledHalfdays / 2)}
        error={errors.days}
      />
      <Field name="reason" label="Reason for the change" error={errors.reason} />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        {formatDays(balance.usedHalfdays)} already used. Use half days like 2.5 if needed.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          Save balance
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
