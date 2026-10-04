import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { DayRow } from "@/bindings/DayRow";
import type { ReviewItem } from "@/bindings/ReviewItem";
import { Field, FormAlert } from "@/components/form";
import { Badge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { attendanceDay, markReviewed, reviewQueue, saveAttendance } from "./api";
import { formatClock, formatMinutes, shiftDate, todayIso } from "./format";

const STATUS_LABELS: Record<string, string> = {
  PRESENT: "Present",
  ABSENT: "Absent",
  REST_DAY: "Rest day",
  HOLIDAY: "Holiday",
  ON_LEAVE: "On leave",
};

/** Who is being corrected: an employee on a date, with any times already recorded. */
type Editing = {
  employeeId: number;
  employeeName: string;
  workDate: string;
  timeIn: string | null;
  timeOut: string | null;
};

/** HR's attendance (`/attendance`): the review queue, then everyone's day on one date. */
export function AttendancePage() {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return <p className="text-zinc-600 dark:text-zinc-400">Only Admin and HR can see this.</p>;
  }
  return <Attendance />;
}

function Attendance() {
  const [date, setDate] = useState(todayIso);
  const [rows, setRows] = useState<DayRow[] | null>(null);
  const [queue, setQueue] = useState<ReviewItem[]>([]);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [alert, setAlert] = useState<string>();

  const reload = useCallback(async () => {
    const [day, flagged] = await Promise.all([attendanceDay(date), reviewQueue()]);
    setRows(day);
    setQueue(flagged);
  }, [date]);

  useEffect(() => {
    if (!date) return;
    let live = true;
    Promise.all([attendanceDay(date), reviewQueue()])
      .then(([day, flagged]) => {
        if (!live) return;
        setRows(day);
        setQueue(flagged);
        setAlert(undefined);
      })
      .catch((e: AppError) => {
        if (live) setAlert(e.message);
      });
    return () => {
      live = false;
    };
  }, [date]);

  async function looksFine(id: number) {
    setAlert(undefined);
    try {
      await markReviewed(id);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  const today = todayIso();

  return (
    <div className="max-w-6xl space-y-8">
      <h1 className="text-xl font-semibold">Attendance</h1>
      <FormAlert message={alert} />

      {queue.length > 0 && (
        <section className="space-y-3" aria-labelledby="review-heading">
          <h2 id="review-heading" className="text-lg font-semibold">
            Needs review ({queue.length})
          </h2>
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <th className="py-2 font-medium">Date</th>
                <th className="py-2 font-medium">Employee</th>
                <th className="py-2 font-medium">Time in</th>
                <th className="py-2 font-medium">Time out</th>
                <th className="py-2 font-medium">Why</th>
                <th className="py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {queue.map(({ employeeName, record: r }) => (
                <tr key={r.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2">{formatDate(r.workDate)}</td>
                  <td className="py-2 font-medium">{employeeName}</td>
                  <td className="py-2 tabular-nums">{formatClock(r.timeIn)}</td>
                  <td className="py-2 tabular-nums">{formatClock(r.timeOut)}</td>
                  <td className="py-2 text-zinc-600 dark:text-zinc-400">{r.reviewNote}</td>
                  <td className="py-2">
                    <div className="flex justify-end gap-1">
                      <button
                        type="button"
                        className={quietButton}
                        disabled={r.locked}
                        aria-label={`Correct ${employeeName} on ${r.workDate}`}
                        onClick={() =>
                          setEditing({
                            employeeId: r.employeeId,
                            employeeName,
                            workDate: r.workDate,
                            timeIn: r.timeIn,
                            timeOut: r.timeOut,
                          })
                        }
                      >
                        Correct
                      </button>
                      {/* A forgotten time out isn't flagged; only a correction settles it. */}
                      {r.needsReview && (
                        <button
                          type="button"
                          className={quietButton}
                          aria-label={`Mark ${employeeName} on ${r.workDate} as reviewed`}
                          onClick={() => looksFine(r.id)}
                        >
                          Looks fine
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {editing && (
        <CorrectionForm
          key={`${editing.employeeId}-${editing.workDate}`}
          editing={editing}
          onCancel={() => setEditing(null)}
          onSaved={async () => {
            setEditing(null);
            await reload();
          }}
        />
      )}

      <section className="space-y-3" aria-labelledby="day-heading">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <h2 id="day-heading" className="text-lg font-semibold">
            {date ? formatDate(date) : "Pick a date"}
            {rows?.[0]?.holiday && (
              <span className="ml-2 text-sm font-normal text-zinc-500">{rows[0].holiday}</span>
            )}
          </h2>
          <div className="flex items-end gap-1">
            <button
              type="button"
              className={quietButton}
              onClick={() => setDate((d) => shiftDate(d, -1))}
            >
              Previous day
            </button>
            <div className="flex flex-col gap-1">
              <label htmlFor="date" className="text-sm font-medium">
                Date
              </label>
              <input
                id="date"
                type="date"
                value={date}
                max={today}
                onChange={(e) => setDate(e.target.value)}
                className="rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900"
              />
            </div>
            <button
              type="button"
              className={quietButton}
              disabled={date >= today}
              onClick={() => setDate((d) => shiftDate(d, 1))}
            >
              Next day
            </button>
          </div>
        </div>
        {rows && (
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <th className="py-2 font-medium">Employee</th>
                <th className="py-2 font-medium">Department</th>
                <th className="py-2 font-medium">Status</th>
                <th className="py-2 font-medium">Time in</th>
                <th className="py-2 font-medium">Time out</th>
                <th className="py-2 font-medium">Late</th>
                <th className="py-2 font-medium">Undertime</th>
                <th className="py-2 font-medium">Worked</th>
                <th className="py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.length === 0 && (
                <tr>
                  <td colSpan={9} className="py-6 text-center text-zinc-500">
                    No current employees on this date.
                  </td>
                </tr>
              )}
              {rows.map((row) => {
                const r = row.record;
                return (
                  <tr
                    key={row.employeeId}
                    className="border-b border-zinc-100 dark:border-zinc-900"
                  >
                    <td className="py-2">
                      <span className="font-medium">{row.employeeName}</span>{" "}
                      <span className="text-zinc-500">{row.employeeNo}</span>
                    </td>
                    <td className="py-2">{row.departmentName}</td>
                    <td className="space-x-1 py-2">
                      <StatusBadge status={row.status} isToday={date === today} />
                      {row.leave && <span className="text-zinc-500">{row.leave}</span>}
                      {r?.needsReview && <Badge tone="muted">Needs review</Badge>}
                      {r?.source === "MANUAL" && <Badge tone="muted">Corrected</Badge>}
                    </td>
                    <td className="py-2 tabular-nums">{formatClock(r?.timeIn)}</td>
                    <td className="py-2 tabular-nums">{formatClock(r?.timeOut)}</td>
                    <td className="py-2 tabular-nums">{formatMinutes(r?.lateMinutes ?? 0)}</td>
                    <td className="py-2 tabular-nums">{formatMinutes(r?.undertimeMinutes ?? 0)}</td>
                    <td className="py-2 tabular-nums">{formatMinutes(r?.workedMinutes ?? 0)}</td>
                    <td className="py-2 text-right">
                      {(r || !row.fullDayLeave) && (
                        <button
                          type="button"
                          className={quietButton}
                          disabled={r?.locked}
                          aria-label={`${r ? "Correct" : "Add"} ${row.employeeName}`}
                          onClick={() =>
                            setEditing({
                              employeeId: row.employeeId,
                              employeeName: row.employeeName,
                              workDate: date,
                              timeIn: r?.timeIn ?? null,
                              timeOut: r?.timeOut ?? null,
                            })
                          }
                        >
                          {r ? "Correct" : "Add"}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

function StatusBadge({ status, isToday }: { status: string | null; isToday: boolean }) {
  if (status === null) return <Badge tone="muted">No schedule</Badge>;
  if (status === "ABSENT" && isToday) return <Badge tone="muted">No time in yet</Badge>;
  const tone = status === "PRESENT" ? "good" : status === "ABSENT" ? "bad" : "muted";
  return <Badge tone={tone}>{STATUS_LABELS[status] ?? status}</Badge>;
}

function CorrectionForm({
  editing,
  onSaved,
  onCancel,
}: {
  editing: Editing;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const v = formValues(e.currentTarget);
    const local: Record<string, string> = {};
    if (!v.timeIn) local.timeIn = "Enter the time in";
    if ((v.reason ?? "").trim().length < 3) local.reason = "Say why, in 3 to 200 characters";
    if (Object.keys(local).length) {
      setErrors(local);
      return;
    }
    setBusy(true);
    try {
      await saveAttendance({
        employeeId: editing.employeeId,
        workDate: editing.workDate,
        timeIn: v.timeIn,
        timeOut: v.timeOut || null,
        reason: v.reason.trim(),
      });
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  const hhmm = (t: string | null) => (t ? t.slice(11, 16) : undefined);

  return (
    <form
      onSubmit={submit}
      noValidate
      aria-label={`Attendance for ${editing.employeeName}`}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <p className="font-medium">
        {editing.employeeName} · {formatDate(editing.workDate)}
      </p>
      <FormAlert message={alert} />
      <div className="grid grid-cols-2 gap-4">
        <Field
          name="timeIn"
          label="Time in"
          type="time"
          defaultValue={hhmm(editing.timeIn)}
          error={errors.timeIn}
        />
        <Field
          name="timeOut"
          label="Time out"
          type="time"
          defaultValue={hhmm(editing.timeOut)}
          error={errors.timeOut}
        />
      </div>
      <Field name="reason" label="Reason for the change" error={errors.reason} />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        A time out earlier than the time in means the next day. The reason is saved in the audit
        log. Leave the time out empty if they are still at work.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          Save
        </button>
        <button type="button" onClick={onCancel} className={quietButton}>
          Cancel
        </button>
      </div>
    </form>
  );
}
