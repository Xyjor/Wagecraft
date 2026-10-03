import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import type { Employee } from "@/bindings/Employee";
import { FormAlert } from "@/components/form";
import { Badge, quietButton } from "@/components/ui";
import { formatDate } from "@/lib/dates";
import { formatId } from "@/lib/govIds";
import type { AppError } from "@/lib/ipc";
import { archiveEmployee, getEmployee } from "./api";
import { CompensationTab } from "./CompensationTab";
import { CIVIL_STATUS_LABELS, fullName, SEX_LABELS, STATUS_LABELS } from "./format";

const TABS = ["Personal", "Employment", "Government IDs", "Compensation"] as const;
type Tab = (typeof TABS)[number];

const SEPARATED = ["RESIGNED", "TERMINATED"];

export function EmployeeProfilePage() {
  const id = Number(useParams().id);
  const [employee, setEmployee] = useState<Employee | null>(null);
  const [tab, setTab] = useState<Tab>("Personal");
  const [alert, setAlert] = useState<string>();

  const reload = useCallback(async () => setEmployee(await getEmployee(id)), [id]);

  useEffect(() => {
    getEmployee(id)
      .then(setEmployee)
      .catch((e: AppError) => setAlert(e.message));
  }, [id]);

  async function setArchived(archived: boolean) {
    setAlert(undefined);
    try {
      await archiveEmployee(id, archived);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  if (!employee) return <FormAlert message={alert} />;
  const e = employee;
  const archived = e.archivedAt !== null;
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
            <Badge tone={SEPARATED.includes(e.employmentStatus) ? "muted" : "good"}>
              {STATUS_LABELS[e.employmentStatus] ?? e.employmentStatus}
            </Badge>
            {archived && <Badge tone="muted">Archived</Badge>}
          </div>
        </div>
        <div className="flex gap-1">
          {!archived && (
            <Link to={`/employees/${e.id}/edit`} className={quietButton}>
              Edit
            </Link>
          )}
          {!archived && SEPARATED.includes(e.employmentStatus) && (
            <button type="button" className={quietButton} onClick={() => setArchived(true)}>
              Archive
            </button>
          )}
          {archived && (
            <button type="button" className={quietButton} onClick={() => setArchived(false)}>
              Unarchive
            </button>
          )}
        </div>
      </header>
      <FormAlert message={alert} />

      <div
        role="tablist"
        aria-label="Profile sections"
        className="flex gap-1 border-b border-zinc-200 dark:border-zinc-800"
      >
        {TABS.map((t) => (
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
        {tab === "Compensation" && <CompensationTab employee={e} />}
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
