import {
  Banknote,
  Building2,
  CircleHelp,
  CalendarCheck,
  CalendarMinus,
  CalendarOff,
  CalendarRange,
  Clock,
  DatabaseBackup,
  Flag,
  History,
  LayoutDashboard,
  ReceiptText,
  Settings,
  Timer,
  UserCog,
  UserRound,
  Users,
  type LucideIcon,
} from "lucide-react";
import type { Role } from "@/bindings/Role";

export type NavItem = { to: string; label: string; icon: LucideIcon };
export type NavGroup = { label: string | null; roles?: Role[]; items: NavItem[] };

// Menu groups by job. `roles` hides a group from everyone else; the backend still checks every
// command, so this only keeps the menu tidy.
const NAV: NavGroup[] = [
  { label: null, items: [{ to: "/", label: "Home", icon: LayoutDashboard }] },
  {
    label: "Me",
    items: [
      { to: "/me", label: "My profile", icon: UserRound },
      { to: "/my-attendance", label: "My attendance", icon: CalendarCheck },
      { to: "/my-leave", label: "My leave", icon: CalendarMinus },
      { to: "/my-payslips", label: "My payslips", icon: ReceiptText },
    ],
  },
  {
    label: "People",
    roles: ["ADMIN", "HR"],
    items: [
      { to: "/employees", label: "Employees", icon: Users },
      { to: "/organization", label: "Organization", icon: Building2 },
    ],
  },
  {
    label: "Time and leave",
    roles: ["ADMIN", "HR"],
    items: [
      { to: "/attendance", label: "Attendance", icon: Clock },
      { to: "/overtime", label: "Overtime", icon: Timer },
      { to: "/leave", label: "Leave", icon: CalendarOff },
      { to: "/leave-calendar", label: "Leave calendar", icon: CalendarRange },
      { to: "/holidays", label: "Holidays", icon: Flag },
    ],
  },
  {
    label: "Payroll",
    roles: ["ADMIN", "HR"],
    items: [{ to: "/payroll", label: "Payroll", icon: Banknote }],
  },
  {
    label: "Admin",
    roles: ["ADMIN"],
    items: [
      { to: "/users", label: "Users", icon: UserCog },
      { to: "/backups", label: "Backups", icon: DatabaseBackup },
      { to: "/settings", label: "Settings", icon: Settings },
      { to: "/activity", label: "Activity", icon: History },
    ],
  },
];

/** Help sits at the foot of the menu for everyone. */
export const HELP_ITEM: NavItem = { to: "/help", label: "Help", icon: CircleHelp };

/** The menu groups this role can use. */
export function visibleNav(role: Role): NavGroup[] {
  return NAV.filter((g) => !g.roles || g.roles.includes(role));
}
