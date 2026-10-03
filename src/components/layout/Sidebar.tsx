import { NavLink } from "react-router";

// Menu entries arrive with each feature (Employees in Week 4, Attendance in Week 6, ...).
const ITEMS = [{ to: "/", label: "Home" }];

export function Sidebar() {
  return (
    <nav
      aria-label="Main"
      className="flex w-56 shrink-0 flex-col gap-1 border-r border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-900"
    >
      <div className="px-2 pb-4 text-lg font-semibold tracking-tight">Wagecraft</div>
      {ITEMS.map((item) => (
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
