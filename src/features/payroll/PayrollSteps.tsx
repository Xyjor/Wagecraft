import type { ReactNode } from "react";
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
 * Where a pay period is in Draft → Computed → Approved → Posted, ruled like the column heads of
 * a ledger page, with what to do next and the buttons for it underneath. Posting is the last
 * step, so a posted period shows every step done.
 */
export function PayrollSteps({ status, children }: { status: Status; children?: ReactNode }) {
  const at = ORDER.indexOf(status);
  const finished = status === "POSTED";

  return (
    <section
      aria-label="Where this payroll is"
      className="rounded-sm border border-rule-strong bg-surface"
    >
      <ol
        aria-label="Payroll steps"
        className="grid grid-cols-4 divide-x divide-rule border-b-[3px] border-double border-rule-strong"
      >
        {ORDER.map((step, i) => {
          const done = i < at || finished;
          const current = i === at;
          return (
            <li
              key={step}
              aria-current={current ? "step" : undefined}
              className={`flex items-center gap-2 px-4 py-2.5 text-sm ${
                current ? "bg-accent-soft shadow-[inset_0_3px_0_var(--wc-accent)]" : ""
              }`}
            >
              <span aria-hidden="true" className="text-xs text-ink-faint tabular-nums">
                {i + 1}
              </span>
              <span
                data-label
                className={
                  current ? "font-semibold text-ink" : done ? "text-ink" : "text-ink-faint"
                }
              >
                {STATUS_LABELS[step]}
              </span>
              {done && (
                <>
                  <Check
                    aria-hidden="true"
                    className="ml-auto size-4 text-accent"
                    strokeWidth={2.5}
                  />
                  <span className="sr-only">, done</span>
                </>
              )}
            </li>
          );
        })}
      </ol>
      <div className="space-y-3 px-4 py-3">
        <p className="text-sm text-ink-muted">{NEXT[status]}</p>
        {children}
      </div>
    </section>
  );
}
