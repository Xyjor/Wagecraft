import { matchPath } from "react-router";
import type { Role } from "@/bindings/Role";
import { slugify } from "./manual";

type HelpRoute = {
  pattern: string;
  /** The manual heading that explains this screen, which can depend on who is reading. */
  heading: string | ((role: Role) => string);
  /** Roles that can open this screen; the rest never ask for its help. */
  roles?: Role[];
};

const TEAM: Role[] = ["ADMIN", "HR"];

/** Which part of the manual each screen opens. manual.test.ts checks every heading exists. */
export const HELP_ROUTES: HelpRoute[] = [
  { pattern: "/", heading: (role) => (role === "STAFF" ? "Home" : "4.1 Home") },
  { pattern: "/me", heading: "My profile" },
  { pattern: "/password", heading: "Sign in and out" },
  { pattern: "/my-attendance", heading: "My attendance" },
  { pattern: "/my-leave", heading: "My leave" },
  { pattern: "/my-payslips", heading: "My payslips" },
  { pattern: "/organization", heading: "4.2 Organization", roles: TEAM },
  { pattern: "/holidays", heading: "4.3 Holidays", roles: TEAM },
  { pattern: "/employees/*", heading: "4.4 Employees", roles: TEAM },
  { pattern: "/employees", heading: "4.4 Employees", roles: TEAM },
  { pattern: "/attendance", heading: "4.5 Attendance", roles: TEAM },
  { pattern: "/overtime", heading: "4.6 Overtime", roles: TEAM },
  { pattern: "/leave", heading: "4.7 Leave", roles: TEAM },
  { pattern: "/leave-calendar", heading: "4.7 Leave", roles: TEAM },
  { pattern: "/payroll/*", heading: "4.8 Payroll", roles: TEAM },
  { pattern: "/payroll", heading: "4.8 Payroll", roles: TEAM },
  { pattern: "/users", heading: "5.1 Users", roles: ["ADMIN"] },
  { pattern: "/backups", heading: "5.2 Backups", roles: ["ADMIN"] },
  { pattern: "/settings", heading: "5.3 Settings", roles: ["ADMIN"] },
  { pattern: "/activity", heading: "5.4 Activity", roles: ["ADMIN"] },
];

/** The Help page, opened at the section for this screen when there is one. */
export function helpHref(pathname: string, role: Role): string {
  const route = HELP_ROUTES.find((r) => matchPath(r.pattern, pathname));
  if (!route || (route.roles && !route.roles.includes(role))) return "/help";
  const heading = typeof route.heading === "function" ? route.heading(role) : route.heading;
  return `/help#${slugify(heading)}`;
}
