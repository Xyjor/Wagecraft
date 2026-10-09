import { useEffect, useState } from "react";
import { Link } from "react-router";
import type { AuditEntry } from "@/bindings/AuditEntry";
import type { AuditFilters } from "@/bindings/AuditFilters";
import type { AuditPage } from "@/bindings/AuditPage";
import type { AuditQuery } from "@/bindings/AuditQuery";
import { FormAlert } from "@/components/form";
import { Pager } from "@/components/Pager";
import { quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formatWhen } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { activityFilters, exportActivity, listActivity } from "./api";
import { changes } from "./diff";
import { groupByArea, recordLabel, recordLink } from "./format";

const PAGE_SIZE = 50;

const FIRST_PAGE: AuditQuery = {
  from: null,
  to: null,
  actor: null,
  action: null,
  entityType: null,
  entityId: null,
  page: 1,
  pageSize: PAGE_SIZE,
};

const NO_FILTERS: AuditFilters = { actors: [], actions: [], entityTypes: [] };

const filterClass =
  "h-9 rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900";

/** The audit log for Admin (plan §6.8): who did what, when, and what changed. */
export function ActivityPage() {
  const { me } = useSession();
  if (me.role !== "ADMIN") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">Only Admins can see the activity log.</p>
    );
  }
  return <ActivityLog />;
}

