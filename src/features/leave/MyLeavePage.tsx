import { useCallback, useEffect, useState } from "react";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import type { LeaveType } from "@/bindings/LeaveType";
import { FormAlert } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import type { AppError } from "@/lib/ipc";
import { cancelLeave, listLeaveTypes, myBalances, myLeaveRequests } from "./api";
import { BalanceTable } from "./BalanceTable";
import { leaveDates } from "./format";
import { LeaveRequestForm } from "./LeaveRequestForm";
import { LeaveRequestTable } from "./LeaveRequestTable";

/** The signed-in person's leave (`/my-leave`): filing, their requests and this year's balances. */
export function MyLeavePage({ year = new Date().getFullYear() }: { year?: number }) {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn&apos;t linked to an employee record. HR can link it from your employee
        profile.
      </p>
    );
  }
  return <MyLeave year={year} />;
}

function MyLeave({ year }: { year: number }) {
  const [balances, setBalances] = useState<LeaveBalance[] | null>(null);
  const [requests, setRequests] = useState<LeaveRequest[] | null>(null);
  const [types, setTypes] = useState<LeaveType[]>([]);
  const [alert, setAlert] = useState<string>();
  const [filing, setFiling] = useState(false);

  const reload = useCallback(async () => {
    const [b, r] = await Promise.all([myBalances(year), myLeaveRequests(year)]);
    setBalances(b);
    setRequests(r);
  }, [year]);

  useEffect(() => {
    let live = true;
    Promise.all([myBalances(year), myLeaveRequests(year), listLeaveTypes()])
      .then(([b, r, t]) => {
        if (!live) return;
        setBalances(b);
        setRequests(r);
        setTypes(t);
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [year]);

  async function cancel(id: number) {
    setAlert(undefined);
    try {
      await cancelLeave(id);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <div className="max-w-4xl space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">My leave</h1>
        {!filing && (
          <button type="button" className={primaryButton} onClick={() => setFiling(true)}>
            File leave
          </button>
        )}
      </div>
      <FormAlert message={alert} />
      {filing && (
        <LeaveRequestForm
          types={types}
          onCancel={() => setFiling(false)}
          onSaved={async () => {
            setFiling(false);
            await reload();
          }}
        />
      )}

      <section className="space-y-3" aria-labelledby="requests-heading">
        <h2 id="requests-heading" className="text-lg font-semibold">
          Requests in {year}
        </h2>
        {requests && (
          <LeaveRequestTable
            requests={requests}
            empty={`You haven't filed leave for ${year}.`}
            actions={(r) =>
              r.status === "PENDING" &&
              !r.locked && (
                <button
                  type="button"
                  className={quietButton}
                  aria-label={`Cancel leave on ${leaveDates(r)}`}
                  onClick={() => cancel(r.id)}
                >
                  Cancel
                </button>
              )
            }
          />
        )}
        <p className="text-sm text-zinc-500">
          You can cancel leave until HR decides. To take back approved leave, ask HR.
        </p>
      </section>

      <section className="space-y-3" aria-labelledby="balances-heading">
        <h2 id="balances-heading" className="text-lg font-semibold">
          Balances for {year}
        </h2>
        {balances && (
          <BalanceTable balances={balances} empty={`You have no leave balances for ${year}.`} />
        )}
        <p className="text-sm text-zinc-500">
          Service Incentive Leave starts after a year of service. If a balance looks wrong, ask HR.
        </p>
      </section>
    </div>
  );
}
