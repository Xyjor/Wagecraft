import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { MyProfilePage } from "./MyProfilePage";

const jose: Me = {
  userId: 5,
  username: "jose",
  role: "STAFF",
  employeeId: 8,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const record = {
  id: 8,
  employeeNo: "EMP-0008",
  firstName: "Jose",
  middleName: null,
  lastName: "Rizal",
  suffix: "Jr.",
  employmentStatus: "REGULAR",
  departmentName: "Operations",
  positionTitle: "Driver",
  tin: "123456789",
  sssNo: null,
  philhealthNo: null,
  pagibigNo: null,
  archivedAt: null,
} as Employee;

function renderAs(me: Me) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MyProfilePage />
    </SessionContext.Provider>,
  );
}

describe("MyProfilePage", () => {
  beforeEach(() => {
    call.mockImplementation(async (cmd: string) => (cmd === "employee_me" ? record : null));
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("shows your own record without asking for an id", async () => {
    renderAs(jose);
    expect(await screen.findByRole("heading", { name: "Jose Rizal Jr." })).toBeTruthy();
    expect(call).toHaveBeenCalledWith("employee_me", undefined);
    fireEvent.click(screen.getByRole("tab", { name: "Government IDs" }));
    expect(screen.getByText("123-456-789")).toBeTruthy();
  });

  it("is read-only: no edit, pay or account tabs", async () => {
    renderAs(jose);
    await screen.findByRole("heading", { name: "Jose Rizal Jr." });
    expect(screen.queryByRole("link", { name: "Edit" })).toBeNull();
    expect(screen.queryByRole("tab", { name: "Compensation" })).toBeNull();
    expect(screen.queryByRole("tab", { name: "Sign-in account" })).toBeNull();
  });

  it("explains when the account has no employee record", async () => {
    renderAs({ ...jose, role: "ADMIN", employeeId: null });
    expect(screen.getByText(/isn't linked to an employee record/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});
