import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TeamDashboard as Summary } from "@/bindings/TeamDashboard";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

// Recharts needs a real layout to draw, which jsdom doesn't have; the tables carry the numbers.
vi.mock("./DashboardCharts", () => ({ DepartmentChart: () => null, PayrollCostChart: () => null }));

import { TeamDashboard } from "./TeamDashboard";

const summary: Summary = {
  headcount: 42,
  newHiresThisMonth: 3,
  today: { present: 30, late: 4, notIn: 5, onLeave: 2 },
  pendingLeave: 2,
  pendingOvertime: 1,
  byDepartment: [
    { department: "Operations", count: 30 },
    { department: null, count: 12 },
  ],
  payrollCost: [
    {
      periodStart: "2026-09-16",
      periodEnd: "2026-09-30",
      grossCents: 50_000_000,
      employerCents: 4_500_050,
      costCents: 54_500_050,
    },
    {
      periodStart: "2026-10-01",
      periodEnd: "2026-10-15",
      grossCents: 51_000_000,
      employerCents: 4_600_000,
      costCents: 55_600_000,
    },
  ],
};

function show(data: Summary = summary) {
  call.mockImplementation(async (cmd: string) => {
    if (cmd === "dashboard_team") return data;
    throw new Error(`unexpected ${cmd}`);
  });
  render(
    <MemoryRouter>
      <TeamDashboard />
    </MemoryRouter>,
  );
}

const tile = (label: string) => screen.getByRole("group", { name: label });

beforeEach(() => {
  call.mockReset();
});

afterEach(cleanup);

describe("TeamDashboard", () => {
  it("shows the headcount and today's attendance", async () => {
    show();

    await waitFor(() => expect(tile("Employees").textContent).toContain("42"));
    expect(tile("Employees").textContent).toContain("3 new this month");
    expect(tile("Present").textContent).toContain("30");
    expect(tile("Late").textContent).toContain("4");
    expect(tile("Not clocked in").textContent).toContain("5");
    expect(tile("On leave").textContent).toContain("2");
  });

  it("links the requests waiting for review", async () => {
    show();

    const leave = await screen.findByRole("link", { name: /2 leave requests/ });
    expect(leave.getAttribute("href")).toBe("/leave");
    const ot = screen.getByRole("link", { name: /1 overtime request\b/ });
    expect(ot.getAttribute("href")).toBe("/overtime");
  });

  it("says when nothing is waiting", async () => {
    show({ ...summary, pendingLeave: 0, pendingOvertime: 0 });

    await screen.findByText("No requests waiting for review.");
    expect(screen.queryByRole("link", { name: /request/ })).toBeNull();
  });

  it("gives each chart a table of its numbers", async () => {
    show();

    const depts = await screen.findByRole("table", { name: "Employees by department" });
    const rows = within(depts)
      .getAllByRole("row")
      .map((r) => r.textContent);
    expect(rows).toEqual(["DepartmentEmployees", "Operations30", "No department12"]);

    const cost = screen.getByRole("table", { name: "Payroll cost by period" });
    expect(within(cost).getByText("Sep 16 – 30, 2026")).toBeTruthy();
    expect(within(cost).getByText("₱545,000.50")).toBeTruthy();
    expect(within(cost).getByText("₱556,000.00")).toBeTruthy();
  });

  it("explains an empty payroll chart", async () => {
    show({ ...summary, payrollCost: [] });

    await screen.findByText("No payroll posted yet.");
    expect(screen.queryByRole("table", { name: "Payroll cost by period" })).toBeNull();
  });

  it("shows why the dashboard couldn't load", async () => {
    call.mockRejectedValue({ code: "INTERNAL", message: "Database is locked", fields: [] });
    render(
      <MemoryRouter>
        <TeamDashboard />
      </MemoryRouter>,
    );

    expect((await screen.findByRole("alert")).textContent).toContain("Database is locked");
  });
});
