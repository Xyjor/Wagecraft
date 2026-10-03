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
      <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
        <tr>
          {showEmployee && <th className="py-2 font-medium">Employee</th>}
          <th className="py-2 font-medium">Leave</th>
          <th className="py-2 font-medium">Dates</th>
          <th className="py-2 font-medium">Takes</th>
          <th className="py-2 font-medium">Reason</th>
          <th className="py-2 font-medium">Status</th>
          {actions && (
            <th className="py-2 font-medium">
              <span className="sr-only">Actions</span>
            </th>
          )}
        </tr>
      </thead>
      <tbody>
        {requests.length === 0 && (
          <tr>
            <td colSpan={columns} className="py-3 text-zinc-500">
              {empty}
            </td>
          </tr>
        )}
        {requests.map((r) => (
          <tr key={r.id} className="border-b border-zinc-100 align-top dark:border-zinc-900">
            {showEmployee && (
              <td className="py-2">
                <div className="font-medium">{r.employeeName}</div>
                <div className="text-zinc-500">{r.employeeNo}</div>
              </td>
            )}
            <td className="py-2">
              {r.leaveTypeName}
              {!r.isPaid && <div className="text-zinc-500">Unpaid</div>}
            </td>
            <td className="py-2">{leaveDates(r)}</td>
            <td className="py-2 tabular-nums">{formatDays(r.halfdays)}</td>
            <td className="py-2">{r.reason}</td>
            <td className="py-2">
              <Badge tone={STATUS_TONES[r.status]}>{STATUS_LABELS[r.status]}</Badge>
              {r.decisionNote && (
                <div className="mt-1 text-zinc-500">&ldquo;{r.decisionNote}&rdquo;</div>
              )}
            </td>
            {actions && <td className="py-2">{actions(r)}</td>}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
