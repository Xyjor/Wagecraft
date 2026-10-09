import { useId, useState } from "react";
import type { RestorePreview } from "@/bindings/RestorePreview";
import { FormAlert } from "@/components/form";
import { primaryButton as primary, quietButton as quiet } from "@/components/ui";
import { formatDate, formatWhen } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { chooseRestore, restoreBackup } from "./api";

const CONFIRM = "RESTORE";

/** Restore (plan §6.7): pick a backup, see what it holds, type RESTORE, and the app restarts. */
export function RestorePanel() {
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  const [typed, setTyped] = useState("");
  const [alert, setAlert] = useState<string>();
  const [restoring, setRestoring] = useState(false);
  const headingId = useId();
  const boxId = useId();

  async function choose() {
    setAlert(undefined);
    setTyped("");
    try {
      setPreview(await chooseRestore());
    } catch (e) {
      setPreview(null);
      setAlert((e as AppError).message);
    }
  }

  async function restore() {
    setAlert(undefined);
    setRestoring(true);
    try {
      await restoreBackup();
    } catch (e) {
      setRestoring(false);
      setAlert((e as AppError).message);
    }
  }

  return (
    <div className="space-y-3">
      {!preview && (
        <button type="button" className={quiet} onClick={choose}>
          Restore a backup…
        </button>
      )}
      <FormAlert message={alert} />
      {preview && (
        <section
          aria-labelledby={headingId}
          className="max-w-prose space-y-3 rounded-md border border-amber-300 bg-amber-50 p-4 text-sm dark:border-amber-800 dark:bg-amber-950"
        >
          <h2 id={headingId} className="text-base font-semibold">
            Restore this backup?
          </h2>
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt className="text-zinc-600 dark:text-zinc-400">File</dt>
            <dd className="font-mono text-xs break-all">{preview.fileName}</dd>
            <dt className="text-zinc-600 dark:text-zinc-400">Saved</dt>
            <dd>{preview.savedAt ? formatWhen(preview.savedAt) : "Unknown"}</dd>
            <dt className="text-zinc-600 dark:text-zinc-400">Holds</dt>
            <dd>
              {preview.employeeCount} {preview.employeeCount === 1 ? "employee" : "employees"},{" "}
              {lastPosted(preview)}
            </dd>
          </dl>
          <p>
            Everything entered after this backup will be lost. Wagecraft backs up the current data
            first, then restarts, and everyone signs in again.
          </p>
          {restoring ? (
            <p role="status">Restoring… Wagecraft will restart in a moment.</p>
          ) : (
            <div className="flex flex-wrap items-end gap-2">
              <label htmlFor={boxId} className="flex flex-col gap-1">
                <span>Type {CONFIRM} to confirm</span>
                <input
                  id={boxId}
                  value={typed}
                  onChange={(e) => setTyped(e.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                  className="rounded-md border border-zinc-300 bg-white px-2 py-1 font-mono dark:border-zinc-700 dark:bg-zinc-900"
                />
              </label>
              <button
                type="button"
                className={primary}
                disabled={typed !== CONFIRM}
                onClick={restore}
              >
                Restore and restart
              </button>
              <button type="button" className={quiet} onClick={() => setPreview(null)}>
                Cancel
              </button>
            </div>
          )}
        </section>
      )}
    </div>
  );
}

function lastPosted(p: RestorePreview) {
  if (!p.lastPostedStart || !p.lastPostedEnd) return "No payroll posted yet";
  return `last posted payroll ${formatDate(p.lastPostedStart)} to ${formatDate(p.lastPostedEnd)}`;
}