function ActivityLog() {
  const [query, setQuery] = useState<AuditQuery>(FIRST_PAGE);
  const [result, setResult] = useState<AuditPage | null>(null);
  const [filters, setFilters] = useState<AuditFilters>(NO_FILTERS);
  const [alert, setAlert] = useState<string>();
  const [exportNote, setExportNote] = useState<string>();
  const [exporting, setExporting] = useState(false);

  /** Changes filters and goes back to page 1. */
  const refine = (change: Partial<AuditQuery>) => setQuery((q) => ({ ...q, ...change, page: 1 }));

  useEffect(() => {
    activityFilters()
      .then(setFilters)
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  useEffect(() => {
    let live = true;
    listActivity(query)
      .then((page) => {
        if (!live) return;
        setResult(page);
        setAlert(undefined);
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [query]);

  async function exportCsv() {
    setAlert(undefined);
    setExportNote(undefined);
    setExporting(true);
    try {
      const saved = await exportActivity(query);
      if (saved) setExportNote(`Saved the activity log to ${saved}`);
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setExporting(false);
    }
  }

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Activity</h1>
        <button
          type="button"
          className={`${quietButton} border border-zinc-300 dark:border-zinc-700`}
          disabled={exporting}
          onClick={exportCsv}
        >
          Export CSV
        </button>
      </div>
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        Every sign-in and every change, newest first. Nothing here can be edited or deleted.
      </p>
      <FormAlert message={alert} />
      {exportNote && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-300">
          {exportNote}
        </p>
      )}

      <Filters query={query} options={filters} onChange={refine} />

      {result && (
        <>
          <Entries entries={result.items} filtered={isFiltered(query)} />
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

function Filters({
  query,
  options,
  onChange,
}: {
  query: AuditQuery;
  options: AuditFilters;
  onChange: (change: Partial<AuditQuery>) => void;
}) {
  return (
    <div className="flex flex-wrap items-end gap-3">
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">From</span>
        <input
          type="date"
          className={filterClass}
          value={query.from ?? ""}
          onChange={(e) => onChange({ from: e.target.value || null })}
        />
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">To</span>
        <input
          type="date"
          className={filterClass}
          value={query.to ?? ""}
          onChange={(e) => onChange({ to: e.target.value || null })}
        />
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">User</span>
        <select
          className={filterClass}
          value={query.actor ?? ""}
          onChange={(e) => onChange({ actor: e.target.value || null })}
        >
          <option value="">Everyone</option>
          {options.actors.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">Action</span>
        <select
          className={filterClass}
          value={query.action ?? ""}
          onChange={(e) => onChange({ action: e.target.value || null })}
        >
          <option value="">All actions</option>
          {groupByArea(options.actions).map((g) => (
            <optgroup key={g.area} label={g.area}>
              <option value={`${g.area}.`}>All {g.area} actions</option>
              {g.actions.map((a) => (
                <option key={a} value={a}>
                  {a}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">Record</span>
        <select
          className={filterClass}
          value={query.entityType ?? ""}
          onChange={(e) => onChange({ entityType: e.target.value || null })}
        >
          <option value="">Any record</option>
          {options.entityTypes.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="font-medium">Record ID</span>
        <input
          type="number"
          min={1}
          className={`${filterClass} w-28`}
          value={query.entityId ?? ""}
          onChange={(e) => onChange({ entityId: Number(e.target.value) || null })}
        />
      </label>
    </div>
  );
}

function isFiltered(q: AuditQuery): boolean {
  return Boolean(q.from || q.to || q.actor || q.action || q.entityType || q.entityId);
}

function Entries({ entries, filtered }: { entries: AuditEntry[]; filtered: boolean }) {
  const [open, setOpen] = useState<number | null>(null);

  if (entries.length === 0) {
    return (
      <p className="text-zinc-500">
        {filtered ? "No activity matches these filters." : "Nothing recorded yet."}
      </p>
    );
  }
  return (
    <table className="w-full text-left text-sm">
      <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
        <tr>
          <th className="py-2 font-medium">When</th>
          <th className="py-2 font-medium">User</th>
          <th className="py-2 font-medium">Action</th>
          <th className="py-2 font-medium">Record</th>
          <th className="py-2">
            <span className="sr-only">Details</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {entries.map((e) => (
          <EntryRows
            key={e.id}
            entry={e}
            open={open === e.id}
            onToggle={() => setOpen(open === e.id ? null : e.id)}
          />
        ))}
      </tbody>
    </table>
  );
}

function EntryRows({
  entry,
  open,
  onToggle,
}: {
  entry: AuditEntry;
  open: boolean;
  onToggle: () => void;
}) {
  const record = recordLabel(entry.entityType, entry.entityId);
  const link = recordLink(entry.entityType, entry.entityId);
  return (
    <>
      <tr className="border-b border-zinc-100 dark:border-zinc-900">
        <td className="py-2 whitespace-nowrap tabular-nums">
          <time dateTime={entry.at}>{formatWhen(entry.at)}</time>
        </td>
        <td className="py-2">{entry.actorUsername}</td>
        <td className="py-2 font-mono text-xs">{entry.action}</td>
        <td className="py-2">
          {link ? (
            <Link to={link} className="hover:underline">
              {record}
            </Link>
          ) : (
            record
          )}
        </td>
        <td className="py-2 text-right">
          <button
            type="button"
            className={quietButton}
            aria-label={`${open ? "Hide" : "View"} details of ${entry.action}`}
            onClick={onToggle}
          >
            {open ? "Hide" : "View"}
          </button>
        </td>
      </tr>
      {open && (
        <tr className="border-b border-zinc-100 dark:border-zinc-900">
          <td colSpan={5} className="py-2 pl-4">
            <Details entry={entry} record={record} />
          </td>
        </tr>
      )}
    </>
  );
}

function Details({ entry, record }: { entry: AuditEntry; record: string }) {
  const rows = changes(entry.before, entry.after);
  const name = record ? `Changes to ${record}` : `Details of ${entry.action}`;
  if (rows.length === 0) {
    return <p className="text-sm text-zinc-500">Nothing was recorded besides the action.</p>;
  }
  return (
    <table aria-label={name} className="w-full max-w-3xl text-left text-sm">
      <thead className="text-zinc-500">
        <tr>
          <th className="py-1 font-medium">Field</th>
          <th className="py-1 font-medium">Before</th>
          <th className="py-1 font-medium">After</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((c) => (
          <tr key={c.field} className="align-top">
            <th scope="row" className="py-1 pr-4 font-mono text-xs font-normal">
              {c.field}
            </th>
            <td className="py-1 pr-4 break-all text-zinc-600 dark:text-zinc-400">
              {c.before ?? "—"}
            </td>
            <td className="py-1 break-all">{c.after ?? "—"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
