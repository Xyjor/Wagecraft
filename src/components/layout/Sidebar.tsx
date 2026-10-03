import { NavLink } from "react-router";
import type { Role } from "@/bindings/Role";
import { useSession } from "@/features/auth/session";

// Menu entries arrive with each feature. `roles` hides an entry from everyone else;
// the backend still checks every command, so this is only to keep the menu tidy.
const ITEMS: { to: string; label: string; roles?: Role[] }[] = [
  { to: "/", label: "Home" },
  { to: "/me", label: "My profile" },
  { to: "/my-attendance", label: "My attendance" },
  { to: "/employees", label: "Employees", roles: ["ADMIN", "HR"] },
  { to: "/attendance", label: "Attendance", roles: ["ADMIN", "HR"] },
  { to: "/organization", label: "Organization", roles: ["ADMIN", "HR"] },
  { to: "/holidays", label: "Holidays", roles: ["ADMIN", "HR"] },
  { to: "/users", label: "Users", roles: ["ADMIN"] },
];

export function Sidebar() {
  const { me } = useSession();
  const items = ITEMS.filter((i) => !i.roles || i.roles.includes(me.role));

  return (
    <nav
      aria-label="Main"
      className="flex w-56 shrink-0 flex-col gap-1 border-r border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-900"
    >
      <div className="px-2 pb-4 text-lg font-semibold tracking-tight">Wagecraft</div>
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end
          className={({ isActive }) =>
            `rounded-md px-2 py-1.5 text-sm ${
              isActive
                ? "bg-zinc-200 font-medium dark:bg-zinc-800"
                : "text-zinc-600 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800/60"
            }`
          }
        >
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}
