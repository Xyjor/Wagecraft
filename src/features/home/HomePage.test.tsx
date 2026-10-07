import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));
vi.mock("@/features/dashboard/DashboardCharts", () => ({
  DepartmentChart: () => null,
  PayrollCostChart: () => null,
}));

import { HomePage } from "./HomePage";

const hr: Me = {
  userId: 2,
  username: "hr",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function renderAs(me: Me) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

beforeEach(() => {
  call.mockReset();
  call.mockImplementation(async (cmd: string) => {
    if (cmd === "dashboard_team")
      return {
        headcount: 7,
        newHiresThisMonth: 0,
        today: { present: 0, late: 0, notIn: 0, onLeave: 0 },
        pendingLeave: 0,
        pendingOvertime: 0,
        byDepartment: [],
        payrollCost: [],
      };
    if (cmd === "system_health") return { schemaVersion: 14 };
    throw new Error(`unexpected ${cmd}`);
  });
});

afterEach(cleanup);

describe("HomePage", () => {
  it("shows HR and Admin the team dashboard", async () => {
    for (const role of ["HR", "ADMIN"] as const) {
      renderAs({ ...hr, role });
      await waitFor(() =>
        expect(screen.getByRole("group", { name: "Employees" }).textContent).toContain("7"),
      );
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Dashboard");
      cleanup();
    }
  });

  it("doesn't ask Staff's session for the team's numbers", async () => {
    renderAs({ ...hr, role: "STAFF", employeeId: 5 });

    await screen.findByText("Welcome to Wagecraft");
    expect(call).not.toHaveBeenCalledWith("dashboard_team", undefined);
  });
});
