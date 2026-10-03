import { useState, type FormEvent } from "react";
import type { Me } from "@/bindings/Me";
import { call, type AppError } from "@/lib/ipc";
import { AuthCard, Field, FormAlert, SubmitButton } from "./form";
import { formValues } from "./formData";

export function LoginPage({
  onSignedIn,
  notice,
}: {
  onSignedIn: (me: Me) => void;
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
    </AuthCard>
  );
}
