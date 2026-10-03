import { useEffect, useRef, useState } from "react";
import type { KioskPunch } from "@/bindings/KioskPunch";
import type { PunchKind } from "@/bindings/PunchKind";
import { Field, FormAlert } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import type { AppError } from "@/lib/ipc";
import { clockIn, clockOut } from "./api";
import { formatClock } from "./format";

/** How long a confirmation stays up before the kiosk is ready for the next person. */
const CONFIRM_MS = 8000;

/**
 * The shared clock-in screen (plan §6.3), opened from the sign-in screen. It starts no
 * session: each punch sends the employee number and PIN, and the backend reads the time.
 */
export function KioskPage({
  onClose,
  confirmMs = CONFIRM_MS,
}: {
  onClose: () => void;
  /** Tests shorten this. */
  confirmMs?: number;
}) {
  const formRef = useRef<HTMLFormElement>(null);
  const [busy, setBusy] = useState(false);
  const [alert, setAlert] = useState<string>();
  const [done, setDone] = useState<KioskPunch | null>(null);

  useEffect(() => {
    if (!done) return;
    const t = setTimeout(() => setDone(null), confirmMs);
    return () => clearTimeout(t);
  }, [done, confirmMs]);

  async function punch(kind: PunchKind) {
    const form = formRef.current;
    if (!form) return;
    const data = new FormData(form);
    const employeeNo = String(data.get("employeeNo") ?? "").trim();
    const pin = String(data.get("pin") ?? "");
    setDone(null);
    if (!employeeNo || !pin) {
      setAlert("Enter your employee number and PIN.");
      return;
    }
    setBusy(true);
    setAlert(undefined);
    try {
      const result = await (kind === "IN" ? clockIn : clockOut)(employeeNo, pin);
      form.reset();
      setDone(result);
    } catch (e) {
      setAlert((e as AppError).message);
      // Clear the PIN so the next attempt starts fresh; keep the number.
      const pinInput = form.elements.namedItem("pin");
      if (pinInput instanceof HTMLInputElement) pinInput.value = "";
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-zinc-50 p-6 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <div className="w-full max-w-sm space-y-6 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
        <div>
          <h1 className="text-xl font-semibold">Time clock</h1>
          <LiveClock />
        </div>
        {done ? (
          <Confirmation punch={done} />
        ) : (
          <form
            ref={formRef}
            aria-label="Time clock"
            className="flex flex-col gap-4"
            onSubmit={(e) => e.preventDefault()}
          >
            <FormAlert message={alert} />
            <Field name="employeeNo" label="Employee no." autoComplete="off" autoFocus />
            <Field
              name="pin"
              label="PIN"
              type="password"
              inputMode="numeric"
              maxLength={6}
              autoComplete="off"
            />
            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                disabled={busy}
                className={`${primaryButton} py-2`}
                onClick={() => punch("IN")}
              >
                Time in
              </button>
              <button
                type="button"
                disabled={busy}
                className={`${primaryButton} py-2`}
                onClick={() => punch("OUT")}
              >
                Time out
              </button>
            </div>
          </form>
        )}
        <button type="button" className={quietButton} onClick={onClose}>
          Back to sign in
        </button>
      </div>
    </main>
  );
}

function Confirmation({ punch }: { punch: KioskPunch }) {
  const time = formatClock(punch.at);
  const late = punch.kind === "IN" && punch.lateMinutes > 0;
  return (
    <div
      role="status"
      className="space-y-1 rounded-md bg-emerald-50 px-4 py-3 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200"
    >
      <p className="font-medium">
        {punch.kind === "IN" ? "Time in" : "Time out"} recorded, {punch.firstName}.
      </p>
      <p className="text-sm">
        {time}
        {late && ` · ${punch.lateMinutes} minutes late`}
      </p>
    </div>
  );
}

/** The PC's time, ticking. The punch itself uses the backend's clock, not this one. */
function LiveClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(t);
  }, []);
  return (
    <p className="mt-1 text-sm text-zinc-600 tabular-nums dark:text-zinc-400">
      {now.toLocaleDateString("en-PH", { weekday: "long", month: "long", day: "numeric" })} ·{" "}
      {now.toLocaleTimeString("en-PH", { hour: "numeric", minute: "2-digit", second: "2-digit" })}
    </p>
  );
}
