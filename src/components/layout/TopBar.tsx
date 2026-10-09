import { Link } from "react-router";
import { quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { ThemeMenu } from "./ThemeMenu";

export function TopBar() {
  const { me, signOut } = useSession();

  return (
    <header className="flex h-14 items-center justify-end gap-4 border-b border-zinc-200 px-4 dark:border-zinc-800">
      <ThemeMenu />
      <span className="text-sm text-zinc-600 dark:text-zinc-400">{me.username}</span>
      <Link to="/password" className={quietButton}>
        Change password
      </Link>
      <button type="button" onClick={() => void signOut()} className={quietButton}>
        Sign out
      </button>
    </header>
  );
}
