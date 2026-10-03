import { useState, type FormEvent } from "react";
import type { Me } from "@/bindings/Me";
import { call, type AppError } from "@/lib/ipc";
import { AuthCard, Field, FormAlert, SubmitButton } from "@/components/form";
import { formValues } from "@/lib/formData";

export function LoginPage({
  onSignedIn,
  onOpenKiosk,
  notice,
}: {
  onSignedIn: (me: Me) => void;
  /** Opens the shared time clock, where staff clock in with their employee number and PIN. */
  onOpenKiosk?: () => void;
  notice?: string;
}) {
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const { username, password } = formValues(e.currentTarget);
    setBusy(true);
    try {
      onSignedIn(await call<Me>("auth_login", { username, password }));
    } catch (err) {
      setAlert((err as AppError).message);
      setBusy(false);
    }
  }

  return (
    <AuthCard title="Sign in" intro={notice}>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <FormAlert message={alert} />
        <Field name="username" label="Username" autoComplete="username" />
        <Field name="password" label="Password" type="password" autoComplete="current-password" />
        <SubmitButton busy={busy}>Sign in</SubmitButton>
      </form>
      {onOpenKiosk && (
        <button
          type="button"
          onClick={onOpenKiosk}
          className="mt-4 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm hover:bg-zinc-50 focus-visible:ring-2 focus-visible:ring-sky-500 dark:border-zinc-700 dark:hover:bg-zinc-800"
        >
          Clock in or out
        </button>
      )}
    </AuthCard>
  );
}
