import { useEffect, useState } from "react";
import { Link } from "react-router";
import type { Archived } from "@/bindings/Archived";
import type { Department } from "@/bindings/Department";
import type { EmployeePage } from "@/bindings/EmployeePage";
import type { EmployeeQuery } from "@/bindings/EmployeeQuery";
import type { EmployeeSort } from "@/bindings/EmployeeSort";
import { FormAlert } from "@/components/form";
import { Pager } from "@/components/Pager";
import { Badge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { listDepartments } from "@/features/org/api";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { exportMasterlist, listEmployees } from "./api";
import { listName, STATUS_LABELS } from "./format";

const PAGE_SIZE = 25;
const SEARCH_DELAY_MS = 250;

const FIRST_PAGE: EmployeeQuery = {
  search: null,
  departmentId: null,
  positionId: null,
  employmentStatus: null,
  archived: "exclude",
  sort: "name",
  descending: false,
  page: 1,
  pageSize: PAGE_SIZE,
};

const filterClass =
  "h-9 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900";

export function EmployeesPage() {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Only Admin and HR can see the employee list.
      </p>
    );
  }
  return <EmployeeList />;
}

function EmployeeList() {
  const [query, setQuery] = useState<EmployeeQuery>(FIRST_PAGE);
  const [searchText, setSearchText] = useState("");
  const [result, setResult] = useState<EmployeePage | null>(null);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [alert, setAlert] = useState<string>();
  const [exportNote, setExportNote] = useState<string>();
  const [exporting, setExporting] = useState(false);

  async function exportCsv() {
    setAlert(undefined);
    setExportNote(undefined);
    setExporting(true);
    try {
      const saved = await exportMasterlist(query);
      if (saved) setExportNote(`Saved the masterlist to ${saved}`);
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setExporting(false);
    }
  }

  /** Changes filters or sort, and goes back to page 1. */
  const refine = (change: Partial<EmployeeQuery>) =>
    setQuery((q) => ({ ...q, ...change, page: 1 }));

  useEffect(() => {
    listDepartments()
      .then(setDepartments)
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  // Wait for a pause in typing before searching.
  useEffect(() => {
    const search = searchText.trim() || null;
    const timer = setTimeout(
      () => setQuery((q) => (q.search === search ? q : { ...q, search, page: 1 })),
      SEARCH_DELAY_MS,
    );
    return () => clearTimeout(timer);
  }, [searchText]);

  useEffect(() => {
    let live = true;
    listEmployees(query)
      .then((page) => live && setResult(page))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [query]);

  function sortBy(sort: EmployeeSort) {
    refine({ sort, descending: query.sort === sort ? !query.descending : false });
  }

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Employees</h1>
        <div className="flex gap-2">
          <button
            type="button"
            className={`${quietButton} border border-zinc-300 dark:border-zinc-700`}
            disabled={exporting}
            onClick={exportCsv}
          >
            Export CSV
          </button>
          <Link to="/employees/new" className={primaryButton}>
            Add employee
          </Link>
        </div>
      </div>
      <FormAlert message={alert} />
      {exportNote && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-300">
          {exportNote}
        </p>
      )}

      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Search</span>
          <input
            type="search"
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            placeholder="Number, name or email"
            className={`${filterClass} w-64`}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Department</span>
          <select
            className={filterClass}
            value={query.departmentId ?? ""}
            onChange={(e) => refine({ departmentId: Number(e.target.value) || null })}
          >
            <option value="">All departments</option>
            {departments.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Status</span>
          <select
            className={filterClass}
            value={query.employmentStatus ?? ""}
            onChange={(e) => refine({ employmentStatus: e.target.value || null })}
          >
            <option value="">All statuses</option>
            {Object.entries(STATUS_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Show</span>
          <select
            className={filterClass}
            value={query.archived}
            onChange={(e) => refine({ archived: e.target.value as Archived })}
          >
            <option value="exclude">Current employees</option>
            <option value="only">Archived only</option>
            <option value="include">Everyone</option>
          </select>
        </label>
      </div>

      {result && (
        <>
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <SortHeader label="Employee no." sort="employeeNo" query={query} onSort={sortBy} />
                <SortHeader label="Name" sort="name" query={query} onSort={sortBy} />
                <SortHeader label="Department" sort="department" query={query} onSort={sortBy} />
                <th className="py-2 font-medium">Position</th>
                <th className="py-2 font-medium">Status</th>
                <SortHeader label="Hire date" sort="hireDate" query={query} onSort={sortBy} />
              </tr>
            </thead>
            <tbody>
              {result.items.length === 0 && (
                <tr>
                  <td colSpan={6} className="py-3 text-zinc-500">
                    {isFiltered(query)
                      ? "No employees match these filters."
                      : "No employees yet. Use Add employee to start the list."}
                  </td>
                </tr>
              )}
              {result.items.map((e) => (
                <tr key={e.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2 tabular-nums">{e.employeeNo}</td>
                  <td className="py-2">
                    <Link to={`/employees/${e.id}`} className="font-medium hover:underline">
                      {listName(e)}
                    </Link>
                  </td>
                  <td className="py-2">{e.departmentName}</td>
                  <td className="py-2">{e.positionTitle}</td>
                  <td className="py-2">
                    <div className="flex gap-1">
                      <span>{STATUS_LABELS[e.employmentStatus] ?? e.employmentStatus}</span>
                      {e.archived && <Badge tone="muted">Archived</Badge>}
                    </div>
                  </td>
                  <td className="py-2 tabular-nums">{formatDate(e.hireDate)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <Pager
            page={result.page}
            pageSize={result.pageSize}
            total={result.total}
            onPage={(page) => setQuery((q) => ({ ...q, page }))}
          />
        </>
      )}
    </section>
  );
}

function SortHeader(props: {
  label: string;
  sort: EmployeeSort;
  query: EmployeeQuery;
  onSort: (sort: EmployeeSort) => void;
}) {
  const active = props.query.sort === props.sort;
  const direction = props.query.descending ? "descending" : "ascending";
  return (
    <th className="py-2 font-medium" aria-sort={active ? direction : undefined}>
      <button
        type="button"
        onClick={() => props.onSort(props.sort)}
        className="hover:text-zinc-900 dark:hover:text-zinc-100"
      >
        {props.label}
        {active && <span aria-hidden="true">{props.query.descending ? " ↓" : " ↑"}</span>}
      </button>
    </th>
  );
}

function isFiltered(q: EmployeeQuery): boolean {
  return (
    Boolean(q.search || q.departmentId || q.positionId || q.employmentStatus) ||
    q.archived !== "exclude"
  );
}
