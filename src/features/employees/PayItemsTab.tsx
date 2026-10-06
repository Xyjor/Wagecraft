import { useEffect, useState, type FormEvent } from "react";
import type { Employee } from "@/bindings/Employee";
import type { RecurringItem } from "@/bindings/RecurringItem";
import { Field, FormAlert, SelectField } from "@/components/form";
import { Badge, primaryButton, quietButton } from "@/components/ui";
import { formatDate } from "@/lib/dates";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { addRecurringItem, deleteRecurringItem, recurringItems, updateRecurringItem } from "./api";
import { checkPayItem, KIND_LABELS, PAY_ITEM_FIELD, SCHEDULE_LABELS } from "./payItems";

const options = (labels: Record<string, string>) =>
  Object.entries(labels).map(([value, label]) => ({ value, label }));

/** 1200050 → "12000.50", for editing an amount. */
const amountText = (cents: number) =>
  `${Math.trunc(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;

function runs(i: RecurringItem) {
  return i.endDate
    ? `${formatDate(i.startDate)} to ${formatDate(i.endDate)}`
    : `From ${formatDate(i.startDate)}`;
}

/** Recurring allowances, loans and other deductions that payroll adds each cutoff. */
export function PayItemsTab({ employee }: { employee: Employee }) {
  const [items, setItems] = useState<RecurringItem[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [editing, setEditing] = useState<RecurringItem | null>(null);
  const [confirming, setConfirming] = useState<number | null>(null);
  const editable = employee.archivedAt === null;

  useEffect(() => {
    let current = true;
    recurringItems(employee.id)
      .then((i) => current && setItems(i))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, [employee.id]);

  async function reload() {
    setItems(await recurringItems(employee.id));
  }

  async function remove(i: RecurringItem) {
    setAlert(undefined);
    try {
      await deleteRecurringItem(i.id);
      setConfirming(null);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  if (!items) return <FormAlert message={alert} />;

  return (
    <div className="space-y-6">
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        Payroll adds these on every cutoff they apply to. Deductions come off after contributions
        and tax: loans first, oldest first. If net pay can't cover one, the rest carries over to the
        next pay period.
      </p>
      <FormAlert message={alert} />
      {items.length === 0 ? (
        <p className="text-sm text-zinc-500">No allowances or deductions yet.</p>
      ) : (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Item</th>
              <th className="py-2 font-medium">Cutoffs</th>
              <th className="py-2 text-right font-medium">Per cutoff</th>
              <th className="py-2 text-right font-medium">Still owed</th>
              <th className="py-2 pl-6 font-medium">Runs</th>
              <th className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {items.map((i) => (
              <tr key={i.id} className="border-b border-zinc-100 align-top dark:border-zinc-900">
                <td className="py-2">
                  <div className="font-medium">{i.label}</div>
                  <div className="flex gap-1 pt-0.5">
                    <Badge tone="muted">{KIND_LABELS[i.kind] ?? i.kind}</Badge>
                    {i.taxable && <Badge tone="muted">Taxable</Badge>}
                  </div>
                </td>
                <td className="py-2">{SCHEDULE_LABELS[i.schedule] ?? i.schedule}</td>
                <td className="py-2 text-right tabular-nums">{formatPesos(i.amountCents)}</td>
                <td className="py-2 text-right tabular-nums">
                  {i.remainingBalanceCents === null ? "" : formatPesos(i.remainingBalanceCents)}
                </td>
                <td className="py-2 pl-6">{runs(i)}</td>
                <td className="py-2">
                  {editable && (
                    <div className="flex items-center justify-end gap-1">
                      {confirming === i.id ? (
                        <>
                          <span>Remove {i.label}?</span>
                          <button type="button" className={quietButton} onClick={() => remove(i)}>
                            Yes, remove
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
                        <>
                          <button
                            type="button"
                            className={quietButton}
                            aria-label={`Edit ${i.label}`}
                            onClick={() => setEditing(i)}
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            className={quietButton}
                            aria-label={`Remove ${i.label}`}
                            onClick={() => setConfirming(i.id)}
                          >
                            Remove
                          </button>
                        </>
                      )}
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {editable && (
        <ItemForm
          key={editing ? `edit-${editing.id}` : `add-${items.length}`}
          employeeId={employee.id}
          editing={editing}
          onCancel={() => setEditing(null)}
          onSaved={async () => {
            setEditing(null);
            await reload();
          }}
        />
      )}
    </div>
  );
}

function ItemForm({
  employeeId,
  editing,
  onCancel,
  onSaved,
}: {
  employeeId: number;
  editing: RecurringItem | null;
  onCancel: () => void;
  onSaved: () => Promise<void>;
}) {
  const [kind, setKind] = useState(editing?.kind ?? "ALLOWANCE");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);
  const title = editing ? `Edit ${editing.label}` : "Add an allowance or deduction";

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkPayItem({ ...formValues(e.currentTarget), kind });
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      if (editing) await updateRecurringItem(editing.id, checked.value);
      else await addRecurringItem(employeeId, checked.value);
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(
        Object.fromEntries(Object.entries(s.fields).map(([k, v]) => [PAY_ITEM_FIELD[k] ?? k, v])),
      );
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate aria-label={title} className="space-y-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <FormAlert message={alert} />
      <div className="grid gap-4 sm:grid-cols-2">
        {!editing && (
          <SelectField
            name="kind"
            label="Kind"
            options={options(KIND_LABELS)}
            defaultValue={kind}
            onChange={setKind}
            error={errors.kind}
          />
        )}
        <Field name="label" label="Name" defaultValue={editing?.label} error={errors.label} />
        <Field
          name="amount"
          label="Amount per cutoff (₱)"
          defaultValue={editing ? amountText(editing.amountCents) : undefined}
          error={errors.amount}
        />
        <SelectField
          name="schedule"
          label="Cutoffs"
          options={options(SCHEDULE_LABELS)}
          defaultValue={editing?.schedule ?? "EVERY_CUTOFF"}
          error={errors.schedule}
        />
        <Field
          name="startDate"
          label="Starts on"
          type="date"
          defaultValue={editing?.startDate}
          error={errors.startDate}
        />
        <Field
          name="endDate"
          label="Ends on (optional)"
          type="date"
          defaultValue={editing?.endDate ?? undefined}
          error={errors.endDate}
        />
        {kind === "LOAN" && (
          <Field
            name="balance"
            label="Still owed (₱)"
            defaultValue={
              editing?.remainingBalanceCents != null
                ? amountText(editing.remainingBalanceCents)
                : undefined
            }
            error={errors.balance}
          />
        )}
        {kind === "ALLOWANCE" && (
          <label className="flex items-center gap-2 self-end pb-2 text-sm">
            <input type="checkbox" name="taxable" defaultChecked={editing?.taxable ?? false} />
            Taxable
          </label>
        )}
      </div>
      {errors.taxable && <p className="text-sm text-red-600 dark:text-red-400">{errors.taxable}</p>}
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          {editing ? "Save changes" : "Add item"}
        </button>
        {editing && (
          <button type="button" onClick={onCancel} className={quietButton}>
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}
