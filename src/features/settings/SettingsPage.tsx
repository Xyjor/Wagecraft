import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import type { Settings } from "@/bindings/Settings";
import { Field, FormAlert } from "@/components/form";
import { primaryButton as primary, quietButton as quiet } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { fieldErrors } from "@/features/auth/validation";
import { formValues, serverErrors } from "@/lib/formData";
import { formatId } from "@/lib/govIds";
import type { AppError } from "@/lib/ipc";
import { clearLogo, getSettings, pickBackupFolder, pickLogo, saveSettings } from "./api";
import { settingsSchema } from "./schemas";

export function SettingsPage() {
  const { me } = useSession();
  if (me.role !== "ADMIN") {
    return <p className="text-zinc-600 dark:text-zinc-400">Only Admins can change settings.</p>;
  }
  return <SettingsLoader />;
}

function SettingsLoader() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [alert, setAlert] = useState<string>();
  const [notice, setNotice] = useState<string>();

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  return (
    <section className="max-w-xl space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
      <FormAlert message={alert} />
      {notice && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
          {notice}
        </p>
      )}
      {!settings && !alert && <p className="text-zinc-500">Loading settings…</p>}
      {settings && (
        <LogoPanel
          logo={settings.companyLogo}
          onChanged={(s, message) => {
            setSettings(s);
            setNotice(message);
          }}
        />
      )}
      {/* Keyed so a save re-fills the form with what Rust stored, such as the TIN grouped.
          The logo is left out of the key, so changing it keeps edits not yet saved. */}
      {settings && (
        <SettingsForm
          key={JSON.stringify({ ...settings, companyLogo: null })}
          saved={settings}
          onEdit={() => setNotice(undefined)}
          onSaved={(s) => {
            setSettings(s);
            setNotice("Settings saved.");
          }}
        />
      )}
    </section>
  );
}

function SettingsForm({
  saved,
  onEdit,
  onSaved,
}: {
  saved: Settings;
  onEdit: () => void;
  onSaved: (s: Settings) => void;
}) {
  const [folder, setFolder] = useState(saved.backupFolder);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function chooseFolder() {
    setAlert(undefined);
    try {
      const picked = await pickBackupFolder();
      if (picked) setFolder(picked);
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setAlert(undefined);
    onEdit();
    const parsed = settingsSchema.safeParse(formValues(e.currentTarget));
    if (!parsed.success) {
      setErrors(fieldErrors(parsed.error.issues));
      return;
    }
    setErrors({});
    setBusy(true);
    try {
      onSaved(await saveSettings({ ...parsed.data, backupFolder: folder }));
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate aria-label="Settings" className="space-y-6">
      <FormAlert message={alert} />

      <Group title="Company" note="Shown on payslips and reports.">
        <Field
          name="companyName"
          label="Company name"
          defaultValue={saved.companyName}
          error={errors.companyName}
        />
        <Field
          name="companyAddress"
          label="Address"
          defaultValue={saved.companyAddress}
          error={errors.companyAddress}
        />
        <Field
          name="companyTin"
          label="TIN"
          inputMode="numeric"
          defaultValue={formatId("tin", saved.companyTin)}
          error={errors.companyTin}
        />
      </Group>

      <Group title="Security">
        <Field
          name="idleTimeoutMinutes"
          label="Sign out after this many idle minutes"
          type="number"
          inputMode="numeric"
          defaultValue={String(saved.idleTimeoutMinutes)}
          error={errors.idleTimeoutMinutes}
        />
      </Group>

      <Group
        title="Daily backups"
        note="Wagecraft backs up once a day when it starts. Backups before posting payroll or restoring go to the same folder."
      >
        <div className="space-y-2">
          <p className="text-sm font-medium">Folder</p>
          <p className="font-mono text-xs break-all">{folder || saved.defaultBackupFolder}</p>
          {!folder && <p className="text-xs text-zinc-500">The default folder.</p>}
          <div className="flex gap-2">
            <button type="button" className={quiet} onClick={chooseFolder}>
              Choose folder…
            </button>
            {folder && (
              <button type="button" className={quiet} onClick={() => setFolder("")}>
                Use the default folder
              </button>
            )}
          </div>
          {errors.backupFolder && (
            <p className="text-sm text-red-600 dark:text-red-400">{errors.backupFolder}</p>
          )}
        </div>
        <Field
          name="backupKeep"
          label="Daily backups to keep"
          type="number"
          inputMode="numeric"
          defaultValue={String(saved.backupKeep)}
          error={errors.backupKeep}
        />
      </Group>

      <button type="submit" className={primary} disabled={busy}>
        {busy ? "Saving…" : "Save settings"}
      </button>
    </form>
  );
}

/** The logo is saved as soon as it is chosen or removed, apart from the form. */
function LogoPanel({
  logo,
  onChanged,
}: {
  logo: string | null;
  onChanged: (s: Settings, message: string) => void;
}) {
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function change(action: () => Promise<Settings>, message: string) {
    setAlert(undefined);
    setBusy(true);
    try {
      const s = await action();
      // Cancelling the picker returns the settings unchanged.
      if (s.companyLogo !== logo) onChanged(s, message);
    } catch (e) {
      setAlert((e as AppError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Group
      title="Company logo"
      note="Printed on payslips and the payroll register. PNG or JPEG, up to 200 KB."
    >
      <FormAlert message={alert} />
      {logo ? (
        <img src={logo} alt="Company logo" className="max-h-16 max-w-48 object-contain" />
      ) : (
        <p className="text-sm text-zinc-500">No logo yet.</p>
      )}
      <div className="flex gap-2">
        <button
          type="button"
          className={quiet}
          disabled={busy}
          onClick={() => change(pickLogo, "Logo saved.")}
        >
          Choose logo…
        </button>
        {logo && (
          <button
            type="button"
            className={quiet}
            disabled={busy}
            onClick={() => change(clearLogo, "Logo removed.")}
          >
            Remove logo
          </button>
        )}
      </div>
    </Group>
  );
}

function Group({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <fieldset className="space-y-3 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800">
      <legend className="px-1 text-sm font-semibold">{title}</legend>
      {note && <p className="text-sm text-zinc-600 dark:text-zinc-400">{note}</p>}
      {children}
    </fieldset>
  );
}
