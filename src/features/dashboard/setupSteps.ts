import type { Department } from "@/bindings/Department";
import type { Holiday } from "@/bindings/Holiday";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { Position } from "@/bindings/Position";
import type { Settings } from "@/bindings/Settings";
import type { WorkSchedule } from "@/bindings/WorkSchedule";

/** What the Admin checklist reads, all from commands that already exist. */
export type SetupData = {
  settings: Pick<Settings, "companyAddress" | "companyTin">;
  departments: Pick<Department, "isActive">[];
  positions: Pick<Position, "isActive">[];
  schedules: Pick<WorkSchedule, "isActive">[];
  holidays: Pick<Holiday, "id">[];
  employeeCount: number;
  periods: Pick<PayrollPeriod, "id">[];
};

export type SetupStep = { id: string; label: string; hint: string; to: string; done: boolean };

const anyActive = (rows: { isActive: boolean }[]) => rows.some((r) => r.isActive);

/** The first-run steps, in the order a new company does them. */
export function setupSteps(d: SetupData, year: number): SetupStep[] {
  return [
    {
      id: "company",
      label: "Add the company address and TIN",
      hint: "They print on every payslip.",
      to: "/settings",
      done: d.settings.companyAddress.trim() !== "" && d.settings.companyTin.trim() !== "",
    },
    {
      id: "org",
      label: "Set up departments and positions",
      hint: "Each employee belongs to one of each.",
      to: "/organization",
      done: anyActive(d.departments) && anyActive(d.positions),
    },
    {
      id: "schedule",
      label: "Create a work schedule",
      hint: "Attendance and leave are counted against it.",
      to: "/organization",
      done: anyActive(d.schedules),
    },
    {
      id: "holidays",
      label: `Add the ${year} holidays`,
      hint: "Holiday pay depends on them.",
      to: "/holidays",
      done: d.holidays.length > 0,
    },
    {
      id: "employees",
      label: "Add your employees",
      hint: "Give each one a pay rate and a schedule.",
      to: "/employees/new",
      done: d.employeeCount > 0,
    },
    {
      id: "payroll",
      label: "Create the first pay period",
      hint: "Then compute, approve and post it.",
      to: "/payroll",
      done: d.periods.length > 0,
    },
  ];
}
