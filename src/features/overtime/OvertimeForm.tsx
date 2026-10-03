import { useState, type FormEvent } from "react";
import { Field, FormAlert } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import { todayIso } from "@/features/attendance/format";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { fileOvertime } from "./api";
import { checkOvertime } from "./validation";

/** Files overtime for yourself, or, with `forSomeone`, HR filing for an employee. */
export function OvertimeForm({
  forSomeone = false,
  onSaved,
  onCancel,
}: {
  forSomeone?: boolean;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkOvertime(formValues(e.currentTarget), forSomeone);
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      await fileOvertime(checked.value);
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
      aria-label="File overtime"
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      {forSomeone && (
        <Field name="employeeNo" label="Employee number" autoFocus error={errors.employeeNo} />
      )}
      <Field
        name="workDate"
        label="Date"
        type="date"
        autoFocus={!forSomeone}
        defaultValue={todayIso()}
        error={errors.workDate}
      />
      <div className="grid grid-cols-2 gap-4">
        <Field name="startTime" label="From" type="time" error={errors.startTime} />
        <Field name="endTime" label="To" type="time" error={errors.endTime} />
      </div>
      <Field name="reason" label="What it's for" error={errors.reason} />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Overtime must be outside the work schedule, unless it&apos;s a rest day or a holiday off,
        and at most 12 hours. A time earlier than the start means the next day. It&apos;s paid only
        once HR approves it.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          File overtime
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
