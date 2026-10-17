import { describe, expect, it } from "vitest";
import { setupSteps, type SetupData } from "./setupSteps";

const fresh: SetupData = {
  settings: { companyAddress: "", companyTin: "" },
  departments: [],
  positions: [],
  schedules: [],
  holidays: [],
  employeeCount: 0,
  periods: [],
};

const done = (d: SetupData) =>
  setupSteps(d, 2026)
    .filter((s) => s.done)
    .map((s) => s.id);

describe("setupSteps", () => {
  it("has nothing done on a fresh install, in setup order", () => {
    const steps = setupSteps(fresh, 2026);
    expect(steps.map((s) => s.id)).toEqual([
      "company",
      "org",
      "schedule",
      "holidays",
      "employees",
      "payroll",
    ]);
    expect(done(fresh)).toEqual([]);
    expect(steps[3].label).toBe("Add the 2026 holidays");
  });

  it("needs both the address and the TIN", () => {
    expect(done({ ...fresh, settings: { companyAddress: "Makati", companyTin: " " } })).toEqual([]);
    expect(done({ ...fresh, settings: { companyAddress: "Makati", companyTin: "123" } })).toEqual([
      "company",
    ]);
  });

  it("counts only active departments, positions and schedules", () => {
    const inactive = [{ isActive: false }];
    const active = [{ isActive: true }];
    expect(done({ ...fresh, departments: active, positions: inactive })).toEqual([]);
    expect(done({ ...fresh, departments: active, positions: active, schedules: inactive })).toEqual(
      ["org"],
    );
    expect(done({ ...fresh, schedules: active })).toEqual(["schedule"]);
  });

  it("is all done for a set-up company", () => {
    const active = [{ isActive: true }];
    const all: SetupData = {
      settings: { companyAddress: "Makati", companyTin: "123" },
      departments: active,
      positions: active,
      schedules: active,
      holidays: [{ id: 1 }],
      employeeCount: 12,
      periods: [{ id: 1 }],
    };
    expect(done(all)).toHaveLength(6);
  });
});
