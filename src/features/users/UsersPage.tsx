import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { Role } from "@/bindings/Role";
import type { UserSummary } from "@/bindings/UserSummary";
import { Field, FormAlert, SelectField } from "@/components/form";
import { fieldErrors } from "@/features/auth/validation";
import { useSession } from "@/features/auth/session";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { createUser, listUsers, resetPassword, setActive, setRole } from "./api";
import { newUserSchema, temporaryPasswordSchema } from "./schemas";

const ROLE_OPTIONS = [
  { value: "ADMIN", label: "Admin" },
  { value: "HR", label: "HR" },
];

const button =
  "rounded-md px-2.5 py-1 text-sm focus-visible:ring-2 focus-visible:ring-sky-500 disabled:opacity-60";
const primary = `${button} bg-zinc-900 font-medium text-white hover:bg-zinc-700 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300`;
const quiet = `${button} text-zinc-700 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800`;

export function UsersPage() {
  const { me } = useSession();
  if (me.role !== "ADMIN") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">Only Admins can manage user accounts.</p>
    );
  }
  return <UserAdmin myId={me.userId} />;
}

function UserAdmin({ myId }: { myId: number }) {
  const [users, setUsers] = useState<UserSummary[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [adding, setAdding] = useState(false);
  const [resettingId, setResettingId] = useState<number | null>(null);

  const reload = useCallback(async () => setUsers(await listUsers()), []);

  useEffect(() => {
    listUsers()
      .then(setUsers)
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  /** Runs one change, then shows its outcome and refreshes the list. */
  async function run(change: () => Promise<unknown>, success?: string) {
    setAlert(undefined);
    setNotice(undefined);
    try {
      await change();
      setNotice(success);
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Users</h1>
        {!adding && (
          <button type="button" className={primary} onClick={() => setAdding(true)}>
            Add user
          </button>
        )}
      </div>

      <FormAlert message={alert} />
      {notice && (
        <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
          {notice}
        </p>
      )}

      {adding && (
        <AddUserForm
          onCancel={() => setAdding(false)}
          onCreated={async (u) => {
            setAdding(false);
            setNotice(`${u.username} must choose a new password at first sign-in.`);
            await reload();
          }}
        />
      )}

      {!users && !alert && <p className="text-zinc-500">Loading users…</p>}
      {users && (
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
            <tr>
              <th className="py-2 font-medium">Username</th>
              <th className="py-2 font-medium">Role</th>
              <th className="py-2 font-medium">Status</th>
              <th className="py-2 font-medium">Last sign-in</th>
              <th className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {users.map((u) => (
              <UserRow
                key={u.id}
                user={u}
                isMe={u.id === myId}
                resetting={resettingId === u.id}
                onRole={(role) => run(() => setRole(u.id, role))}
                onActive={(active) => run(() => setActive(u.id, active))}
                onResetStart={() => setResettingId(u.id)}
                onResetDone={async (pw) => {
                  await run(
                    () => resetPassword(u.id, pw),
                    `${u.username} must choose a new password at next sign-in.`,
                  );
                  setResettingId(null);
                }}
                onResetCancel={() => setResettingId(null)}
              />
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

type RowProps = {
  user: UserSummary;
  isMe: boolean;
  resetting: boolean;
  onRole: (role: Role) => void;
  onActive: (active: boolean) => void;
  onResetStart: () => void;
  onResetDone: (temporaryPassword: string) => Promise<void>;
  onResetCancel: () => void;
};

function UserRow({ user: u, isMe, resetting, ...on }: RowProps) {
  return (
    <>
      <tr className="border-b border-zinc-100 dark:border-zinc-900">
        <td className="py-2 font-medium">
          {u.username}
          {isMe && <span className="ml-1 text-zinc-500">(you)</span>}
        </td>
        <td className="py-2">
          <select
            aria-label={`Role for ${u.username}`}
            value={u.role}
            disabled={isMe}
            onChange={(e) => on.onRole(e.target.value as Role)}
            className="rounded-md border border-zinc-300 bg-white px-2 py-1 text-sm disabled:opacity-60 dark:border-zinc-700 dark:bg-zinc-900"
          >
            {ROLE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
            {u.role === "STAFF" && (
              <option value="STAFF" disabled>
                Staff
              </option>
            )}
          </select>
        </td>
        <td className="py-2">
          <div className="flex flex-wrap gap-1">
            <Badge tone={u.isActive ? "good" : "muted"}>{u.isActive ? "Active" : "Inactive"}</Badge>
            {u.locked && <Badge tone="bad">Locked</Badge>}
            {u.mustChangePassword && <Badge tone="muted">Must change password</Badge>}
          </div>
        </td>
        <td className="py-2 text-zinc-600 dark:text-zinc-400">{formatWhen(u.lastLoginAt)}</td>
        <td className="py-2">
          <div className="flex justify-end gap-1">
            <button
              type="button"
              className={quiet}
              aria-label={`Reset password for ${u.username}`}
              onClick={on.onResetStart}
            >
              Reset password
            </button>
            {!isMe && (
              <button
                type="button"
                className={quiet}
                aria-label={`${u.isActive ? "Deactivate" : "Activate"} ${u.username}`}
                onClick={() => on.onActive(!u.isActive)}
              >
                {u.isActive ? "Deactivate" : "Activate"}
              </button>
            )}
          </div>
        </td>
      </tr>
      {resetting && (
        <tr>
          <td colSpan={5} className="py-2">
            <ResetPasswordForm
              username={u.username}
              onSave={on.onResetDone}
              onCancel={on.onResetCancel}
            />
          </td>
        </tr>
      )}
    </>
  );
}

function AddUserForm({
  onCreated,
  onCancel,
}: {
  onCreated: (u: UserSummary) => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const parsed = newUserSchema.safeParse(formValues(e.currentTarget));
    if (!parsed.success) {
      setErrors(fieldErrors(parsed.error.issues));
      return;
    }
    setBusy(true);
    try {
      await onCreated(await createUser(parsed.data));
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      noValidate
      aria-label="Add user"
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <Field
        name="username"
        label="Username"
        autoComplete="off"
        autoFocus
        error={errors.username}
      />
      <SelectField
        name="role"
        label="Role"
        options={ROLE_OPTIONS}
        defaultValue="HR"
        error={errors.role}
      />
      <Field
        name="password"
        label="Temporary password"
        type="password"
        autoComplete="new-password"
        error={errors.password}
      />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Staff accounts are created from the employee's profile.
      </p>
      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primary}>
          Create user
        </button>
        <button type="button" onClick={onCancel} className={quiet}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function ResetPasswordForm({
  username,
  onSave,
  onCancel,
}: {
  username: string;
  onSave: (pw: string) => Promise<void>;
  onCancel: () => void;
}) {
  const [error, setError] = useState<string>();

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const parsed = temporaryPasswordSchema(username).safeParse(
      formValues(e.currentTarget).temporaryPassword,
    );
    if (!parsed.success) {
      setError(parsed.error.issues[0]?.message);
      return;
    }
    await onSave(parsed.data);
  }

  return (
    <form onSubmit={submit} noValidate className="flex max-w-md items-end gap-2">
      <div className="flex-1">
        <Field
          name="temporaryPassword"
          label={`New temporary password for ${username}`}
          type="password"
          autoFocus
          autoComplete="new-password"
          error={error}
        />
      </div>
      <button type="submit" className={primary}>
        Save password
      </button>
      <button type="button" onClick={onCancel} className={quiet}>
        Cancel
      </button>
    </form>
  );
}

function Badge({ tone, children }: { tone: "good" | "bad" | "muted"; children: string }) {
  const tones = {
    good: "bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300",
    bad: "bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-300",
    muted: "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300",
  };
  return <span className={`rounded px-1.5 py-0.5 text-xs ${tones[tone]}`}>{children}</span>;
}

function formatWhen(iso: string | null): string {
  if (!iso) return "Never";
  return new Date(iso).toLocaleString("en-PH", { dateStyle: "medium", timeStyle: "short" });
}
