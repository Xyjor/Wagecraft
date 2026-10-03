import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { EmployeeListItem } from "@/bindings/EmployeeListItem";
import type { EmployeeQuery } from "@/bindings/EmployeeQuery";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { EmployeesPage } from "./EmployeesPage";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function item(id: number, last: string, extra: Partial<EmployeeListItem> = {}): EmployeeListItem {
  return {
    id,
    employeeNo: `EMP-${String(id).padStart(4, "0")}`,
    lastName: last,
    firstName: "Juan",
    middleName: "Santos",
    suffix: null,
    departmentName: "Operations",
    positionTitle: "Driver",
    employmentStatus: "REGULAR",
    hireDate: "2024-06-03",
    archived: false,
    ...extra,
  };
}

/** The `query` argument of the most recent employee_list call. */
function lastQuery(): EmployeeQuery {
  const calls = call.mock.calls.filter(([cmd]) => cmd === "employee_list");
  return calls[calls.length - 1][1].query;
}

function renderAs(who: Me = hr) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <MemoryRouter>
        <EmployeesPage />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

describe("EmployeesPage", () => {
  beforeEach(() => {
    call.mockImplementation(async (cmd: string, args?: { query: EmployeeQuery }) => {
      if (cmd === "department_list") {
        return [{ id: 1, code: "OPS", name: "Operations", description: null, isActive: true }];
      }
      if (cmd === "employee_list") {
        const q = args!.query;
        return {
          items: [item(1, "Dela Cruz"), item(2, "Reyes", { suffix: "Jr." })],
          total: 60,
          page: q.page,
          pageSize: q.pageSize,
        };
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists employees as Last, First M. with a link to each profile", async () => {
    renderAs();
    const link = await screen.findByRole("link", { name: "Dela Cruz, Juan S." });
    expect(link.getAttribute("href")).toBe("/employees/1");
    expect(screen.getByRole("link", { name: "Reyes Jr., Juan S." })).toBeTruthy();
    expect(screen.getByText("Showing 1–25 of 60")).toBeTruthy();
    expect(lastQuery()).toMatchObject({ archived: "exclude", sort: "name", page: 1 });
  });

  it("tells Staff the list is for Admin and HR", async () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(await screen.findByText("Only Admin and HR can see the employee list.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("searches as you type and starts again from page 1", async () => {
    renderAs();
    await screen.findByText("Showing 1–25 of 60");
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    await waitFor(() => expect(lastQuery().page).toBe(2));

    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "dela" } });
    await waitFor(() => expect(lastQuery()).toMatchObject({ search: "dela", page: 1 }));
  });

  it("filters by department, status and archived", async () => {
    renderAs();
    await screen.findByText("Showing 1–25 of 60");
    fireEvent.change(screen.getByLabelText("Department"), { target: { value: "1" } });
    fireEvent.change(screen.getByLabelText("Status"), { target: { value: "PROBATIONARY" } });
    fireEvent.change(screen.getByLabelText("Show"), { target: { value: "only" } });
    await waitFor(() =>
      expect(lastQuery()).toMatchObject({
        departmentId: 1,
        employmentStatus: "PROBATIONARY",
        archived: "only",
      }),
    );
  });

  it("sorts by a column and flips direction on a second click", async () => {
    renderAs();
    await screen.findByText("Showing 1–25 of 60");
    fireEvent.click(screen.getByRole("button", { name: "Hire date" }));
    await waitFor(() => expect(lastQuery()).toMatchObject({ sort: "hireDate", descending: false }));
    fireEvent.click(screen.getByRole("button", { name: "Hire date" }));
    await waitFor(() => expect(lastQuery()).toMatchObject({ sort: "hireDate", descending: true }));
  });

  it("pages through the results", async () => {
    renderAs();
    await screen.findByText("Showing 1–25 of 60");
    expect(
      (screen.getByRole("button", { name: "Previous page" }) as HTMLButtonElement).disabled,
    ).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(await screen.findByText("Showing 26–50 of 60")).toBeTruthy();
  });
});
