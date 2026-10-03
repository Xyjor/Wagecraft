import { useEffect, useState } from "react";
import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import { FormAlert } from "@/components/form";
import { Badge } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { myAttendance } from "./api";
import { formatClock, formatMinutes, monthRange, thisMonth } from "./format";

/** The signed-in person's own time records (`/my-attendance`), a month at a time. */
export function MyAttendancePage() {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn't linked to an employee record. HR can link it from your employee profile.
      </p>
    );
  }
  return <MyAttendance />;
}

function MyAttendance() {
  const [month, setMonth] = useState(thisMonth);
  const [records, setRecords] = useState<AttendanceRecord[] | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    if (!month) return;
    let live = true;
    myAttendance(monthRange(month))
      .then((rows) => {
        if (!live) return;
        setRecords(rows);
        setAlert(undefined);
      })
      .catch((e: AppError) => {
        if (live) setAlert(e.message);
      });
    return () => {
      live = false;
    };
  }, [month]);

  return (
    <div className="max-w-4xl space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-xl font-semibold">My attendance</h1>
        <div className="flex flex-col gap-1">
          <label htmlFor="month" className="text-sm font-medium">
            Month
          </label>
          <input
            id="month"
            type="month"
            value={month}
            onChange={(e) => setMonth(e.target.value)}
            className="rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900"
          />
        </div>
      </div>
      <FormAlert message={alert} />
      {records && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Date</th>
              <th className="py-2 font-medium">Time in</th>
              <th className="py-2 font-medium">Time out</th>
              <th className="py-2 font-medium">Late</th>
              <th className="py-2 font-medium">Undertime</th>
              <th className="py-2 font-medium">Worked</th>
              <th className="py-2 font-medium">Night</th>
              <th className="py-2 font-medium">
                <span className="sr-only">Notes</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {records.length === 0 && (
              <tr>
                <td colSpan={8} className="py-6 text-center text-zinc-500">
                  No time records this month.
                </td>
              </tr>
            )}
            {records.map((r) => (
              <tr key={r.id} className="border-b border-zinc-100 dark:border-zinc-900">
                <td className="py-2">{formatDate(r.workDate)}</td>
                <td className="py-2 tabular-nums">{formatClock(r.timeIn)}</td>
                <td className="py-2 tabular-nums">
                  {r.timeOut ? formatClock(r.timeOut) : r.timeIn ? "Not yet" : ""}
                </td>
                <td className="py-2 tabular-nums">{formatMinutes(r.lateMinutes)}</td>
                <td className="py-2 tabular-nums">{formatMinutes(r.undertimeMinutes)}</td>
                <td className="py-2 tabular-nums">{formatMinutes(r.workedMinutes)}</td>
                <td className="py-2 tabular-nums">{formatMinutes(r.nightMinutes)}</td>
                <td className="py-2">
                  {r.needsReview && <Badge tone="muted">HR to review</Badge>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <p className="text-sm text-zinc-500">
        Clock in and out on the time clock from the sign-in screen. Something wrong? Ask HR to
        correct it.
      </p>
    </div>
  );
}
