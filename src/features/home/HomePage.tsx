import { useEffect, useState } from "react";
import type { Health } from "@/bindings/Health";
import { call, type AppError } from "@/lib/ipc";

export function HomePage() {
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
