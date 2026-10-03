import { useState, type ReactNode } from "react";
import type { Employee } from "@/bindings/Employee";
import { FormAlert } from "@/components/form";
import { Badge } from "@/components/ui";
import { formatDate } from "@/lib/dates";
import { formatId } from "@/lib/govIds";
import { CIVIL_STATUS_LABELS, fullName, isSeparated, SEX_LABELS, STATUS_LABELS } from "./format";

/** Extra tabs a page adds after the three every profile has. */
export type ExtraTab = { name: string; content: ReactNode };

/**
 * An employee's header and tabbed details. HR's profile page and Staff's own "My profile"
 * both use it; each passes its own actions and extra tabs.
 */
export function ProfileView({
  employee: e,
  actions,
  alert,
  extraTabs = [],
}: {
  employee: Employee;
  actions?: ReactNode;
  alert?: string;
  extraTabs?: ExtraTab[];
}) {
  const tabs = ["Personal", "Employment", "Government IDs", ...extraTabs.map((t) => t.name)];
  const [tab, setTab] = useState(tabs[0]);
  const role = [e.positionTitle, e.departmentName].filter(Boolean).join(", ");

  return (
    <article className="max-w-3xl space-y-6">
      <header className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">{fullName(e)}</h1>
          <p className="text-zinc-600 dark:text-zinc-400">
            {[e.employeeNo, role].filter(Boolean).join(" · ")}
          </p>
          <div className="flex gap-1">
            <Badge tone={isSeparated(e) ? "muted" : "good"}>
              {STATUS_LABELS[e.employmentStatus] ?? e.employmentStatus}
            </Badge>
            {e.archivedAt !== null && <Badge tone="muted">Archived</Badge>}
          </div>
        </div>
        {actions && <div className="flex gap-1">{actions}</div>}
      </header>
      <FormAlert message={alert} />

      <div
        role="tablist"
        aria-label="Profile sections"
        className="flex gap-1 border-b border-zinc-200 dark:border-zinc-800"
      >
        {tabs.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className="-mb-px border-b-2 border-transparent px-3 py-2 text-sm text-zinc-600 aria-selected:border-zinc-900 aria-selected:font-medium aria-selected:text-zinc-900 dark:text-zinc-400 dark:aria-selected:border-zinc-100 dark:aria-selected:text-zinc-100"
          >
            {t}
          </button>
        ))}
      </div>

      <div role="tabpanel" aria-label={tab}>
        {tab === "Personal" && (
          <Details
            rows={[
              ["Birth date", formatDate(e.birthDate)],
              ["Sex", e.sex && SEX_LABELS[e.sex]],
              ["Civil status", e.civilStatus && CIVIL_STATUS_LABELS[e.civilStatus]],
              ["Email", e.email],
              ["Mobile", e.mobile],
              ["Address", e.address],
            ]}
          />
        )}
        {tab === "Employment" && (
          <Details
            rows={[
              ["Department", e.departmentName],
              ["Position", e.positionTitle],
              ["Hire date", formatDate(e.hireDate)],
              ["Regularization date", formatDate(e.regularizationDate)],
              ["Separation date", formatDate(e.separationDate)],
              ["Kiosk PIN", e.hasKioskPin ? "Set" : "Not set"],
            ]}
          />
        )}
        {tab === "Government IDs" && (
          <Details
            rows={[
              ["TIN", formatId("tin", e.tin)],
              ["SSS no.", formatId("sss", e.sssNo)],
              ["PhilHealth no.", formatId("philhealth", e.philhealthNo)],
              ["Pag-IBIG MID", formatId("pagibig", e.pagibigNo)],
              ["Bank", e.bankName],
              ["Account no.", e.bankAccountNo],
            ]}
          />
        )}
        {extraTabs.find((t) => t.name === tab)?.content}
      </div>
    </article>
  );
}

function Details({ rows }: { rows: [label: string, value: string | null | undefined][] }) {
  return (
    <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-[12rem_1fr]">
      {rows.map(([label, value]) => (
        <div key={label} className="contents">
          <dt className="text-sm text-zinc-500">{label}</dt>
          <dd className="text-sm">{value || <span className="text-zinc-400">Not set</span>}</dd>
        </div>
      ))}
    </dl>
  );
}
