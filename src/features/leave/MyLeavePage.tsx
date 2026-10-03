import { useEffect, useState } from "react";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import { FormAlert } from "@/components/form";
import { useSession } from "@/features/auth/session";
import type { AppError } from "@/lib/ipc";
import { myBalances } from "./api";
import { BalanceTable } from "./BalanceTable";

/** The signed-in person's leave (`/my-leave`): this year's balances. */
export function MyLeavePage({ year = new Date().getFullYear() }: { year?: number }) {
  const { me } = useSession();
  const linked = me.employeeId !== null;
  const [balances, setBalances] = useState<LeaveBalance[] | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    if (!linked) return;
    let live = true;
    myBalances(year)
      .then((b) => live && setBalances(b))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [linked, year]);

  if (!linked) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn&apos;t linked to an employee record. HR can link it from your employee
        profile.
      </p>
    );
  }
  return (
    <div className="max-w-4xl space-y-4">
      <h1 className="text-xl font-semibold">My leave</h1>
      <h2 className="text-lg font-semibold">Balances for {year}</h2>
      <FormAlert message={alert} />
      {balances && (
        <BalanceTable balances={balances} empty={`You have no leave balances for ${year}.`} />
      )}
      <p className="text-sm text-zinc-500">
        Service Incentive Leave starts after a year of service. If a balance looks wrong, ask HR.
      </p>
    </div>
  );
}
