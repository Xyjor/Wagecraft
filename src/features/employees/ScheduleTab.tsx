import { useEffect, useState, type FormEvent } from "react";
import type { Employee } from "@/bindings/Employee";
import type { ScheduleAssignment } from "@/bindings/ScheduleAssignment";
import type { WorkSchedule } from "@/bindings/WorkSchedule";
import { Field, FormAlert, SelectField } from "@/components/form";
import { Badge, primaryButton } from "@/components/ui";
import { listSchedules } from "@/features/org/api";
import { scheduleHours, workDaysLabel } from "@/features/org/format";
import { formatDate } from "@/lib/dates";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { changeSchedule, scheduleHistory } from "./api";

type Loaded = { history: ScheduleAssignment[]; schedules: WorkSchedule[] };

function describe(s: WorkSchedule): string {
  return `${s.name} · ${scheduleHours(s)}, ${workDaysLabel(s.workDays)}`;
}

/** Today on this computer, as YYYY-MM-DD. */
function today(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Schedule history, newest first, with a form to move the employee from a date. */
export function ScheduleTab({ employee }: { employee: Employee }) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    Promise.all([scheduleHistory(employee.id), listSchedules()])
      .then(([history, schedules]) => setLoaded({ history, schedules }))
      .catch((e: AppError) => setAlert(e.message));
  }, [employee.id]);

  if (!loaded) return <FormAlert message={alert} />;
  const { history, schedules } = loaded;
  const byId = new Map(schedules.map((s) => [s.id, s]));
  const now = today();
  const current = [...history].reverse().find((a) => a.effectiveFrom <= now);
  const newestFirst = [...history].reverse();

  return (
    <div className="space-y-6">
      {history.length === 0 ? (
        <p className="text-sm text-zinc-500">
          No work schedule yet. Set one before this employee clocks in.
        </p>
      ) : (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Starts</th>
              <th className="py-2 font-medium">Schedule</th>
              <th className="py-2 pl-6 font-medium">Reason</th>
              <th className="py-2 font-medium">Changed by</th>
            </tr>
          </thead>
          <tbody>
            {newestFirst.map((a) => {
              const s = a.scheduleId === null ? undefined : byId.get(a.scheduleId);
              return (
                <tr key={a.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2">
                    <span className="mr-2">{formatDate(a.effectiveFrom)}</span>
                    {a === current && <Badge tone="good">Current</Badge>}
                    {a.effectiveFrom > now && <Badge tone="muted">Upcoming</Badge>}
                  </td>
                  <td className="py-2">{s ? describe(s) : (a.scheduleName ?? "No schedule")}</td>
                  <td className="py-2 pl-6">{a.reason}</td>
                  <td className="py-2">{a.createdByName}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      {employee.archivedAt === null && (
        <MoveForm
          key={history.length}
          employee={employee}
          schedules={schedules.filter((s) => s.isActive)}
          onMoved={(history) => setLoaded({ history, schedules })}
        />
      )}
    </div>
  );
}

function MoveForm({
  employee,
  schedules,
  onMoved,
}: {
  employee: Employee;
  schedules: WorkSchedule[];
  onMoved: (history: ScheduleAssignment[]) => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const v = formValues(e.currentTarget);
    const found: Record<string, string> = {};
    if (!v.scheduleId) found.scheduleId = "Pick a schedule";
    if (!v.effectiveFrom) found.effectiveFrom = "Enter the date the new schedule starts";
    const reason = v.reason?.trim() || null;
    if (reason && reason.length > 200) found.reason = "Keep the reason to 200 characters";
    setErrors(found);
    setAlert(undefined);
    if (Object.keys(found).length) return;
    setBusy(true);
    try {
      onMoved(
        await changeSchedule(employee.id, {
          scheduleId: Number(v.scheduleId),
          effectiveFrom: v.effectiveFrom,
          reason,
        }),
      );
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate className="space-y-3">
      <h2 className="text-lg font-semibold">Change schedule</h2>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Days before the start date keep the schedule they had. The start date has to come after the
        last day with attendance recorded. Leave from that date is counted again with the new work
        days.
      </p>
      <FormAlert message={alert} />
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField
          name="scheduleId"
          label="New schedule"
          options={[
            { value: "", label: "Choose a schedule" },
            ...schedules.map((s) => ({ value: String(s.id), label: describe(s) })),
          ]}
          defaultValue=""
          error={errors.scheduleId}
        />
        <Field name="effectiveFrom" label="Starts on" type="date" error={errors.effectiveFrom} />
        <Field name="reason" label="Reason" error={errors.reason} />
      </div>
      <button type="submit" disabled={busy} className={primaryButton}>
        Save schedule
      </button>
    </form>
  );
}
