import { useState, type ReactNode } from "react";
import type { Employee } from "@/bindings/Employee";
import { FormAlert } from "@/components/form";
import { PageHeader, type Crumb } from "@/components/PageHeader";
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
  crumbs,
  extraTabs = [],
}: {
  employee: Employee;
  actions?: ReactNode;
  alert?: string;
  /** Breadcrumbs back to the list, on HR's page. */
  crumbs?: Crumb[];
  extraTabs?: ExtraTab[];
}) {
  const tabs = ["Personal", "Employment", "Government IDs", ...extraTabs.map((t) => t.name)];
  const [tab, setTab] = useState(tabs[0]);
  const role = [e.positionTitle, e.departmentName].filter(Boolean).join(", ");

  return (
    <article className="max-w-5xl space-y-6">
      <PageHeader
        title={fullName(e)}
        crumbs={crumbs}
        description={[e.employeeNo, role].filter(Boolean).join(" · ")}
        meta={
          <>
            <Badge tone={isSeparated(e) ? "muted" : "good"}>
              {STATUS_LABELS[e.employmentStatus] ?? e.employmentStatus}
            </Badge>
            {e.archivedAt !== null && <Badge tone="muted">Archived</Badge>}
          </>
        }
        actions={actions}
      />
      <FormAlert message={alert} />

      <div
        role="tablist"
        aria-label="Profile sections"
        className="flex overflow-x-auto overflow-y-hidden border-b border-rule"
      >
        {tabs.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className="-mb-px border-b-2 border-transparent px-2 py-2 text-sm whitespace-nowrap text-ink-muted hover:text-ink aria-selected:border-accent aria-selected:font-medium aria-selected:text-ink"
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
          <dt className="text-sm text-ink-faint">{label}</dt>
          <dd className="text-sm">{value || <span className="text-ink-faint">Not set</span>}</dd>
        </div>
      ))}
    </dl>
  );
}
