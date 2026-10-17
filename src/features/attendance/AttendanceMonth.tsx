import { useEffect, useState } from "react";
import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import type { DateRange } from "@/bindings/DateRange";
import { FormAlert } from "@/components/form";
import { Badge } from "@/components/ui";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { formatClock, formatMinutes, monthRange, thisMonth } from "./format";
import { minutesInk } from "./format";

/** One person's time records for a month they pick. Used on My attendance and the profile. */
export function AttendanceMonth({
  load,
}: {
  load: (range: DateRange) => Promise<AttendanceRecord[]>;
}) {
  const [month, setMonth] = useState(thisMonth);
  const [records, setRecords] = useState<AttendanceRecord[] | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    if (!month) return;
    let live = true;
    load(monthRange(month))
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
  }, [month, load]);

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-1">
        <label htmlFor="month" className="text-sm font-medium">
          Month
        </label>
        <input
          id="month"
          type="month"
          value={month}
          onChange={(e) => setMonth(e.target.value)}
          className="w-fit rounded-md border border-field bg-surface px-3 py-1.5 text-sm text-ink"
        />
      </div>
      <FormAlert message={alert} />
      {records && <RecordTable records={records} />}
    </div>
  );
}

function RecordTable({ records }: { records: AttendanceRecord[] }) {
  return (
    <table className="w-full text-left text-sm">
      <thead className="border-b border-rule-strong bg-sunken text-xs whitespace-nowrap text-ink-muted">
        <tr>
          <th className="px-3 py-2 font-medium">Date</th>
          <th className="px-3 py-2 font-medium">Time in</th>
          <th className="px-3 py-2 font-medium">Time out</th>
          <th className="px-3 py-2 font-medium">Late</th>
          <th className="px-3 py-2 font-medium">Undertime</th>
          <th className="px-3 py-2 font-medium">Worked</th>
          <th className="px-3 py-2 font-medium">Night</th>
          <th className="px-3 py-2 font-medium">
            <span className="sr-only">Notes</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {records.length === 0 && (
          <tr>
            <td colSpan={8} className="px-3 py-6 text-center text-ink-muted">
              No time records this month.
            </td>
          </tr>
        )}
        {records.map((r) => (
          <tr key={r.id} className="border-b border-rule hover:bg-sunken/60">
            <td className="px-3 py-2">{formatDate(r.workDate)}</td>
            <td className="px-3 py-2 whitespace-nowrap tabular-nums">{formatClock(r.timeIn)}</td>
            <td className="px-3 py-2 whitespace-nowrap tabular-nums">
              {r.timeOut ? formatClock(r.timeOut) : r.timeIn ? "Not yet" : ""}
            </td>
            <td className={`px-3 py-2 tabular-nums ${minutesInk(r.lateMinutes)}`}>
              {formatMinutes(r.lateMinutes)}
            </td>
            <td className={`px-3 py-2 tabular-nums ${minutesInk(r.undertimeMinutes)}`}>
              {formatMinutes(r.undertimeMinutes)}
            </td>
            <td className="px-3 py-2 whitespace-nowrap tabular-nums">
              {formatMinutes(r.workedMinutes)}
            </td>
            <td className="px-3 py-2 whitespace-nowrap tabular-nums">
              {formatMinutes(r.nightMinutes)}
            </td>
            <td className="space-x-1 px-3 py-2">
              {r.needsReview && <Badge tone="muted">HR to review</Badge>}
              {r.source === "MANUAL" && <Badge tone="muted">Corrected by HR</Badge>}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
