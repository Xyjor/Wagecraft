import { useEffect, useId, useRef, useState } from "react";
import { Link, useLocation } from "react-router";
import { ChevronDown, KeyRound, LogOut } from "lucide-react";
import { useSession } from "@/features/auth/session";
import { ThemeMenu } from "./ThemeMenu";

const ROLE_NAMES = { ADMIN: "Admin", HR: "HR", STAFF: "Staff" } as const;

const item =
  "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm text-ink hover:bg-sunken focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus";

/**
 * The signed-in account: name and role, the theme switch, Change password and Sign out.
 * A disclosure rather than an ARIA menu, so its links and buttons keep their usual roles.
 */
export function UserMenu() {
  const { me, signOut } = useSession();
  // The page the menu was opened on; it counts as closed once the page changes,
  // e.g. after following Change password.
  const [openOn, setOpenOn] = useState<string | null>(null);
  const { pathname } = useLocation();
  const open = openOn === pathname;
  const panelId = useId();
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    function onPointer(e: PointerEvent) {
      if (!root.current?.contains(e.target as Node)) setOpenOn(null);
    }
    document.addEventListener("pointerdown", onPointer);
    return () => document.removeEventListener("pointerdown", onPointer);
  }, [open]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape" && open) {
      setOpenOn(null);
      trigger.current?.focus();
    }
  }

  const initial = me.username.slice(0, 1).toUpperCase();

  return (
    <div ref={root} className="relative" onKeyDown={onKeyDown}>
      <button
        ref={trigger}
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpenOn(open ? null : pathname)}
        className="flex items-center gap-2 rounded-md py-1 pr-2 pl-1 text-sm hover:bg-sunken focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <span
          aria-hidden="true"
          className="grid size-7 place-items-center rounded-full bg-accent-soft text-xs font-semibold text-accent"
        >
          {initial}
        </span>
        <span className="font-medium text-ink">{me.username}</span>
        <span className="text-ink-faint">{ROLE_NAMES[me.role]}</span>
        <ChevronDown aria-hidden="true" className="size-4 text-ink-faint" strokeWidth={1.75} />
      </button>

      {open && (
        <div
          id={panelId}
          className="absolute right-0 z-20 mt-1 w-64 rounded-lg border border-rule bg-surface p-2 shadow-[0_8px_24px_-8px_rgb(0_0_0/0.25)]"
        >
          <div className="px-2.5 pt-1 pb-2">
            <p className="text-xs text-ink-muted">Theme</p>
            <div className="mt-1.5">
              <ThemeMenu />
            </div>
          </div>
          <div className="my-1 border-t border-rule" />
          <Link to="/password" className={item}>
            <KeyRound aria-hidden="true" className="size-4 text-ink-faint" strokeWidth={1.75} />
            Change password
          </Link>
          <button type="button" onClick={() => void signOut()} className={item}>
            <LogOut aria-hidden="true" className="size-4 text-ink-faint" strokeWidth={1.75} />
            Sign out
          </button>
        </div>
      )}
    </div>
  );
}
