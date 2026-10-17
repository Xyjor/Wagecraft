import { lazy, Suspense, useEffect, useState } from "react";
import { Link } from "react-router";
import type { TeamDashboard as Summary } from "@/bindings/TeamDashboard";
import { EmptyState, ErrorState, Loading } from "@/components/states";
import { periodLabel } from "@/features/payroll/periods";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { getTeamDashboard } from "./api";
import { NO_DEPARTMENT, plural } from "./labels";
import { Panel, Tile, TileStrip } from "./parts";

// Recharts is big, so the charts load after the numbers, like the PDF library does.
const DepartmentChart = lazy(() =>
  import("./DashboardCharts").then((m) => ({ default: m.DepartmentChart })),
);
const PayrollCostChart = lazy(() =>
  import("./DashboardCharts").then((m) => ({ default: m.PayrollCostChart })),
);

/** Admin and HR's home page (plan §6.9): the team today, requests to review, two charts. */
export function TeamDashboard() {
  const [data, setData] = useState<Summary | null>(null);
  const [alert, setAlert] = useState<string>();
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    getTeamDashboard()
      .then(setData)
      .catch((e: AppError) => setAlert(e.message));
  }, [attempt]);

  function retry() {
    setAlert(undefined);
    setAttempt((a) => a + 1);
  }

  if (alert) return <ErrorState message={alert} onRetry={retry} />;
  if (!data) return <Loading label="Loading the dashboard…" />;
  const { today } = data;
  const waiting = data.pendingLeave + data.pendingOvertime > 0;

  return (
    <div className="space-y-6">
      <TileStrip columns="grid-cols-5">
        <Tile label="Employees" value={data.headcount}>
          {data.newHiresThisMonth} new this month
        </Tile>
        <Tile label="Present" value={today.present} />
        <Tile label="Late" value={today.late} tone={today.late > 0 ? "danger" : undefined} />
        <Tile
          label="Not clocked in"
          value={today.notIn}
          tone={today.notIn > 0 ? "warn" : undefined}
        />
        <Tile label="On leave" value={today.onLeave} />
      </TileStrip>

      <Panel title="Waiting for review">
        {waiting ? (
          <ul className="space-y-1 text-sm">
            {data.pendingLeave > 0 && (
              <li>
                <Link
                  to="/leave"
                  className="font-medium text-accent underline underline-offset-2 hover:text-accent-hover"
                >
                  {plural(data.pendingLeave, "leave request", "leave requests")}
                </Link>
              </li>
            )}
            {data.pendingOvertime > 0 && (
              <li>
                <Link
                  to="/overtime"
                  className="font-medium text-accent underline underline-offset-2 hover:text-accent-hover"
                >
                  {plural(data.pendingOvertime, "overtime request", "overtime requests")}
                </Link>
              </li>
            )}
          </ul>
        ) : (
          <EmptyState message="No requests waiting for review." />
        )}
      </Panel>

      <div className="grid gap-4 lg:grid-cols-2">
        <Panel title="Employees by department">
          <Suspense fallback={null}>
            <DepartmentChart rows={data.byDepartment} />
          </Suspense>
          <table aria-label="Employees by department" className="sr-only">
            <thead>
              <tr>
                <th>Department</th>
                <th>Employees</th>
              </tr>
            </thead>
            <tbody>
              {data.byDepartment.map((d) => (
                <tr key={d.department ?? ""}>
                  <td>{d.department ?? NO_DEPARTMENT}</td>
                  <td>{d.count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Panel>

        <Panel title="Payroll cost, last 6 posted periods">
          {data.payrollCost.length === 0 ? (
            <EmptyState message="No payroll posted yet." />
          ) : (
            <>
              <p className="text-xs text-ink-faint">Gross pay plus employer contributions.</p>
              <Suspense fallback={null}>
                <PayrollCostChart rows={data.payrollCost} />
              </Suspense>
              <table aria-label="Payroll cost by period" className="sr-only">
                <thead>
                  <tr>
                    <th>Period</th>
                    <th>Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {data.payrollCost.map((p) => (
                    <tr key={p.periodStart}>
                      <td>{periodLabel(p.periodStart, p.periodEnd)}</td>
                      <td>{formatPesos(p.costCents)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </Panel>
      </div>
    </div>
  );
}
