import { Check } from "lucide-react";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import { STATUS_LABELS } from "./periods";

type Status = PayrollPeriod["status"];

const ORDER: Status[] = ["DRAFT", "COMPUTED", "APPROVED", "POSTED"];

/** What to do at each step, shown under the steps. */
const NEXT: Record<Status, string> = {
  DRAFT: "Compute the payroll to see everyone's pay.",
  COMPUTED:
    "Fix attendance, leave or pay rates at the source, then recompute. Approve when the numbers are right.",
  APPROVED:
    "Post it to make the payslips final, or send it back if something needs fixing before it's posted.",
  POSTED:
    "Posted. These payslips are final, staff can see them, and attendance for these days is locked.",
};

/**
 * Where a pay period is in Draft → Computed → Approved → Posted, and what to do next.
 * Posting is the last step, so a posted period shows every step done.
 */
export function PayrollSteps({ status }: { status: Status }) {
  const at = ORDER.indexOf(status);
  const finished = status === "POSTED";

  return (
    <div className="space-y-2.5">
      <ol aria-label="Payroll steps" className="flex max-w-2xl items-center">
        {ORDER.map((step, i) => {
          const done = i < at || finished;
          const current = i === at;
          return (
            <li
              key={step}
              aria-current={current ? "step" : undefined}
              className="flex flex-1 items-center gap-2 last:flex-none"
            >
              <span
                aria-hidden="true"
                className={`grid size-6 shrink-0 place-items-center rounded-full border text-xs font-semibold tabular-nums ${
                  done
                    ? "border-accent bg-accent text-accent-ink"
                    : current
                      ? "border-accent bg-accent-soft text-accent ring-2 ring-accent/25"
                      : "border-field bg-surface text-ink-faint"
                }`}
              >
                {done ? <Check className="size-3.5" strokeWidth={2.5} /> : i + 1}
              </span>
              <span
                data-label
                className={`text-sm whitespace-nowrap ${
                  current ? "font-semibold text-ink" : done ? "text-ink" : "text-ink-faint"
                }`}
              >
                {STATUS_LABELS[step]}
              </span>
              {done && <span className="sr-only">, done</span>}
              {i < ORDER.length - 1 && (
                <span
                  aria-hidden="true"
                  className={`mx-3 h-px flex-1 ${i < at ? "bg-accent" : "bg-rule-strong"}`}
                />
              )}
            </li>
          );
        })}
      </ol>
      <p className="text-sm text-ink-muted">{NEXT[status]}</p>
    </div>
  );
}
