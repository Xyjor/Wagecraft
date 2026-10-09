import { useState } from "react";
import { Link, useLocation } from "react-router";
import { quietButton } from "@/components/ui";
import { ChangePasswordForm } from "./ChangePasswordPage";
import { useSession } from "./session";

/** Where anyone signed in changes their own password, from the link at the top right. */
export function PasswordPage() {
  // Using the link at the top again is a new visit, so it starts from a fresh form.
  const { key: visit } = useLocation();
  return <ChangePassword key={visit} />;
}

function ChangePassword() {
  const { me } = useSession();
  const [done, setDone] = useState(false);

  return (
    <section className="max-w-md space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">Change password</h1>
      {done ? (
        <>
          <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">
            Your password was changed.
          </p>
          <Link to="/" className={quietButton}>
            ← Home
          </Link>
        </>
      ) : (
        <>
          <p className="text-sm text-zinc-600 dark:text-zinc-400">
            Use it the next time you sign in. At least 10 characters; three or four unrelated words
            are easy to remember and hard to guess.
          </p>
          <ChangePasswordForm me={me} onDone={() => setDone(true)} />
        </>
      )}
    </section>
  );
}
