import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { LeaveType } from "@/bindings/LeaveType";
import { Field, FormAlert } from "@/components/form";
import { ActiveBadge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import {
  createLeaveType,
  grantLeave,
  listLeaveTypes,
  setLeaveTypeActive,
  updateLeaveType,
} from "./api";
import { formatDays } from "./format";
import { LeaveRequests } from "./LeaveRequests";
import { checkLeaveType } from "./validation";

/** HR's leave screen (`/leave`): requests to decide, the yearly grant and the leave types. */
export function LeavePage({ thisYear = new Date().getFullYear() }: { thisYear?: number }) {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Only Admin and HR can manage leave. Your own balances are under My leave.
      </p>
    );
  }
  return <Leave thisYear={thisYear} isAdmin={me.role === "ADMIN"} myEmployeeId={me.employeeId} />;
}

function Leave({
  thisYear,
  isAdmin,
  myEmployeeId,
}: {
  thisYear: number;
  isAdmin: boolean;
  myEmployeeId: number | null;
}) {
  const [types, setTypes] = useState<LeaveType[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [granted, setGranted] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [form, setForm] = useState<null | "new" | LeaveType>(null);

  const reload = useCallback(async () => setTypes(await listLeaveTypes()), []);

  useEffect(() => {
    let live = true;
    listLeaveTypes()
      .then((t) => live && setTypes(t))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, []);

  async function grant(year: number) {
    setAlert(undefined);
    setGranted(undefined);
    setBusy(true);
    try {
      const added = await grantLeave(year);
      setGranted(
        added === 0
          ? `Everyone already had their ${year} balances. Nothing was added.`
          : `Added ${added} leave ${added === 1 ? "balance" : "balances"} for ${year}.`,
      );
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setBusy(false);
    }
  }

  async function toggle(t: LeaveType) {
    setAlert(undefined);
    try {
      await setLeaveTypeActive(t.id, !t.isActive);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <div className="space-y-10">
      <h1 className="text-2xl font-semibold tracking-tight">Leave</h1>
      <FormAlert message={alert} />

      <LeaveRequests types={types ?? []} myEmployeeId={myEmployeeId} />

      <section className="space-y-3" aria-labelledby="grant-heading">
        <h2 id="grant-heading" className="text-lg font-semibold">
          Yearly balances
        </h2>
        <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
          Gives every current employee their leave for the year. It only adds what&apos;s missing,
          so it&apos;s safe to run again, for example to give SIL to someone who just reached a year
          of service. An employee&apos;s balances are also added the first time anyone looks at them
          in a new year.
        </p>
        <div className="flex gap-2">
          <button
            type="button"
            className={primaryButton}
            disabled={busy}
            onClick={() => grant(thisYear)}
          >
            Grant {String(thisYear)} leave
          </button>
          <button
            type="button"
            className={quietButton}
            disabled={busy}
            onClick={() => grant(thisYear + 1)}
          >
            Grant {String(thisYear + 1)} leave
          </button>
        </div>
        {granted && (
          <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
            {granted}
          </p>
        )}
      </section>

      <section className="space-y-4" aria-labelledby="types-heading">
        <div className="flex items-center justify-between">
          <h2 id="types-heading" className="text-lg font-semibold">
            Leave types
          </h2>
          {isAdmin && form === null && (
            <button type="button" className={primaryButton} onClick={() => setForm("new")}>
              Add leave type
            </button>
          )}
        </div>
        {form !== null && (
          <LeaveTypeForm
            key={form === "new" ? "new" : form.id}
            editing={form === "new" ? null : form}
            onCancel={() => setForm(null)}
            onSaved={async () => {
              setForm(null);
              await reload();
            }}
          />
        )}
        {types && (
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <th className="py-2 font-medium">Code</th>
                <th className="py-2 font-medium">Name</th>
                <th className="py-2 font-medium">Pay</th>
                <th className="py-2 font-medium">Each year</th>
                <th className="py-2 font-medium">After</th>
                <th className="py-2 font-medium">Status</th>
                {isAdmin && (
                  <th className="py-2 font-medium">
                    <span className="sr-only">Actions</span>
                  </th>
                )}
              </tr>
            </thead>
            <tbody>
              {types.map((t) => (
                <tr key={t.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2 font-medium">{t.code}</td>
                  <td className="py-2">{t.name}</td>
                  <td className="py-2">{t.isPaid ? "Paid" : "Unpaid"}</td>
                  <td className="py-2 tabular-nums">
                    {t.isPaid ? formatDays(t.defaultHalfdaysPerYear) : "No limit"}
                  </td>
                  <td className="py-2">
                    {t.minServiceMonths === 0 ? "Hiring" : `${t.minServiceMonths} months`}
                  </td>
                  <td className="py-2">
                    <ActiveBadge active={t.isActive} />
                  </td>
                  {isAdmin && (
                    <td className="py-2">
                      <div className="flex justify-end gap-1">
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`Edit ${t.name}`}
                          onClick={() => setForm(t)}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`${t.isActive ? "Deactivate" : "Activate"} ${t.name}`}
                          onClick={() => toggle(t)}
                        >
                          {t.isActive ? "Deactivate" : "Activate"}
                        </button>
                      </div>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {!isAdmin && <p className="text-sm text-zinc-500">Only Admin can change leave types.</p>}
      </section>
    </div>
  );
}

function LeaveTypeForm({
  editing,
  onSaved,
  onCancel,
}: {
  editing: LeaveType | null;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkLeaveType(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      if (editing) await updateLeaveType(editing.id, checked.value);
      else await createLeaveType(checked.value);
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
      aria-label={editing ? `Edit ${editing.name}` : "Add leave type"}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <div className="grid grid-cols-3 gap-4">
        <Field
          name="code"
          label="Code"
          autoFocus
          defaultValue={editing?.code}
          error={errors.code}
        />
        <div className="col-span-2">
          <Field name="name" label="Name" defaultValue={editing?.name} error={errors.name} />
        </div>
      </div>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" name="isPaid" defaultChecked={editing?.isPaid ?? true} />
        Paid leave
      </label>
      <div className="grid grid-cols-2 gap-4">
        <Field
          name="defaultDays"
          label="Days each year"
          defaultValue={String((editing?.defaultHalfdaysPerYear ?? 0) / 2)}
          error={errors.defaultDays}
        />
        <Field
          name="minServiceMonths"
          label="Months of service first"
          type="number"
          defaultValue={String(editing?.minServiceMonths ?? 0)}
          error={errors.minServiceMonths}
        />
      </div>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Changes apply to balances granted from now on. Balances already given keep their days; HR
        can adjust one from the employee&apos;s Leave tab.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          Save leave type
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
