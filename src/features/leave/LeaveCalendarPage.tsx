import { useEffect, useState } from "react";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import { FormAlert } from "@/components/form";
import { useSession } from "@/features/auth/session";
import { monthRange, thisMonth, todayIso } from "@/features/attendance/format";
import type { AppError } from "@/lib/ipc";
import { listLeaveRequests } from "./api";
import { monthWeeks } from "./format";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** HR's team leave calendar (`/leave-calendar`): who is away on each day of a month. */
export function LeaveCalendarPage({ initialMonth }: { initialMonth?: string }) {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Only Admin and HR can see everyone&apos;s leave. Yours is under My leave.
      </p>
    );
  }
  return <LeaveCalendar initialMonth={initialMonth ?? thisMonth()} />;
}

function LeaveCalendar({ initialMonth }: { initialMonth: string }) {
  const [month, setMonth] = useState(initialMonth);
  const [requests, setRequests] = useState<LeaveRequest[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const today = todayIso();

  useEffect(() => {
    if (!month) return;
    let live = true;
    listLeaveRequests(null, monthRange(month))
      .then(
        (r) =>
          live && setRequests(r.filter((x) => x.status === "APPROVED" || x.status === "PENDING")),
      )
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [month]);

  const away = (date: string) =>
    (requests ?? []).filter((r) => r.startDate <= date && date <= r.endDate);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-tight">Leave calendar</h1>
        <div className="flex flex-col gap-1">
          <label htmlFor="month" className="text-sm font-medium">
            Month
          </label>
          <input
            id="month"
            type="month"
            value={month}
            onChange={(e) => {
              setRequests(null);
              setMonth(e.target.value);
            }}
            className="w-fit rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900"
          />
        </div>
      </div>
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        Approved leave, plus requests still waiting for a decision (marked &ldquo;waiting&rdquo;).
        Use it to spot days when too many people are out before approving more.
      </p>
      <FormAlert message={alert} />
      {requests && month && (
        <table className="w-full table-fixed border-collapse text-sm">
          <thead>
            <tr>
              {WEEKDAYS.map((d) => (
                <th key={d} className="py-2 text-left font-medium text-zinc-500">
                  {d}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {monthWeeks(month).map((week, i) => (
              <tr key={i}>
                {week.map((date, j) => {
                  if (!date)
                    return <td key={j} className="border border-zinc-100 dark:border-zinc-900" />;
                  const people = away(date);
                  return (
                    <td
                      key={date}
                      aria-label={`${date}: ${people.length === 0 ? "nobody away" : `${people.length} away`}`}
                      className={`h-24 border border-zinc-200 p-1 align-top dark:border-zinc-800 ${
                        j >= 5 ? "bg-zinc-50 dark:bg-zinc-900/50" : ""
                      }`}
                    >
                      <div
                        className={`text-xs tabular-nums ${
                          date === today
                            ? "font-semibold text-sky-700 dark:text-sky-400"
                            : "text-zinc-500"
                        }`}
                      >
                        {Number(date.slice(8))}
                      </div>
                      <ul className="mt-1 space-y-0.5">
                        {people.map((r) => (
                          <li
                            key={r.id}
                            title={`${r.employeeName}: ${r.leaveTypeName}${r.halfDay ? " (half day)" : ""}`}
                            className={`truncate rounded px-1 ${
                              r.status === "APPROVED"
                                ? "bg-emerald-50 text-emerald-900 dark:bg-emerald-950 dark:text-emerald-200"
                                : "text-zinc-500 italic"
                            }`}
                          >
                            {r.employeeName.split(",")[0]} · {r.leaveTypeCode}
                            {r.halfDay && " ½"}
                            {r.status === "PENDING" && " (waiting)"}
                          </li>
                        ))}
                      </ul>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
