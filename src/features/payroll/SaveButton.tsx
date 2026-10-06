import { useState } from "react";
import { FormAlert } from "@/components/form";
import { quietButton } from "@/components/ui";
import type { AppError } from "@/lib/ipc";

/**
 * A button that makes a file and saves it, then says where it went. `save` returns the
 * saved path, or null when the person cancels the Save dialog.
 */
export function SaveButton({
  label,
  busyLabel,
  ariaLabel,
  save,
}: {
  label: string;
  busyLabel: string;
  ariaLabel?: string;
  save: () => Promise<string | null>;
}) {
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState<string>();
  const [alert, setAlert] = useState<string>();

  async function run() {
    setBusy(true);
    setSaved(undefined);
    setAlert(undefined);
    try {
      const path = await save();
      if (path) setSaved(path);
    } catch (e) {
      // A save error comes back as an AppError; a layout error from the PDF library doesn't.
      setAlert((e as Partial<AppError>).message ?? "The file couldn't be made. Please try again");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-3">
      <button
        type="button"
        className={quietButton}
        aria-label={ariaLabel}
        disabled={busy}
        onClick={run}
      >
        {busy ? busyLabel : label}
      </button>
      {saved && <p className="text-sm text-zinc-500">Saved to {saved}</p>}
      <FormAlert message={alert} />
    </div>
  );
}
