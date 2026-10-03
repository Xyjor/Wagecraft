import { useCallback, useEffect, useState } from "react";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { FormAlert } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import type { AppError } from "@/lib/ipc";
import { cancelOvertime, myOvertime } from "./api";
import { recentRange } from "./format";
import { OvertimeForm } from "./OvertimeForm";
import { OvertimeTable } from "./OvertimeTable";

/** The signed-in person's overtime from the month before to the month after today. */
export function MyOvertime() {
  const [requests, setRequests] = useState<OvertimeRequest[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [filing, setFiling] = useState(false);

  const reload = useCallback(async () => {
    setRequests(await myOvertime(recentRange()));
  }, []);

  useEffect(() => {
    let live = true;
    myOvertime(recentRange())
      .then((r) => live && setRequests(r))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, []);

  async function cancel(id: number) {
    setAlert(undefined);
    try {
      await cancelOvertime(id);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <section className="space-y-4" aria-labelledby="overtime-heading">
      <div className="flex items-center justify-between">
        <h2 id="overtime-heading" className="text-lg font-semibold">
          Overtime
        </h2>
        {!filing && (
          <button type="button" className={primaryButton} onClick={() => setFiling(true)}>
            File overtime
          </button>
        )}
      </div>
      <FormAlert message={alert} />
      {filing && (
        <OvertimeForm
          onCancel={() => setFiling(false)}
          onSaved={async () => {
            setFiling(false);
            await reload();
          }}
        />
      )}
      {requests && (
        <OvertimeTable
          requests={requests}
          empty="No overtime from last month to next month."
          actions={(r) =>
            r.status === "PENDING" &&
            !r.locked && (
              <button
                type="button"
                className={quietButton}
                aria-label={`Cancel overtime on ${r.workDate}`}
                onClick={() => cancel(r.id)}
              >
                Cancel
              </button>
            )
          }
        />
      )}
    </section>
  );
}
