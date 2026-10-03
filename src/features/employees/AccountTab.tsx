import { useEffect, useState, type FormEvent } from "react";
import type { AccountSummary } from "@/bindings/AccountSummary";
import type { Employee } from "@/bindings/Employee";
import type { Role } from "@/bindings/Role";
import { Field, FormAlert } from "@/components/form";
import { ActiveBadge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { createAccount, getAccount, linkableAccounts, linkAccount, unlinkAccount } from "./api";

const ROLE_LABELS: Record<Role, string> = { ADMIN: "Admin", HR: "HR", STAFF: "Staff" };

type Loaded = { account: AccountSummary | null; linkable: AccountSummary[] };

/** The sign-in account linked to this employee: create a Staff one, link, or unlink. */
export function AccountTab({ employee }: { employee: Employee }) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [alert, setAlert] = useState<string>();

  const load = () =>
    Promise.all([getAccount(employee.id), linkableAccounts()]).then(([account, linkable]) =>
      setLoaded({ account, linkable }),
    );

  useEffect(() => {
    Promise.all([getAccount(employee.id), linkableAccounts()])
      .then(([account, linkable]) => setLoaded({ account, linkable }))
      .catch((e: AppError) => setAlert(e.message));
  }, [employee.id]);

  async function run(action: Promise<unknown>) {
    setAlert(undefined);
    try {
      await action;
      await load();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  if (!loaded) return <FormAlert message={alert} />;
  const { account, linkable } = loaded;

  return (
    <div className="space-y-6">
      <FormAlert message={alert} />
      {account ? (
        <Linked account={account} onUnlink={() => run(unlinkAccount(employee.id))} />
      ) : employee.archivedAt !== null ? (
        <p className="text-sm text-zinc-500">No account. Archived employees can't get one.</p>
      ) : (
        <>
          <p className="text-sm text-zinc-500">
            No account yet. Staff need one to see their payslips and file leave.
          </p>
          <CreateStaff employeeId={employee.id} onCreated={load} />
          {linkable.length > 0 && (
            <LinkExisting
              accounts={linkable}
              onLink={(userId) => run(linkAccount(employee.id, userId))}
            />
          )}
        </>
      )}
    </div>
  );
}

function Linked({ account, onUnlink }: { account: AccountSummary; onUnlink: () => void }) {
  const { me } = useSession();
  const [confirming, setConfirming] = useState(false);
  const isMe = me.userId === account.userId;
  const staff = account.role === "STAFF";

  return (
    <div className="space-y-3">
      <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-[12rem_1fr]">
        <dt className="text-sm text-zinc-500">Username</dt>
        <dd className="text-sm font-medium">{account.username}</dd>
        <dt className="text-sm text-zinc-500">Role</dt>
        <dd className="text-sm">{ROLE_LABELS[account.role]}</dd>
        <dt className="text-sm text-zinc-500">Status</dt>
        <dd>
          <ActiveBadge active={account.isActive} />
        </dd>
      </dl>
      {!isMe && !confirming && (
        <button type="button" className={quietButton} onClick={() => setConfirming(true)}>
          Unlink account
        </button>
      )}
      {confirming && (
        <div className="space-y-2 rounded-md border border-zinc-200 p-3 dark:border-zinc-800">
          <p className="text-sm">
            Unlink {account.username}?
            {staff && " A Staff account can't work without an employee, so it will be turned off."}
          </p>
          <div className="flex gap-1">
            <button type="button" className={primaryButton} onClick={onUnlink}>
              Unlink
            </button>
            <button type="button" className={quietButton} onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function CreateStaff({
  employeeId,
  onCreated,
}: {
  employeeId: number;
  onCreated: () => Promise<void>;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const v = formValues(e.currentTarget);
    const local: Record<string, string> = {};
    if (!v.username?.trim()) local.username = "Enter a username";
    if (!v.password) local.password = "Enter a temporary password";
    setErrors(local);
    if (Object.keys(local).length) return;
    setBusy(true);
    try {
      await createAccount(employeeId, { username: v.username, password: v.password });
      await onCreated();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} noValidate className="space-y-3">
      <h2 className="text-lg font-semibold">Create a Staff account</h2>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Give the employee this username and temporary password. They choose their own password the
        first time they sign in.
      </p>
      <FormAlert message={alert} />
      <div className="grid gap-4 sm:grid-cols-2">
        <Field name="username" label="Username" autoComplete="off" error={errors.username} />
        <Field
          name="password"
          label="Temporary password"
          type="password"
          autoComplete="new-password"
          error={errors.password}
        />
      </div>
      <button type="submit" disabled={busy} className={primaryButton}>
        Create account
      </button>
    </form>
  );
}

function LinkExisting({
  accounts,
  onLink,
}: {
  accounts: AccountSummary[];
  onLink: (userId: number) => void;
}) {
  const [userId, setUserId] = useState(String(accounts[0].userId));
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Or link an existing account</h2>
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        For an Admin or HR user who is also on the payroll.
      </p>
      <div className="flex items-end gap-2">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Account</span>
          <select
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            className="h-9 rounded-md border border-zinc-300 bg-white px-2 text-sm dark:border-zinc-700 dark:bg-zinc-900"
          >
            {accounts.map((a) => (
              <option key={a.userId} value={a.userId}>
                {a.username} ({ROLE_LABELS[a.role]}
                {a.isActive ? "" : ", inactive"})
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className={`${quietButton} h-9 border border-zinc-300 dark:border-zinc-700`}
          onClick={() => onLink(Number(userId))}
        >
          Link account
        </button>
      </div>
    </div>
  );
}
