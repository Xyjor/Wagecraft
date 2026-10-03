import { useState, type FormEvent } from "react";
import type { Me } from "@/bindings/Me";
import { call, type AppError } from "@/lib/ipc";
import { AuthCard, Field, FormAlert, SubmitButton } from "@/components/form";
import { formValues, serverErrors } from "@/lib/formData";
import { fieldErrors, setupSchema } from "./validation";

/** First run: names the company and creates the first Admin, then signs them in. */
export function SetupWizard({ onDone }: { onDone: (me: Me) => void }) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const parsed = setupSchema.safeParse(formValues(e.currentTarget));
    if (!parsed.success) {
      setErrors(fieldErrors(parsed.error.issues));
      setAlert(undefined);
      return;
    }
    const { companyName, username, password } = parsed.data;
    setBusy(true);
    try {
      await call("auth_setup_create_admin", { companyName, username, password });
      onDone(await call<Me>("auth_login", { username, password }));
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <AuthCard
      title="Set up Wagecraft"
      intro="Name your company and create the first Admin account."
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-4">
        <FormAlert message={alert} />
        <Field
          name="companyName"
          label="Company name"
          autoComplete="organization"
          error={errors.companyName}
        />
        <Field
          name="username"
          label="Admin username"
          autoComplete="username"
          error={errors.username}
        />
        <Field
          name="password"
          label="Password"
          type="password"
          autoComplete="new-password"
          error={errors.password}
        />
        <Field
          name="confirmPassword"
          label="Confirm password"
          type="password"
          autoComplete="new-password"
          error={errors.confirmPassword}
        />
        <SubmitButton busy={busy}>Create Admin</SubmitButton>
      </form>
    </AuthCard>
  );
}
