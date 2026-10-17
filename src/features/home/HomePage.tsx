import { PageHeader } from "@/components/PageHeader";
import { RoleNote } from "@/components/Notice";
import { useSession } from "@/features/auth/session";
import { MyDashboard } from "@/features/dashboard/MyDashboard";
import { TeamDashboard } from "@/features/dashboard/TeamDashboard";

/** The dashboard: Admin and HR see the team, staff see their own day and pay. */
export function HomePage() {
  const { me } = useSession();
  return (
    <section>
      <PageHeader title="Dashboard" />
      {me.role !== "STAFF" ? (
        <TeamDashboard />
      ) : me.employeeId === null ? (
        <RoleNote>
          Your account isn&apos;t linked to an employee record. HR can link it from your employee
          profile.
        </RoleNote>
      ) : (
        <MyDashboard />
      )}
    </section>
  );
}
