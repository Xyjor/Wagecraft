import { useSyncExternalStore } from "react";
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
// Colours come from the Ledger tokens. SVG attributes can't be relied on to resolve CSS
// variables, so the charts read the current values and re-read them when the theme changes.

const TOKENS = ["chart", "rule", "rule-strong", "ink", "ink-muted", "surface", "sunken"] as const;
type Colors = Record<(typeof TOKENS)[number], string>;

let cached: { key: string; colors: Colors } | null = null;

function readColors(): Colors {
  const key = document.documentElement.className;
  if (cached?.key === key) return cached.colors;
  const style = getComputedStyle(document.documentElement);
  const colors = Object.fromEntries(
    TOKENS.map((t) => [t, style.getPropertyValue(`--wc-${t}`).trim()]),
  ) as Colors;
  cached = { key, colors };
  return colors;
}

function subscribe(onChange: () => void) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
  return () => observer.disconnect();
}

function useChartColors() {
  return useSyncExternalStore(subscribe, readColors);
}

const short = (start: string) =>
  new Date(`${start}T00:00:00`).toLocaleDateString("en-US", { month: "short", day: "numeric" });

function axis(c: Colors) {
  return {
    tick: { fill: c["ink-muted"], fontSize: 12 },
    axisLine: { stroke: c["rule-strong"] },
    tickLine: { stroke: c["rule-strong"] },
  };
}

function tooltip(c: Colors) {
  return {
    contentStyle: {
      background: c.surface,
      border: `1px solid ${c.rule}`,
      borderRadius: 6,
      color: c.ink,
      fontSize: 12,
    },
    labelStyle: { color: c.ink, fontWeight: 600 },
    itemStyle: { color: c.ink },
  };
}

export function DepartmentChart({ rows }: { rows: DepartmentCount[] }) {
  const c = useChartColors();
  const data = rows.map((r) => ({ name: r.department ?? NO_DEPARTMENT, count: r.count }));
  return (
    <div aria-hidden className="h-56 text-xs">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} layout="vertical" margin={{ left: 16, right: 16 }} barCategoryGap={6}>
          <CartesianGrid horizontal={false} stroke={c.rule} />
          <XAxis type="number" allowDecimals={false} {...axis(c)} />
          <YAxis type="category" dataKey="name" width={110} {...axis(c)} />
          <Tooltip {...tooltip(c)} cursor={{ fill: c.sunken }} />
          <Bar dataKey="count" name="Employees" fill={c.chart} radius={[0, 4, 4, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function PayrollCostChart({ rows }: { rows: PeriodCost[] }) {
  const c = useChartColors();
  const data = rows.map((r) => ({ name: short(r.periodStart), cost: r.costCents }));
  return (
    <div aria-hidden className="h-56 text-xs">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ left: 16, right: 16, top: 8 }}>
          <CartesianGrid vertical={false} stroke={c.rule} />
          <XAxis dataKey="name" {...axis(c)} />
          <YAxis tickFormatter={compactPesos} width={60} {...axis(c)} />
          <Tooltip
            {...tooltip(c)}
            formatter={(v) => formatPesos(Number(v))}
            cursor={{ stroke: c["rule-strong"] }}
          />
          <Line
            dataKey="cost"
            name="Cost"
            stroke={c.chart}
            strokeWidth={2}
            dot={{ r: 4, fill: c.chart, stroke: c.surface, strokeWidth: 2 }}
            activeDot={{ r: 5, fill: c.chart, stroke: c.surface, strokeWidth: 2 }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
