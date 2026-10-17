import type { ReactNode } from "react";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import { formatDays } from "./format";

/** A year of leave balances. `actions` adds a cell per row, such as HR's Adjust button. */
export function BalanceTable({
  balances,
  empty,
  actions,
}: {
  balances: LeaveBalance[];
  empty: string;
  actions?: (b: LeaveBalance) => ReactNode;
}) {
  return (
    <table className="w-full max-w-2xl text-left text-sm">
      <thead className="border-b border-rule-strong bg-sunken text-xs text-ink-muted">
        <tr>
          <th className="px-3 py-2 font-medium">Leave</th>
          <th className="px-3 py-2 font-medium">For the year</th>
          <th className="px-3 py-2 font-medium">Used</th>
          <th className="px-3 py-2 font-medium">Left</th>
          {actions && (
            <th className="px-3 py-2 font-medium">
              <span className="sr-only">Actions</span>
            </th>
          )}
        </tr>
      </thead>
      <tbody>
        {balances.length === 0 && (
          <tr>
            <td colSpan={actions ? 5 : 4} className="px-3 py-3 text-ink-muted">
              {empty}
            </td>
          </tr>
        )}
        {balances.map((b) => (
          <tr key={b.id} className="border-b border-rule align-top hover:bg-sunken/60">
            <td className="px-3 py-2 font-medium">{b.leaveTypeName}</td>
            <td className="px-3 py-2 tabular-nums">{formatDays(b.entitledHalfdays)}</td>
            <td className="px-3 py-2 tabular-nums">{formatDays(b.usedHalfdays)}</td>
            <td className="py-2 font-medium tabular-nums">
              {formatDays(b.entitledHalfdays - b.usedHalfdays)}
            </td>
            {actions && <td className="px-3 py-2">{actions(b)}</td>}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
