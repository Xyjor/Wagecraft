import { useCallback, useEffect, useState } from "react";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import type { LeaveType } from "@/bindings/LeaveType";
import { FormAlert } from "@/components/form";
import { RejectForm } from "@/components/RejectForm";
import { primaryButton, quietButton } from "@/components/ui";
import { monthRange, thisMonth } from "@/features/attendance/format";
import type { AppError } from "@/lib/ipc";
import { cancelLeave, decideLeave, listLeaveRequests, pendingLeave } from "./api";
import { leaveDates, STATUS_LABELS } from "./format";
import { LeaveRequestForm } from "./LeaveRequestForm";
import { LeaveRequestTable } from "./LeaveRequestTable";

type Status = LeaveRequest["status"];

/** HR's side of leave requests: the queue to decide, every request by month, and filing. */
export function LeaveRequests({
  types,
  myEmployeeId,
}: {
  types: LeaveType[];
  myEmployeeId: number | null;
}) {
  const [pending, setPending] = useState<LeaveRequest[] | null>(null);
  const [history, setHistory] = useState<LeaveRequest[] | null>(null);
  const [month, setMonth] = useState(thisMonth);
  const [status, setStatus] = useState<Status | "">("");
  const [alert, setAlert] = useState<string>();
  const [filing, setFiling] = useState(false);
  const [rejecting, setRejecting] = useState<number | null>(null);

  const load = useCallback(
    () =>
      Promise.all([
        pendingLeave(),
        month ? listLeaveRequests(status || null, monthRange(month)) : Promise.resolve([]),
      ]),
    [month, status],
  );

  const reload = useCallback(async () => {
    const [p, h] = await load();
    setPending(p);
    setHistory(h);
  }, [load]);

  useEffect(() => {
    let live = true;
    load()
      .then(([p, h]) => {
        if (!live) return;
        setPending(p);
        setHistory(h);
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [load]);

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

  function decisionButtons(r: LeaveRequest) {
    if (r.employeeId === myEmployeeId) {
      return <span className="text-zinc-500">Your own: someone else decides</span>;
    }
    if (rejecting === r.id) {
      return (
        <RejectForm
          label={`Reject ${r.employeeName}'s leave`}
          onCancel={() => setRejecting(null)}
          onReject={(note) => act(decideLeave(r.id, false, note))}
        />
      );
    }
    return (
      <div className="flex justify-end gap-1">
        <button
          type="button"
          className={quietButton}
          aria-label={`Approve ${r.employeeName}'s leave`}
          onClick={() => act(decideLeave(r.id, true, null))}
        >
          Approve
        </button>
        <button
          type="button"
          className={quietButton}
          aria-label={`Reject ${r.employeeName}'s leave`}
          onClick={() => setRejecting(r.id)}
        >
          Reject
        </button>
      </div>
    );
  }

  function cancelButton(r: LeaveRequest) {
    if (r.locked) return <span className="text-zinc-500">Payroll posted</span>;
    if (r.status !== "PENDING" && r.status !== "APPROVED") return null;
    return (
      <div className="flex justify-end">
        <button
          type="button"
          className={quietButton}
          aria-label={`Cancel ${r.employeeName}'s leave on ${leaveDates(r)}`}
          onClick={() => act(cancelLeave(r.id))}
        >
          Cancel
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Requests</h2>
        {!filing && (
          <button type="button" className={primaryButton} onClick={() => setFiling(true)}>
            File for an employee
          </button>
        )}
      </div>
      <FormAlert message={alert} />
      {filing && (
        <LeaveRequestForm
          types={types}
          forSomeone
          onCancel={() => setFiling(false)}
          onSaved={async () => {
            setFiling(false);
            await reload();
          }}
        />
      )}

      <section className="space-y-3" aria-labelledby="pending-heading">
        <h3 id="pending-heading" className="font-semibold">
          Waiting for a decision
        </h3>
        {pending && (
          <LeaveRequestTable
            requests={pending}
            empty="Nothing to decide."
            showEmployee
            actions={decisionButtons}
          />
        )}
        <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
          Approving paid leave takes the days from the employee&apos;s balance. Cancelling approved
          leave gives them back.
        </p>
      </section>

      <section className="space-y-3" aria-labelledby="history-heading">
        <h3 id="history-heading" className="font-semibold">
          All requests
        </h3>
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
              className="w-fit rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900"
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
              className="w-fit rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900"
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
          <LeaveRequestTable
            requests={history}
            empty="No leave requests match."
            showEmployee
            actions={cancelButton}
          />
        )}
      </section>
    </div>
  );
}
