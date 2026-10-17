import { useCallback, useEffect, useState } from "react";
import type { BackupEntry } from "@/bindings/BackupEntry";
import { FormAlert } from "@/components/form";
import { Badge, primaryButton as primary } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import type { AppError } from "@/lib/ipc";
import { backUpNow, listBackups } from "./api";
import { formatBytes, kindLabel, splitPath } from "./backups";
import { RestorePanel } from "./RestorePanel";
import { formatWhen } from "@/lib/dates";
import { PageHeader } from "@/components/PageHeader";
import { StatusMessage } from "@/components/Notice";

export function BackupsPage() {
  const { me } = useSession();
  if (me.role !== "ADMIN") {
    return <p className="text-ink-muted">Only Admins can manage backups.</p>;
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
      <PageHeader
        title="Backups"
        actions={
          <>
            <button type="button" className={primary} onClick={backUp} disabled={busy}>
              {busy ? "Backing up…" : "Back up now"}
            </button>
          </>
        }
      />
      <p className="max-w-prose text-sm text-ink-muted">
        Wagecraft backs up by itself once a day when it starts, and keeps the newest daily backups
        (14 unless changed in Settings, where you can also pick the folder). It also backs up before
        posting payroll. Use <strong>Back up now</strong> to save a copy to a USB drive or another
        folder, so a broken PC doesn&apos;t take your payroll with it.
      </p>

      <RestorePanel />

      <FormAlert message={alert} />
      <StatusMessage>{notice}</StatusMessage>

      {!backups && !alert && <p className="text-ink-faint">Loading backups…</p>}
      {backups?.length === 0 && <p className="text-ink-faint">No backups yet.</p>}
      {backups && backups.length > 0 && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-rule-strong bg-sunken text-xs text-ink-muted">
            <tr>
              <th className="py-2 pr-4 font-medium whitespace-nowrap">Kind</th>
              <th className="py-2 pr-4 font-medium whitespace-nowrap">Taken</th>
              <th className="py-2 pr-4 font-medium whitespace-nowrap">By</th>
              <th className="py-2 pr-4 text-right font-medium whitespace-nowrap">Size</th>
              <th className="w-full py-2 font-medium">File</th>
            </tr>
          </thead>
          <tbody>
            {backups.map((b) => {
              const { folder, name } = splitPath(b.path);
              return (
                <tr key={b.id} className="border-b border-rule hover:bg-sunken/60">
                  <th scope="row" className="py-2 pr-4 font-normal whitespace-nowrap">
                    {kindLabel(b.kind)}
                  </th>
                  <td className="py-2 pr-4 whitespace-nowrap">{formatWhen(b.createdAt)}</td>
                  <td className="py-2 pr-4 whitespace-nowrap text-ink-muted">
                    {b.createdByName ?? "Automatic"}
                  </td>
                  <td className="py-2 pr-4 text-right tabular-nums whitespace-nowrap">
                    {formatBytes(b.sizeBytes)}
                  </td>
                  <td className="px-3 py-2">
                    <span className="font-mono text-xs">
                      <span className="break-all text-ink-faint">{folder}</span>
                      <span className="whitespace-nowrap">{name}</span>
                    </span>{" "}
                    {!b.onDisk && <Badge tone="bad">File missing</Badge>}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </section>
  );
}
