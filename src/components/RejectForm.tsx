import { useState, type FormEvent } from "react";
import { quietButton } from "@/components/ui";

/** The inline note HR writes when rejecting a request. A rejection always says why. */
export function RejectForm({
  label,
  onReject,
  onCancel,
}: {
  label: string;
  onReject: (note: string) => Promise<void>;
  onCancel: () => void;
}) {
  const [error, setError] = useState<string>();

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const note = String(new FormData(e.currentTarget).get("note") ?? "").trim();
    if (note.length < 3 || note.length > 200) {
      setError("Say why, in 3 to 200 characters");
      return;
    }
    await onReject(note);
  }

  return (
    <form onSubmit={submit} noValidate aria-label={label} className="space-y-1">
      <label htmlFor="note" className="sr-only">
        Why it&apos;s rejected
      </label>
      <input
        id="note"
        name="note"
        autoFocus
        placeholder="Why it's rejected"
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? "note-error" : undefined}
        className="w-full rounded-md border border-zinc-300 bg-white px-2 py-1 text-sm aria-invalid:border-red-500 dark:border-zinc-700 dark:bg-zinc-900"
      />
      {error && (
        <p id="note-error" className="text-sm text-red-600 dark:text-red-400">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-1">
        <button type="submit" className={quietButton}>
          Reject
        </button>
        <button type="button" className={quietButton} onClick={onCancel}>
          Keep
        </button>
      </div>
    </form>
  );
}
