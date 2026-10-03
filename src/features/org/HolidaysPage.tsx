import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { Holiday } from "@/bindings/Holiday";
import { Field, FormAlert, SelectField } from "@/components/form";
import { Badge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { createHoliday, deleteHoliday, listHolidays, updateHoliday } from "./api";
import { HOLIDAY_KIND_LABELS, weekdayOf } from "./format";
import { checkHoliday, HOLIDAY_KINDS } from "./validation";

export function HolidaysPage({ initialYear }: { initialYear?: number }) {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Only Admin and HR can manage the holiday calendar.
      </p>
    );
  }
  return <Holidays initialYear={initialYear ?? new Date().getFullYear()} />;
}

function Holidays({ initialYear }: { initialYear: number }) {
  const [year, setYear] = useState(initialYear);
  const [holidays, setHolidays] = useState<Holiday[] | null>(null);
  const [alert, setAlert] = useState<string>();
  /** Which form is open: none, a new holiday, or editing one. */
  const [form, setForm] = useState<null | "new" | Holiday>(null);
  const [confirming, setConfirming] = useState<number | null>(null);

  const reload = useCallback(async () => {
    setHolidays(await listHolidays(year));
  }, [year]);

  useEffect(() => {
    let current = true;
    listHolidays(year)
      .then((h) => current && setHolidays(h))
      .catch((e: AppError) => current && setAlert(e.message));
    return () => {
      current = false;
    };
  }, [year]);

  function changeYear(by: number) {
    setForm(null);
    setConfirming(null);
    setAlert(undefined);
    setHolidays(null);
    setYear((y) => y + by);
  }

  async function remove(h: Holiday) {
    setAlert(undefined);
    try {
      await deleteHoliday(h.id);
      setConfirming(null);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Holidays</h1>
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
      </div>
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        Enter each year&apos;s holidays from the official proclamation. The type sets the holiday
        pay. For a double holiday, add both holidays on the same date.
      </p>
      <FormAlert message={alert} />

      {form === null && (
        <button type="button" className={primaryButton} onClick={() => setForm("new")}>
          Add holiday
        </button>
      )}
      {form !== null && (
        <HolidayForm
          key={form === "new" ? "new" : form.id}
          editing={form === "new" ? null : form}
          year={year}
          onCancel={() => setForm(null)}
          onSaved={async () => {
            setForm(null);
            await reload();
          }}
        />
      )}

      {holidays && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Date</th>
              <th className="py-2 font-medium">Holiday</th>
              <th className="py-2 font-medium">Type</th>
              <th className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {holidays.length === 0 && (
              <tr>
                <td colSpan={4} className="py-3 text-zinc-500">
                  No holidays for {year} yet.
                </td>
              </tr>
            )}
            {holidays.map((h) => (
              <tr key={h.id} className="border-b border-zinc-100 dark:border-zinc-900">
                <td className="py-2 tabular-nums">
                  {weekdayOf(h.date)}, {formatDate(h.date)}
                </td>
                <td className="py-2 font-medium">{h.name}</td>
                <td className="py-2">
                  <Badge tone={h.kind === "SPECIAL_WORKING" ? "muted" : "good"}>
                    {HOLIDAY_KIND_LABELS[h.kind]}
                  </Badge>
                </td>
                <td className="py-2">
                  <div className="flex items-center justify-end gap-1">
                    {h.locked ? (
                      <span className="text-zinc-500">Payroll posted</span>
                    ) : confirming === h.id ? (
                      <>
                        <span>Delete {h.name}?</span>
                        <button type="button" className={quietButton} onClick={() => remove(h)}>
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
                      <>
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`Edit ${h.name}`}
                          onClick={() => setForm(h)}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`Delete ${h.name}`}
                          onClick={() => setConfirming(h.id)}
                        >
                          Delete
                        </button>
                      </>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

const KIND_OPTIONS = HOLIDAY_KINDS.map((k) => ({ value: k, label: HOLIDAY_KIND_LABELS[k] }));

function HolidayForm({
  editing,
  year,
  onSaved,
  onCancel,
}: {
  editing: Holiday | null;
  year: number;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    const checked = checkHoliday(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      if (editing) await updateHoliday(editing.id, checked.value);
      else await createHoliday(checked.value);
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
      aria-label={editing ? `Edit ${editing.name}` : "Add holiday"}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <Field
        name="date"
        label="Date"
        type="date"
        autoFocus
        defaultValue={editing?.date ?? `${year}-01-01`}
        error={errors.date}
      />
      <Field name="name" label="Name" defaultValue={editing?.name} error={errors.name} />
      <SelectField
        name="kind"
        label="Type"
        options={KIND_OPTIONS}
        defaultValue={editing?.kind ?? "REGULAR"}
        error={errors.kind}
      />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Regular holidays and special non-working days are days off. A special working day is an
        ordinary workday with no extra pay.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          Save holiday
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
