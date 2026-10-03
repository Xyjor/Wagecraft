import { useState, type FormEvent } from "react";
import type { LeaveType } from "@/bindings/LeaveType";
import { Field, FormAlert, SelectField } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import { todayIso } from "@/features/attendance/format";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { fileLeave } from "./api";
import { checkLeaveRequest } from "./validation";

/** Files leave for yourself, or, with `forSomeone`, HR filing for an employee. */
export function LeaveRequestForm({
  types,
  forSomeone = false,
  onSaved,
  onCancel,
}: {
  types: LeaveType[];
  forSomeone?: boolean;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);
  const options = types
    .filter((t) => t.isActive)
    .map((t) => ({ value: String(t.id), label: t.isPaid ? t.name : `${t.name} (unpaid)` }));

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkLeaveRequest(formValues(e.currentTarget), forSomeone);
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      await fileLeave(checked.value);
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
      aria-label="File leave"
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      {forSomeone && (
        <Field name="employeeNo" label="Employee number" autoFocus error={errors.employeeNo} />
      )}
      <SelectField
        name="leaveTypeId"
        label="Leave type"
        options={options}
        defaultValue={options[0]?.value}
        error={errors.leaveTypeId}
      />
      <div className="grid grid-cols-2 gap-4">
        <Field
          name="startDate"
          label="First day"
          type="date"
          defaultValue={todayIso()}
          error={errors.startDate}
        />
        <Field name="endDate" label="Last day" type="date" error={errors.endDate} />
      </div>
      <div>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            name="halfDay"
            aria-invalid={errors.halfDay ? true : undefined}
            aria-describedby={errors.halfDay ? "halfDay-error" : undefined}
          />
          Half day only
        </label>
        {errors.halfDay && (
          <p id="halfDay-error" className="mt-1 text-sm text-red-600 dark:text-red-400">
            {errors.halfDay}
          </p>
        )}
      </div>
      <Field name="reason" label="What it's for" error={errors.reason} />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Leave the last day empty for a one-day leave. Only scheduled work days count, so weekends
        and holidays off are free. Paid leave comes out of the balance once HR approves it.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy || options.length === 0} className={primaryButton}>
          File leave
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
