import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import type { Employee } from "@/bindings/Employee";
import { FormAlert } from "@/components/form";
import { quietButton } from "@/components/ui";
import type { AppError } from "@/lib/ipc";
import { archiveEmployee, getEmployee } from "./api";
import { AccountTab } from "./AccountTab";
import { CompensationTab } from "./CompensationTab";
import { isSeparated } from "./format";
import { ProfileView } from "./ProfileView";

/** HR's view of any employee (`/employees/:id`), with edit, archive, pay and account. */
export function EmployeeProfilePage() {
  const id = Number(useParams().id);
  const [employee, setEmployee] = useState<Employee | null>(null);
  const [alert, setAlert] = useState<string>();

  const reload = useCallback(async () => setEmployee(await getEmployee(id)), [id]);

  useEffect(() => {
    getEmployee(id)
      .then(setEmployee)
      .catch((e: AppError) => setAlert(e.message));
  }, [id]);

  async function setArchived(archived: boolean) {
    setAlert(undefined);
    try {
      await archiveEmployee(id, archived);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  if (!employee) return <FormAlert message={alert} />;
  const e = employee;
  const archived = e.archivedAt !== null;

  return (
    <ProfileView
      employee={e}
      alert={alert}
      actions={
        <>
          {!archived && (
            <Link to={`/employees/${e.id}/edit`} className={quietButton}>
              Edit
            </Link>
          )}
          {!archived && isSeparated(e) && (
            <button type="button" className={quietButton} onClick={() => setArchived(true)}>
              Archive
            </button>
          )}
          {archived && (
            <button type="button" className={quietButton} onClick={() => setArchived(false)}>
              Unarchive
            </button>
          )}
        </>
      }
      extraTabs={[
        { name: "Compensation", content: <CompensationTab employee={e} /> },
        { name: "Sign-in account", content: <AccountTab employee={e} /> },
      ]}
    />
  );
}
