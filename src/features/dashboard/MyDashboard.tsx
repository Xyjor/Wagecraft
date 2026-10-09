import { useEffect, useState } from "react";
import { Link } from "react-router";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { MyDashboard as Summary } from "@/bindings/MyDashboard";
import type { MyDay } from "@/bindings/MyDay";
import type { MyPayslip } from "@/bindings/MyPayslip";
import { FormAlert } from "@/components/form";
import { formatClock, formatMinutes } from "@/features/attendance/format";
import { myBalances } from "@/features/leave/api";
import { formatDays } from "@/features/leave/format";
import { myPayslips } from "@/features/payroll/api";
import { periodLabel } from "@/features/payroll/periods";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { getMyDashboard } from "./api";
import { plural } from "./labels";
import { Panel, Tile } from "./parts";

interface Loaded {
  summary: Summary;
  balances: LeaveBalance[];
  latest: MyPayslip | undefined;
}

/** Staff's home page (plan §6.9): today, this cutoff, leave left, pay and requests. */
export function MyDashboard() {
  const [data, setData] = useState<Loaded | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    let live = true;
    getMyDashboard()
      .then(async (summary) => {
        // The office clock's year, so the balances match the day the dashboard shows.
        const year = Number(summary.today.date.slice(0, 4));
        const [balances, slips] = await Promise.all([myBalances(year), myPayslips()]);
        if (live) setData({ summary, balances, latest: slips[0] });
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, []);

  if (alert) return <FormAlert message={alert} />;
  if (!data) return <p className="text-zinc-500">Loading your dashboard…</p>;
  const { summary, balances, latest } = data;
  const { cutoff } = summary;
  const waiting = summary.pendingLeave + summary.pendingOvertime > 0;

  return (
    <div className="space-y-6">
      <Panel title="Today">
        <TodayLine day={summary.today} />
      </Panel>

      <Panel title={`This cutoff, ${periodLabel(cutoff.periodStart, cutoff.periodEnd)}`}>
        <div className="grid grid-cols-3 gap-3">
          <Tile label="Days present" value={cutoff.daysPresent} />
          <Tile label="Late" value={formatMinutes(cutoff.lateMinutes) || "None"} />
          <Tile label="Approved overtime" value={formatMinutes(cutoff.overtimeMinutes) || "None"} />
        </div>
      </Panel>

      <div className="grid gap-4 lg:grid-cols-3">
        <Panel title="Leave left this year">
          {balances.length === 0 ? (
            <p className="text-sm text-zinc-500">No leave credits yet.</p>
          ) : (
            <ul className="space-y-1 text-sm">
              {balances.map((b) => (
                <li key={b.id} className="flex justify-between gap-2">
                  <span>{b.leaveTypeName}</span>
                  <span className="text-zinc-600 tabular-nums dark:text-zinc-400">
                    {formatDays(b.entitledHalfdays - b.usedHalfdays)} left of{" "}
                    {formatDays(b.entitledHalfdays)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="Latest payslip">
          {latest ? (
            <div className="space-y-1 text-sm">
              <p>{periodLabel(latest.periodStart, latest.periodEnd)}</p>
              <p className="text-2xl font-semibold tabular-nums">{formatPesos(latest.netCents)}</p>
              <p className="text-zinc-500">
                Net pay, paid {formatDate(latest.payDate)}. Gross {formatPesos(latest.grossCents)}.
              </p>
              <Link to="/my-payslips" className="underline">
                View or download in My payslips
              </Link>
            </div>
          ) : (
            <p className="text-sm text-zinc-500">
              No payslips yet. Each one shows up once HR posts that payroll.
            </p>
          )}
        </Panel>

        <Panel title="My requests">
          {waiting ? (
            <ul className="space-y-1 text-sm">
              {summary.pendingLeave > 0 && (
                <li>
                  <Link to="/my-leave" className="underline">
                    {plural(summary.pendingLeave, "leave request", "leave requests")} waiting for HR
                  </Link>
                </li>
              )}
              {summary.pendingOvertime > 0 && (
                <li>
                  <Link to="/my-attendance" className="underline">
                    {plural(summary.pendingOvertime, "overtime request", "overtime requests")}{" "}
                    waiting for HR
                  </Link>
                </li>
              )}
            </ul>
          ) : (
            <p className="text-sm text-zinc-500">Nothing waiting for HR.</p>
          )}
        </Panel>
      </div>
    </div>
  );
}

function TodayLine({ day }: { day: MyDay }) {
  const late = day.lateMinutes > 0 ? `, ${formatMinutes(day.lateMinutes)} late` : "";
  let text: string;
  if (day.timeOut) text = `Clocked out at ${formatClock(day.timeOut)}${late}`;
  else if (day.timeIn) text = `Clocked in at ${formatClock(day.timeIn)}${late}`;
  else if (day.status === "HOLIDAY") text = `Holiday: ${day.holiday ?? ""}`;
  else if (day.status === "ON_LEAVE") text = `On leave: ${day.leave ?? ""}`;
  else if (day.status === "REST_DAY") text = "Rest day";
  else if (day.status === null) text = "No work schedule yet. HR can assign one.";
  else text = "Not clocked in yet";
  return (
    <p className="text-sm">
      <span className="text-zinc-500">{formatDate(day.date)}. </span>
      {text}
    </p>
  );
}
