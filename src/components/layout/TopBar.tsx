import { useSession } from "@/features/auth/session";
import { ThemeMenu } from "./ThemeMenu";

export function TopBar() {
  const { me, signOut } = useSession();

  return (
    <header className="flex h-14 items-center justify-end gap-4 border-b border-zinc-200 px-4 dark:border-zinc-800">
      <ThemeMenu />
      <span className="text-sm text-zinc-600 dark:text-zinc-400">{me.username}</span>
      <button
        type="button"
        onClick={() => void signOut()}
        className="rounded-md px-2.5 py-1 text-sm text-zinc-700 hover:bg-zinc-100 focus-visible:ring-2 focus-visible:ring-sky-500 dark:text-zinc-300 dark:hover:bg-zinc-800"
      >
        Sign out
      </button>
    </header>
  );
}
