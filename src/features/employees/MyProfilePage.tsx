import { useEffect, useState } from "react";
import type { Employee } from "@/bindings/Employee";
import { FormAlert } from "@/components/form";
import { useSession } from "@/features/auth/session";
import type { AppError } from "@/lib/ipc";
import { getMyProfile } from "./api";
import { ProfileView } from "./ProfileView";

/**
 * The signed-in person's own record (`/me`), read-only. The backend picks the employee
 * from the session, so nobody can ask for someone else's (plan §3.3).
 */
export function MyProfilePage() {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn't linked to an employee record. HR can link it from your employee profile.
      </p>
    );
  }
  return <MyProfile />;
}

function MyProfile() {
  const [employee, setEmployee] = useState<Employee | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    getMyProfile()
      .then(setEmployee)
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  if (!employee) return <FormAlert message={alert} />;
  return (
    <div className="space-y-4">
      <ProfileView employee={employee} />
      <p className="max-w-3xl text-sm text-zinc-500">
        Something wrong or out of date? Ask HR to update it.
      </p>
    </div>
  );
}
