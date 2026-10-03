import { useState, type FormEvent } from "react";
import type { Employee } from "@/bindings/Employee";
import { Field, FormAlert } from "@/components/form";
import { primaryButton } from "@/components/ui";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { setKioskPin } from "./api";
import { checkPin } from "./validation";

/** Sets or replaces the PIN this employee types on the time clock (plan §6.3). */
export function KioskPinTab({
  employee,
  onSaved,
}: {
  employee: Employee;
  onSaved: () => Promise<void>;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);

  if (employee.archivedAt !== null) {
    return <p className="text-sm text-zinc-500">Archived employees can't use the time clock.</p>;
  }

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = e.currentTarget;
    setSaved(false);
    const checked = checkPin(formValues(form));
    if (!checked.ok) {
      setErrors(checked.errors);
      setAlert(undefined);
      return;
    }
    setBusy(true);
    try {
      await setKioskPin(employee.id, checked.value);
      form.reset();
      setErrors({});
      setAlert(undefined);
      setSaved(true);
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="max-w-sm space-y-4">
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        {employee.hasKioskPin
          ? "This employee has a PIN. Setting a new one replaces it and lets them back in if the time clock locked them out."
          : "No PIN yet. Set one so this employee can clock in and out on the time clock."}
      </p>
      {saved && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
          PIN saved. Tell {employee.firstName} the new PIN in person.
        </p>
      )}
      <form onSubmit={submit} noValidate aria-label="Kiosk PIN" className="flex flex-col gap-4">
        <FormAlert message={alert} />
        <Field
          name="pin"
          label="New PIN"
          type="password"
          inputMode="numeric"
          maxLength={6}
          autoComplete="new-password"
          error={errors.pin}
        />
        <Field
          name="confirmPin"
          label="Type it again"
          type="password"
          inputMode="numeric"
          maxLength={6}
          autoComplete="new-password"
          error={errors.confirmPin}
        />
        <div>
          <button type="submit" disabled={busy} className={primaryButton}>
            {employee.hasKioskPin ? "Replace PIN" : "Set PIN"}
          </button>
        </div>
      </form>
    </div>
  );
}
