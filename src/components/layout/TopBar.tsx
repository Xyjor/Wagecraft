import { Link, useLocation } from "react-router";
import { CircleHelp } from "lucide-react";
import { useSession } from "@/features/auth/session";
import { helpHref } from "@/features/help/helpTopics";
import { UserMenu } from "./UserMenu";

export function TopBar() {
  const { me } = useSession();
  const { pathname } = useLocation();

  return (
    <header className="flex h-14 shrink-0 items-center justify-end gap-2 border-b border-rule bg-surface px-4">
      <Link
        to={helpHref(pathname, me.role)}
        aria-label="Help for this page"
        title="Help for this page"
        className="grid size-8 place-items-center rounded-md text-ink-muted hover:bg-sunken hover:text-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <CircleHelp aria-hidden="true" className="size-5" strokeWidth={1.75} />
      </Link>
      <UserMenu />
    </header>
  );
}
