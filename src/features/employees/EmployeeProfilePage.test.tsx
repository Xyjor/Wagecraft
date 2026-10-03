import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { EmployeeProfilePage } from "./EmployeeProfilePage";

const base: Employee = {
  id: 7,
  employeeNo: "EMP-0007",
  firstName: "Juan",
  middleName: "Santos",
  lastName: "Dela Cruz",
  suffix: null,
  birthDate: "1995-03-14",
  sex: "MALE",
  civilStatus: "SINGLE",
  email: "juan@rojyx.ph",
  mobile: "09171234567",
  address: null,
  hireDate: "2024-06-03",
  regularizationDate: null,
  separationDate: null,
  employmentStatus: "REGULAR",
  departmentId: 1,
  departmentName: "Operations",
  positionId: 10,
  positionTitle: "Driver",
  scheduleId: null,
  tin: "123456789000",
  sssNo: "3412345678",
  philhealthNo: "123456789012",
  pagibigNo: "123456789012",
  bankName: "BDO",
  bankAccountNo: "0012-3456-78",
  hasKioskPin: false,
  archivedAt: null,
};

let employee: Employee;

function renderProfile() {
  render(
    <MemoryRouter initialEntries={["/employees/7"]}>
      <Routes>
        <Route path="/employees/:id" element={<EmployeeProfilePage />} />
        <Route path="/employees/:id/edit" element={<p>Edit page</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("EmployeeProfilePage", () => {
  beforeEach(() => {
    employee = base;
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_get") return employee;
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("shows the profile with government IDs in their official format", async () => {
    renderProfile();
    expect(await screen.findByRole("heading", { name: "Juan Santos Dela Cruz" })).toBeTruthy();
    expect(screen.getByText("EMP-0007 · Driver, Operations")).toBeTruthy();

    fireEvent.click(screen.getByRole("tab", { name: "Government IDs" }));
    expect(screen.getByText("123-456-789-000")).toBeTruthy();
    expect(screen.getByText("34-1234567-8")).toBeTruthy();
    expect(screen.getByText("12-345678901-2")).toBeTruthy();
    expect(screen.getByText("1234-5678-9012")).toBeTruthy();
  });

  it("links to the edit form", async () => {
    renderProfile();
    fireEvent.click(await screen.findByRole("link", { name: "Edit" }));
    expect(await screen.findByText("Edit page")).toBeTruthy();
  });

  it("only offers Archive once the employee has left", async () => {
    renderProfile();
    await screen.findByRole("heading", { name: "Juan Santos Dela Cruz" });
    expect(screen.queryByRole("button", { name: "Archive" })).toBeNull();
  });

  it("archives a separated employee", async () => {
    employee = { ...base, employmentStatus: "RESIGNED", separationDate: "2026-09-30" };
    renderProfile();
    fireEvent.click(await screen.findByRole("button", { name: "Archive" }));
    employee = { ...employee, archivedAt: "2026-10-07T01:00:00Z" };

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("employee_archive", { id: 7, archived: true }),
    );
    expect(await screen.findByText("Archived")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Unarchive" })).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Edit" })).toBeNull();
  });
});
