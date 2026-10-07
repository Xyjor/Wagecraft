import { useCallback, useEffect, useState } from "react";
import type { BackupEntry } from "@/bindings/BackupEntry";
import { FormAlert } from "@/components/form";
import { Badge, primaryButton as primary } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import type { AppError } from "@/lib/ipc";
import { backUpNow, listBackups } from "./api";
import { formatBytes, kindLabel } from "./backups";
import { RestorePanel } from "./RestorePanel";

export function BackupsPage() {
  const { me } = useSession();
  if (me.role !== "ADMIN") {
    return <p className="text-zinc-600 dark:text-zinc-400">Only Admins can manage backups.</p>;
  }
  return <Backups />;
}

function Backups() {
  const [backups, setBackups] = useState<BackupEntry[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [busy, setBusy] = useState(false);

  const load = useCallback(
    () =>
      listBackups()
        .then(setBackups)
        .catch((e: AppError) => setAlert(e.message)),
    [],
  );

  useEffect(() => {
    void load();
  }, [load]);

  async function backUp() {
    setAlert(undefined);
    setNotice(undefined);
    setBusy(true);
    try {
      const path = await backUpNow();
      if (path) {
        setNotice(`Backup saved to ${path}`);
        await load();
      }
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Backups</h1>
        <button type="button" className={primary} onClick={backUp} disabled={busy}>
          {busy ? "Backing up…" : "Back up now"}
        </button>
      </div>
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        Wagecraft backs up by itself once a day when it starts, and keeps the newest daily backups
        (14 unless changed in Settings, where you can also pick the folder). It also backs up before
        posting payroll. Use <strong>Back up now</strong> to save a copy to a USB drive or another
        folder, so a broken PC doesn&apos;t take your payroll with it.
      </p>

      <RestorePanel />

      <FormAlert message={alert} />
      {notice && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
          {notice}
        </p>
      )}

      {!backups && !alert && <p className="text-zinc-500">Loading backups…</p>}
      {backups?.length === 0 && <p className="text-zinc-500">No backups yet.</p>}
      {backups && backups.length > 0 && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Kind</th>
              <th className="py-2 font-medium">Taken</th>
              <th className="py-2 font-medium">By</th>
              <th className="py-2 text-right font-medium">Size</th>
              <th className="py-2 pl-4 font-medium">File</th>
            </tr>
          </thead>
          <tbody>
            {backups.map((b) => (
              <tr key={b.id} className="border-b border-zinc-100 dark:border-zinc-900">
                <th scope="row" className="py-2 font-normal">
                  {kindLabel(b.kind)}
                </th>
                <td className="py-2">{formatWhen(b.createdAt)}</td>
                <td className="py-2 text-zinc-600 dark:text-zinc-400">
                  {b.createdByName ?? "Automatic"}
                </td>
                <td className="py-2 text-right tabular-nums">{formatBytes(b.sizeBytes)}</td>
                <td className="py-2 pl-4">
                  <span className="font-mono text-xs break-all">{b.path}</span>{" "}
                  {!b.onDisk && <Badge tone="bad">File missing</Badge>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function formatWhen(iso: string) {
  return new Date(iso).toLocaleString("en-PH", { dateStyle: "medium", timeStyle: "short" });
}
