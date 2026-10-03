import { useSession } from "@/features/auth/session";
import { MyOvertime } from "@/features/overtime/MyOvertime";
import { myAttendance } from "./api";
import { AttendanceMonth } from "./AttendanceMonth";

/** The signed-in person's own time records (`/my-attendance`), a month at a time. */
export function MyAttendancePage() {
  const { me } = useSession();
  if (me.employeeId === null) {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Your account isn't linked to an employee record. HR can link it from your employee profile.
      </p>
    );
  }
  return (
    <div className="max-w-4xl space-y-4">
      <h1 className="text-xl font-semibold">My attendance</h1>
      <AttendanceMonth load={myAttendance} />
      <p className="text-sm text-zinc-500">
        Clock in and out on the time clock from the sign-in screen. Something wrong? Ask HR to
        correct it.
      </p>
      <MyOvertime />
    </div>
  );
}
