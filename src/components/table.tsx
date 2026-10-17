// Ledger tables: a sunken header row, ruled lines, money right-aligned in tabular figures with a
// faint column rule, and a double rule above totals. Class strings rather than a component, so
// pages keep their plain <table> markup.

export const table = "w-full text-left text-sm";
export const thead = "border-b border-rule-strong bg-sunken text-xs font-medium text-ink-muted";
export const th = "px-3 py-2 font-medium";
export const td = "px-3 py-2";
export const row = "border-b border-rule hover:bg-sunken/60";
/** A money or count column. */
export const num = "border-l border-rule/70 text-right tabular-nums";
/** The totals row. */
export const totalRow = "border-t-[3px] border-double border-rule-strong font-semibold";

/** The single row a table shows when it has nothing to list. */
export function EmptyRow({ colSpan, children }: { colSpan: number; children: string }) {
  return (
    <tr>
      <td colSpan={colSpan} className="px-3 py-6 text-center text-sm text-ink-muted">
        {children}
      </td>
    </tr>
  );
}
