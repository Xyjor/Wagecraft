import type { ReactNode } from "react";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { Badge } from "@/components/ui";
import { formatMinutes } from "@/features/attendance/format";
import { formatDate } from "@/lib/dates";
import { overtimeHours, STATUS_LABELS, STATUS_TONES } from "./format";

/** Overtime requests as a table. `actions` adds buttons per row; `showEmployee` adds a column. */
export function OvertimeTable({
  requests,
  empty,
  showEmployee = false,
  actions,
}: {
  requests: OvertimeRequest[];
  empty: string;
  showEmployee?: boolean;
  actions?: (r: OvertimeRequest) => ReactNode;
}) {
  const columns = 5 + (showEmployee ? 1 : 0) + (actions ? 1 : 0);
  return (
    <table className="w-full text-left text-sm">
      <thead className="border-b border-rule-strong bg-sunken text-xs text-ink-muted">
        <tr>
          {showEmployee && <th className="px-3 py-2 font-medium">Employee</th>}
          <th className="px-3 py-2 font-medium">Date</th>
          <th className="px-3 py-2 font-medium">Hours</th>
          <th className="px-3 py-2 font-medium">Length</th>
          <th className="px-3 py-2 font-medium">Reason</th>
          <th className="px-3 py-2 font-medium">Status</th>
          {actions && (
            <th className="px-3 py-2 font-medium">
              <span className="sr-only">Actions</span>
            </th>
          )}
        </tr>
      </thead>
      <tbody>
        {requests.length === 0 && (
          <tr>
            <td colSpan={columns} className="px-3 py-3 text-ink-muted">
              {empty}
            </td>
          </tr>
        )}
        {requests.map((r) => (
          <tr key={r.id} className="border-b border-rule align-top hover:bg-sunken/60">
            {showEmployee && (
              <td className="px-3 py-2">
                <div className="font-medium">{r.employeeName}</div>
                <div className="text-ink-faint">{r.employeeNo}</div>
              </td>
            )}
            <td className="px-3 py-2">{formatDate(r.workDate)}</td>
            <td className="px-3 py-2 tabular-nums">{overtimeHours(r)}</td>
            <td className="px-3 py-2 tabular-nums">{formatMinutes(r.minutes)}</td>
            <td className="px-3 py-2">{r.reason}</td>
            <td className="px-3 py-2">
              <Badge tone={STATUS_TONES[r.status]}>{STATUS_LABELS[r.status]}</Badge>
              {r.decisionNote && (
                <div className="mt-1 text-ink-faint">&ldquo;{r.decisionNote}&rdquo;</div>
              )}
            </td>
            {actions && <td className="px-3 py-2">{actions(r)}</td>}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
