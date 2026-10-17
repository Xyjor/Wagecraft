import { useEffect, useState } from "react";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import { FormAlert } from "@/components/form";
import { useSession } from "@/features/auth/session";
import { monthRange, thisMonth, todayIso } from "@/features/attendance/format";
import type { AppError } from "@/lib/ipc";
import { listLeaveRequests } from "./api";
import { monthWeeks } from "./format";
import { PageHeader } from "@/components/PageHeader";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** HR's team leave calendar (`/leave-calendar`): who is away on each day of a month. */
export function LeaveCalendarPage({ initialMonth }: { initialMonth?: string }) {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-ink-muted">
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
      <PageHeader
        title="Leave calendar"
        actions={
          <>
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
                className="w-fit rounded-md border border-field bg-surface px-3 py-1.5 text-sm text-ink"
              />
            </div>
          </>
        }
      />
      <p className="max-w-prose text-sm text-ink-muted">
        Approved leave, plus requests still waiting for a decision (marked &ldquo;waiting&rdquo;).
        Use it to spot days when too many people are out before approving more.
      </p>
      <FormAlert message={alert} />
      {requests && month && (
        <table className="w-full table-fixed border-collapse text-sm">
          <thead>
            <tr>
              {WEEKDAYS.map((d) => (
                <th key={d} className="py-2 text-left font-medium text-ink-faint">
                  {d}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {monthWeeks(month).map((week, i) => (
              <tr key={i}>
                {week.map((date, j) => {
                  if (!date) return <td key={j} className="border border-rule" />;
                  const people = away(date);
                  return (
                    <td
                      key={date}
                      aria-label={`${date}: ${people.length === 0 ? "nobody away" : `${people.length} away`}`}
                      className={`h-24 border border-rule p-1 align-top ${
                        j >= 5 ? "bg-sunken/50" : ""
                      }`}
                    >
                      <div
                        className={`text-xs tabular-nums ${
                          date === today
                            ? "inline-grid size-5 place-items-center rounded-full bg-accent font-semibold text-accent-ink"
                            : "text-ink-faint"
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
                                ? "bg-accent-soft text-accent"
                                : "text-ink-faint italic"
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
