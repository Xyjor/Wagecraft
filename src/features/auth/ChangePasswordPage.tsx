import { useState, type FormEvent } from "react";
import type { Me } from "@/bindings/Me";
import { call, type AppError } from "@/lib/ipc";
import { AuthCard, Field, FormAlert, SubmitButton } from "@/components/form";
import { formValues, serverErrors } from "@/lib/formData";
import { changePasswordSchema, fieldErrors } from "./validation";

/** Shown right after sign-in when an Admin set this user's password for them. */
export function ChangePasswordPage({ me, onDone }: { me: Me; onDone: (me: Me) => void }) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const parsed = changePasswordSchema(me.username).safeParse(formValues(e.currentTarget));
    if (!parsed.success) {
      setErrors(fieldErrors(parsed.error.issues));
      setAlert(undefined);
      return;
    }
    const { currentPassword, newPassword } = parsed.data;
    setBusy(true);
    try {
      await call("auth_change_password", { currentPassword, newPassword });
      onDone({ ...me, mustChangePassword: false });
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <AuthCard
      title="Change your password"
      intro="Your password was set by an Admin. Pick your own before you continue."
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-4">
        <FormAlert message={alert} />
        <Field
          name="currentPassword"
          label="Current password"
          type="password"
          autoComplete="current-password"
          error={errors.currentPassword}
        />
        <Field
          name="newPassword"
          label="New password"
          type="password"
          autoComplete="new-password"
          error={errors.newPassword}
        />
        <Field
          name="confirmPassword"
          label="Confirm new password"
          type="password"
          autoComplete="new-password"
          error={errors.confirmPassword}
        />
        <SubmitButton busy={busy}>Change password</SubmitButton>
      </form>
    </AuthCard>
  );
}
