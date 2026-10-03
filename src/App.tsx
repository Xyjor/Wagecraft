import { useEffect, useState } from "react";
import type { Health } from "./bindings/Health";
import { call, type AppError } from "./lib/ipc";

function App() {
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    call<Health>("system_health")
      .then(setHealth)
      .catch((e: AppError) => setError(e.message));
  }, []);

  return (
    <main>
      <h1>Wagecraft</h1>
      {error && <p role="alert">{error}</p>}
      {health && <p>Schema version {health.schemaVersion}</p>}
    </main>
  );
}

export default App;
