import { useSession } from "@/features/auth/session";
import { MyDashboard } from "@/features/dashboard/MyDashboard";
import { TeamDashboard } from "@/features/dashboard/TeamDashboard";

/** The dashboard: Admin and HR see the team, staff see their own day and pay. */
export function HomePage() {
  const { me } = useSession();
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
      {me.role !== "STAFF" ? (
        <TeamDashboard />
      ) : me.employeeId === null ? (
        <p className="text-zinc-600 dark:text-zinc-400">
          Your account isn&apos;t linked to an employee record. HR can link it from your employee
          profile.
        </p>
      ) : (
        <MyDashboard />
      )}
    </section>
  );
}
