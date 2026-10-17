import { useSession } from "@/features/auth/session";
import { MyOvertime } from "@/features/overtime/MyOvertime";
import { myAttendance } from "./api";
import { AttendanceMonth } from "./AttendanceMonth";
import { PageHeader } from "@/components/PageHeader";
import { RoleNote } from "@/components/Notice";

/** The signed-in person's own time records (`/my-attendance`), a month at a time. */
export function MyAttendancePage() {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <RoleNote>
        Your account isn't linked to an employee record. HR can link it from your employee profile.
      </RoleNote>
    );
  }
  return (
    <div className="max-w-4xl space-y-4">
      <PageHeader title="My attendance" />
      <AttendanceMonth load={myAttendance} />
      <p className="text-sm text-ink-faint">
        Clock in and out on the time clock from the sign-in screen. Something wrong? Ask HR to
        correct it.
      </p>
      <MyOvertime />
    </div>
  );
}
