import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { DepartmentCount } from "@/bindings/DepartmentCount";
import type { PeriodCost } from "@/bindings/PeriodCost";
import { compactPesos, formatPesos } from "@/lib/money";
import { NO_DEPARTMENT } from "./labels";

// The charts are pictures of the tables beside them, so they are hidden from screen readers.

const short = (start: string) =>
  new Date(`${start}T00:00:00`).toLocaleDateString("en-US", { month: "short", day: "numeric" });

export function DepartmentChart({ rows }: { rows: DepartmentCount[] }) {
  const data = rows.map((r) => ({ name: r.department ?? NO_DEPARTMENT, count: r.count }));
  return (
    <div aria-hidden className="h-56 text-xs">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} layout="vertical" margin={{ left: 16, right: 16 }}>
          <CartesianGrid horizontal={false} strokeOpacity={0.3} />
          <XAxis type="number" allowDecimals={false} />
          <YAxis type="category" dataKey="name" width={110} />
          <Tooltip />
          <Bar dataKey="count" name="Employees" fill="currentColor" className="text-sky-600" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function PayrollCostChart({ rows }: { rows: PeriodCost[] }) {
  const data = rows.map((r) => ({ name: short(r.periodStart), cost: r.costCents }));
  return (
    <div aria-hidden className="h-56 text-xs">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ left: 16, right: 16 }}>
          <CartesianGrid strokeOpacity={0.3} />
          <XAxis dataKey="name" />
          <YAxis tickFormatter={compactPesos} width={60} />
          <Tooltip formatter={(c) => formatPesos(Number(c))} />
          <Line dataKey="cost" name="Cost" stroke="#0284c7" strokeWidth={2} />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
