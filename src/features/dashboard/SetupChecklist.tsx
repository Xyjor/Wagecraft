import { useEffect, useId, useState } from "react";
import { Link } from "react-router";
import { Check, X } from "lucide-react";
import { quietButton } from "@/components/ui";
import { listEmployees } from "@/features/employees/api";
import { listDepartments, listHolidays, listPositions, listSchedules } from "@/features/org/api";
import { listPeriods } from "@/features/payroll/api";
import { getSettings } from "@/features/settings/api";
import { setupSteps, type SetupStep } from "./setupSteps";

export const SETUP_HIDDEN_KEY = "wagecraft.setupChecklist.hidden";

function readHidden() {
  try {
    return localStorage.getItem(SETUP_HIDDEN_KEY) === "1";
  } catch {
    return false;
  }
}

/**
 * The Admin's first-run checklist on the dashboard. It disappears once every step is done, if
 * it can't load, or when the Admin hides it.
 */
export function SetupChecklist({ year }: { year: number }) {
  const [hidden, setHidden] = useState(readHidden);
  const [steps, setSteps] = useState<SetupStep[] | null>(null);
  const headingId = useId();

  useEffect(() => {
    if (hidden) return;
    let live = true;
    Promise.all([
      getSettings(),
      listDepartments(),
      listPositions(),
      listSchedules(),
      listHolidays(year),
      listEmployees({
        search: null,
        departmentId: null,
        positionId: null,
        employmentStatus: null,
        archived: "exclude",
        sort: "name",
        descending: false,
        page: 1,
        pageSize: 1,
      }),
      listPeriods(),
    ])
      .then(([settings, departments, positions, schedules, holidays, employees, periods]) => {
        if (!live) return;
        setSteps(
          setupSteps(
            {
              settings,
              departments,
              positions,
              schedules,
              holidays,
              employeeCount: employees.total,
              periods,
            },
            year,
          ),
        );
      })
      // The checklist is a convenience: if it can't load, the dashboard carries on without it.
      .catch(() => live && setSteps([]));
    return () => {
      live = false;
    };
  }, [hidden, year]);

  function hide() {
    setHidden(true);
    try {
      localStorage.setItem(SETUP_HIDDEN_KEY, "1");
    } catch {
      // Storage can be unavailable; it stays hidden until the app closes.
    }
  }

  if (hidden || !steps || steps.length === 0) return null;
  const doneCount = steps.filter((s) => s.done).length;
  if (doneCount === steps.length) return null;

  return (
    <section
      aria-labelledby={headingId}
      className="mb-6 rounded-lg border border-accent/30 bg-surface p-4"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id={headingId} className="font-semibold">
            Finish setting up
          </h2>
          <p className="text-sm text-ink-muted">
            {doneCount} of {steps.length} done
          </p>
        </div>
        <button
          type="button"
          onClick={hide}
          className={quietButton}
          aria-label="Hide setup checklist"
          title="Hide"
        >
          <X aria-hidden="true" className="size-4" />
        </button>
      </div>
      <div
        aria-hidden="true"
        className="mt-3 h-1.5 overflow-hidden rounded-full bg-sunken"
        title={`${doneCount} of ${steps.length} done`}
      >
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-300 ease-out motion-reduce:transition-none"
          style={{ width: `${(doneCount / steps.length) * 100}%` }}
        />
      </div>
      <ol className="mt-4 grid gap-x-6 gap-y-1 sm:grid-cols-2">
        {steps.map((s) => (
          <li key={s.id} className="flex items-start gap-2.5 py-1">
            <span
              aria-hidden="true"
              className={`mt-0.5 grid size-5 shrink-0 place-items-center rounded-full border ${
                s.done ? "border-accent bg-accent text-accent-ink" : "border-field bg-surface"
              }`}
            >
              {s.done && <Check className="size-3" strokeWidth={3} />}
            </span>
            <div className="min-w-0 text-sm">
              {s.done ? (
                <span className="text-ink-faint">{s.label}</span>
              ) : (
                <Link
                  to={s.to}
                  className="font-medium text-accent underline-offset-2 hover:underline"
                >
                  {s.label}
                </Link>
              )}
              {s.done ? (
                <span className="sr-only">, done</span>
              ) : (
                <p className="text-ink-muted">{s.hint}</p>
              )}
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
