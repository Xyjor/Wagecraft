import type { ReactNode } from "react";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import { Badge } from "@/components/ui";
import { formatDays, leaveDates, STATUS_LABELS, STATUS_TONES } from "./format";

/** Leave requests as a table. `actions` adds buttons per row; `showEmployee` adds a column. */
export function LeaveRequestTable({
  requests,
  empty,
  showEmployee = false,
  actions,
}: {
  requests: LeaveRequest[];
  empty: string;
  showEmployee?: boolean;
  actions?: (r: LeaveRequest) => ReactNode;
}) {
  const columns = 5 + (showEmployee ? 1 : 0) + (actions ? 1 : 0);
  return (
    <table className="w-full text-left text-sm">
      <thead className="border-b border-rule-strong bg-sunken text-xs text-ink-muted">
        <tr>
          {showEmployee && <th className="px-3 py-2 font-medium">Employee</th>}
          <th className="px-3 py-2 font-medium">Leave</th>
          <th className="px-3 py-2 font-medium">Dates</th>
          <th className="px-3 py-2 font-medium">Takes</th>
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
            <td className="px-3 py-2">
              {r.leaveTypeName}
              {!r.isPaid && <div className="text-ink-faint">Unpaid</div>}
            </td>
            <td className="px-3 py-2">{leaveDates(r)}</td>
            <td className="px-3 py-2 tabular-nums">{formatDays(r.halfdays)}</td>
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
