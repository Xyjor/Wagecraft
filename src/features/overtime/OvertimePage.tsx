import { useCallback, useEffect, useState } from "react";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { FormAlert } from "@/components/form";
import { RejectForm } from "@/components/RejectForm";
import { primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { monthRange, thisMonth } from "@/features/attendance/format";
import type { AppError } from "@/lib/ipc";
import { cancelOvertime, decideOvertime, listOvertime, pendingOvertime } from "./api";
import { STATUS_LABELS } from "./format";
import { OvertimeForm } from "./OvertimeForm";
import { OvertimeTable } from "./OvertimeTable";
import { PageHeader } from "@/components/PageHeader";

type Status = OvertimeRequest["status"];

/** HR's overtime screen (`/overtime`): the queue to decide, and every request by month. */
export function OvertimePage() {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-ink-muted">
        Only Admin and HR can decide overtime. File your own from My attendance.
      </p>
    );
  }
  return <Overtime myEmployeeId={me.employeeId} />;
}

function Overtime({ myEmployeeId }: { myEmployeeId: number | null }) {
  const [pending, setPending] = useState<OvertimeRequest[] | null>(null);
  const [history, setHistory] = useState<OvertimeRequest[] | null>(null);
  const [month, setMonth] = useState(thisMonth);
  const [status, setStatus] = useState<Status | "">("");
  const [alert, setAlert] = useState<string>();
  const [filing, setFiling] = useState(false);
  const [rejecting, setRejecting] = useState<number | null>(null);

  const reload = useCallback(async () => {
    const [p, h] = await Promise.all([
      pendingOvertime(),
      month ? listOvertime(status || null, monthRange(month)) : Promise.resolve([]),
    ]);
    setPending(p);
    setHistory(h);
  }, [month, status]);

  useEffect(() => {
    let live = true;
    Promise.all([
      pendingOvertime(),
      month ? listOvertime(status || null, monthRange(month)) : Promise.resolve([]),
    ])
      .then(([p, h]) => {
        if (!live) return;
        setPending(p);
        setHistory(h);
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [month, status]);

  async function act(change: Promise<unknown>) {
    setAlert(undefined);
    try {
      await change;
      setRejecting(null);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  function decisionButtons(r: OvertimeRequest) {
    if (r.employeeId === myEmployeeId) {
      return <span className="text-ink-faint">Your own: someone else decides</span>;
    }
    if (rejecting === r.id) {
      return (
        <RejectForm
          label={`Reject ${r.employeeName}'s overtime`}
          onCancel={() => setRejecting(null)}
          onReject={(note) => act(decideOvertime(r.id, false, note))}
        />
      );
    }
    return (
      <div className="flex justify-end gap-1">
        <button
          type="button"
          className={quietButton}
          aria-label={`Approve ${r.employeeName}'s overtime`}
          onClick={() => act(decideOvertime(r.id, true, null))}
        >
          Approve
        </button>
        <button
          type="button"
          className={quietButton}
          aria-label={`Reject ${r.employeeName}'s overtime`}
          onClick={() => setRejecting(r.id)}
        >
          Reject
        </button>
      </div>
    );
  }

  function cancelButton(r: OvertimeRequest) {
    if (r.locked) return <span className="text-ink-faint">Payroll posted</span>;
    if (r.status !== "PENDING" && r.status !== "APPROVED") return null;
    return (
      <div className="flex justify-end">
        <button
          type="button"
          className={quietButton}
          aria-label={`Cancel ${r.employeeName}'s overtime on ${r.workDate}`}
          onClick={() => act(cancelOvertime(r.id))}
        >
          Cancel
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Overtime"
        actions={
          <>
            {!filing && (
              <button type="button" className={primaryButton} onClick={() => setFiling(true)}>
                File for an employee
              </button>
            )}
          </>
        }
      />
      <FormAlert message={alert} />
      {filing && (
        <OvertimeForm
          forSomeone
          onCancel={() => setFiling(false)}
          onSaved={async () => {
            setFiling(false);
            await reload();
          }}
        />
      )}

      <section className="space-y-3" aria-labelledby="pending-heading">
        <h2 id="pending-heading" className="text-lg font-semibold">
          Waiting for a decision
        </h2>
        {pending && (
          <OvertimeTable
            requests={pending}
            empty="Nothing to decide."
            showEmployee
            actions={decisionButtons}
          />
        )}
      </section>

      <section className="space-y-3" aria-labelledby="history-heading">
        <h2 id="history-heading" className="text-lg font-semibold">
          All requests
        </h2>
        <div className="flex flex-wrap gap-4">
          <div className="flex flex-col gap-1">
            <label htmlFor="month" className="text-sm font-medium">
              Month
            </label>
            <input
              id="month"
              type="month"
              value={month}
              onChange={(e) => setMonth(e.target.value)}
              className="w-fit rounded-md border border-field bg-surface px-3 py-1.5 text-sm text-ink"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label htmlFor="status" className="text-sm font-medium">
              Status
            </label>
            <select
              id="status"
              value={status}
              onChange={(e) => setStatus(e.target.value as Status | "")}
              className="w-fit rounded-md border border-field bg-surface px-3 py-1.5 text-sm text-ink"
            >
              <option value="">Any status</option>
              {(Object.keys(STATUS_LABELS) as Status[]).map((s) => (
                <option key={s} value={s}>
                  {STATUS_LABELS[s]}
                </option>
              ))}
            </select>
          </div>
        </div>
        {history && (
          <OvertimeTable
            requests={history}
            empty="No overtime requests match."
            showEmployee
            actions={cancelButton}
          />
        )}
      </section>
    </div>
  );
}
