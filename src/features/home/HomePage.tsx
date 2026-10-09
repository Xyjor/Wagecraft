import { useEffect, useState } from "react";
import { useSession } from "@/features/auth/session";
import { TeamDashboard } from "@/features/dashboard/TeamDashboard";
import type { Health } from "@/bindings/Health";
import { call, type AppError } from "@/lib/ipc";

export function HomePage() {
  const { me } = useSession();
  if (me.role === "STAFF") return <Welcome />;
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
      <TeamDashboard />
    </section>
  );
}

/** Staff's home page until their own dashboard lands. */
function Welcome() {
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    call<Health>("system_health")
      .then(setHealth)
      .catch((e: AppError) => setError(e.message ?? String(e)));
  }, []);

  return (
    <section className="space-y-2">
      <h1 className="text-2xl font-semibold tracking-tight">Welcome to Wagecraft</h1>
      {error && (
        <p role="alert" className="text-red-600 dark:text-red-400">
          {error}
        </p>
      )}
      {!health && !error && <p className="text-zinc-500">Checking the database…</p>}
      {health && (
        <p className="text-zinc-600 dark:text-zinc-400">Schema version {health.schemaVersion}</p>
      )}
    </section>
  );
}
