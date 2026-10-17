import { describe, expect, it } from "vitest";
import { visibleNav } from "./nav";

const labels = (role: Parameters<typeof visibleNav>[0]) =>
  visibleNav(role).map((g) => [g.label, g.items.map((i) => i.label)]);

describe("visibleNav", () => {
  it("shows staff only their own pages", () => {
    expect(labels("STAFF")).toEqual([
      [null, ["Home"]],
      ["Me", ["My profile", "My attendance", "My leave", "My payslips"]],
    ]);
  });

  it("gives HR the people, time and payroll groups but not Admin", () => {
    const groups = visibleNav("HR").map((g) => g.label);
    expect(groups).toEqual([null, "Me", "People", "Time and leave", "Payroll"]);
  });

  it("gives Admin every group", () => {
    const admin = visibleNav("ADMIN");
    expect(admin.map((g) => g.label)).toEqual([
      null,
      "Me",
      "People",
      "Time and leave",
      "Payroll",
      "Admin",
    ]);
    expect(admin[admin.length - 1].items.map((i) => i.label)).toEqual([
      "Users",
      "Backups",
      "Settings",
      "Activity",
    ]);
  });
});
