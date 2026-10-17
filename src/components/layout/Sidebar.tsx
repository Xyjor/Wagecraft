import { useState } from "react";
import { NavLink } from "react-router";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { useSession } from "@/features/auth/session";
import { HELP_ITEM, visibleNav, type NavItem } from "./nav";

export const SIDEBAR_KEY = "wagecraft.sidebar";

function readCollapsed() {
  try {
    return localStorage.getItem(SIDEBAR_KEY) === "collapsed";
  } catch {
    return false;
  }
}

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ink focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar";

export function Sidebar() {
  const { me } = useSession();
  const [collapsed, setCollapsed] = useState(readCollapsed);
  const groups = visibleNav(me.role);

  function toggle() {
    const next = !collapsed;
    setCollapsed(next);
    try {
      localStorage.setItem(SIDEBAR_KEY, next ? "collapsed" : "expanded");
    } catch {
      // Storage can be unavailable; the choice still holds until the app closes.
    }
  }

  return (
    <nav
      aria-label="Main"
      className={`flex shrink-0 flex-col border-r border-sidebar-ink/10 bg-sidebar text-sidebar-ink transition-[width] duration-200 ease-out motion-reduce:transition-none ${
        collapsed ? "w-14" : "w-60"
      }`}
    >
      <div className={`pt-4 pb-3 ${collapsed ? "px-2 text-center" : "px-5"}`}>
        <span className="inline-block border-b-[3px] border-double border-sidebar-ink/50 pb-0.5 text-lg font-semibold tracking-tight text-sidebar-ink-strong">
          {collapsed ? (
            <>
              <span aria-hidden="true">W</span>
              <span className="sr-only">Wagecraft</span>
            </>
          ) : (
            "Wagecraft"
          )}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-4">
        {groups.map((group, gi) => {
          const headingId = group.label ? `nav-${gi}` : undefined;
          return (
            <section
              key={group.label ?? "top"}
              aria-labelledby={headingId}
              className={gi > 0 ? "mt-3" : ""}
            >
              {group.label &&
                (collapsed ? (
                  <>
                    <h2 id={headingId} className="sr-only">
                      {group.label}
                    </h2>
                    <hr aria-hidden="true" className="mx-2 mb-2 border-sidebar-ink/20" />
                  </>
                ) : (
                  <h2 id={headingId} className="px-3 pb-1 text-xs font-medium text-sidebar-ink/70">
                    {group.label}
                  </h2>
                ))}
              <ul className="flex flex-col gap-0.5">
                {group.items.map((item) => (
                  <li key={item.to}>
                    <MenuLink item={item} collapsed={collapsed} />
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </div>

      <div className="space-y-0.5 border-t border-sidebar-ink/15 p-2">
        <MenuLink item={HELP_ITEM} collapsed={collapsed} />
        <button
          type="button"
          onClick={toggle}
          aria-expanded={!collapsed}
          className={`flex w-full items-center gap-3 rounded-md py-1.5 text-sm hover:bg-sidebar-active/60 hover:text-sidebar-ink-strong ${focusRing} ${
            collapsed ? "justify-center" : "px-3"
          }`}
        >
          {collapsed ? (
            <PanelLeftOpen aria-hidden="true" className="size-4" strokeWidth={1.75} />
          ) : (
            <PanelLeftClose aria-hidden="true" className="size-4" strokeWidth={1.75} />
          )}
          <span className={collapsed ? "sr-only" : ""}>
            {collapsed ? "Expand menu" : "Collapse menu"}
          </span>
        </button>
      </div>
    </nav>
  );
}

function MenuLink({ item, collapsed }: { item: NavItem; collapsed: boolean }) {
  return (
    <NavLink
      to={item.to}
      end={item.to === "/"}
      title={collapsed ? item.label : undefined}
      className={({ isActive }) =>
        `flex items-center gap-3 rounded-md py-1 text-sm ${focusRing} ${
          collapsed ? "justify-center px-0" : "px-3"
        } ${
          isActive
            ? "bg-sidebar-active font-medium text-sidebar-ink-strong"
            : "hover:bg-sidebar-active/60 hover:text-sidebar-ink-strong"
        }`
      }
    >
      <item.icon aria-hidden="true" className="size-4 shrink-0" strokeWidth={1.75} />
      <span className={collapsed ? "sr-only" : ""}>{item.label}</span>
    </NavLink>
  );
}
