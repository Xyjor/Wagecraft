import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { EmployeeFormPage } from "./EmployeeFormPage";

const departments = [
  { id: 1, code: "OPS", name: "Operations", description: null, isActive: true },
  { id: 2, code: "FIN", name: "Finance", description: null, isActive: true },
];
const positions = [
  {
    id: 10,
    departmentId: 1,
    departmentName: "Operations",
    title: "Driver",
    minRateCents: null,
    maxRateCents: null,
    isActive: true,
  },
  {
    id: 20,
    departmentId: 2,
    departmentName: "Finance",
    title: "Clerk",
    minRateCents: null,
    maxRateCents: null,
    isActive: true,
  },
];

const juan: Employee = {
  id: 7,
  employeeNo: "EMP-0007",
  firstName: "Juan",
  middleName: null,
  lastName: "Dela Cruz",
  suffix: null,
  birthDate: "1995-03-14",
  sex: "MALE",
  civilStatus: null,
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
  tin: "123456789",
  sssNo: null,
  philhealthNo: null,
  pagibigNo: null,
  bankName: null,
  bankAccountNo: null,
  hasKioskPin: false,
  archivedAt: null,
};

function renderAt(path: string) {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/employees/new" element={<EmployeeFormPage />} />
        <Route path="/employees/:id/edit" element={<EmployeeFormPage />} />
        <Route path="/employees/:id" element={<p>Profile page</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

function lastInput(cmd: string) {
  const calls = call.mock.calls.filter(([c]) => c === cmd);
  return calls[calls.length - 1][1];
}

describe("EmployeeFormPage", () => {
  beforeEach(() => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "department_list") return departments;
      if (cmd === "position_list") return positions;
      if (cmd === "employee_get") return juan;
      if (cmd === "employee_create") return { ...juan, id: 8 };
      if (cmd === "employee_update") return juan;
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("creates an employee and opens the new profile", async () => {
    renderAt("/employees/new");
    await screen.findByRole("heading", { name: "Add employee" });
    type("Employee no.", "EMP-0008");
    type("First name", "Maria");
    type("Last name", "Santos");
    type("Hire date", "2025-01-06");
    type("TIN", "123-456-789");
    fireEvent.click(screen.getByRole("button", { name: "Save employee" }));

    expect(await screen.findByText("Profile page")).toBeTruthy();
    expect(lastInput("employee_create").input).toMatchObject({
      employeeNo: "EMP-0008",
      firstName: "Maria",
      middleName: null,
      lastName: "Santos",
      hireDate: "2025-01-06",
      employmentStatus: "PROBATIONARY",
      departmentId: null,
      positionId: null,
      tin: "123-456-789",
      email: null,
    });
  });

  it("points at missing and malformed fields before saving", async () => {
    renderAt("/employees/new");
    await screen.findByRole("heading", { name: "Add employee" });
    type("SSS no.", "12-345");
    fireEvent.click(screen.getByRole("button", { name: "Save employee" }));

    expect(await screen.findByText("Enter the employee number")).toBeTruthy();
    expect(screen.getByText("Enter the first name")).toBeTruthy();
    expect(screen.getByText("Enter the hire date")).toBeTruthy();
    expect(screen.getByText("Use 00-0000000-0 (10 digits)")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("employee_create", expect.anything());
  });

  it("shows the backend's message under the field it names", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "department_list") return departments;
      if (cmd === "position_list") return positions;
      throw {
        code: "VALIDATION",
        message: "Some fields are invalid",
        fields: [{ field: "tin", message: "Another employee has this number" }],
      };
    });
    renderAt("/employees/new");
    await screen.findByRole("heading", { name: "Add employee" });
    type("Employee no.", "EMP-0008");
    type("First name", "Maria");
    type("Last name", "Santos");
    type("Hire date", "2025-01-06");
    type("TIN", "123-456-789");
    fireEvent.click(screen.getByRole("button", { name: "Save employee" }));

    expect(await screen.findByText("Another employee has this number")).toBeTruthy();
  });

  it("offers only the chosen department's positions", async () => {
    renderAt("/employees/new");
    await screen.findByRole("heading", { name: "Add employee" });
    await waitFor(() =>
      expect(within(screen.getByLabelText("Department")).getAllByRole("option")).toHaveLength(3),
    );
    type("Department", "2");
    const titles = within(screen.getByLabelText("Position"))
      .getAllByRole("option")
      .map((o) => o.textContent);
    expect(titles).toEqual(["No position", "Clerk"]);
  });

  it("edits an existing employee", async () => {
    renderAt("/employees/7/edit");
    await screen.findByRole("heading", { name: "Edit Juan Dela Cruz" });
    expect((screen.getByLabelText("Last name") as HTMLInputElement).value).toBe("Dela Cruz");
    expect((screen.getByLabelText("TIN") as HTMLInputElement).value).toBe("123-456-789");
    await waitFor(() =>
      expect((screen.getByLabelText("Position") as HTMLSelectElement).value).toBe("10"),
    );
    type("Last name", "Dela Cruz-Reyes");
    fireEvent.click(screen.getByRole("button", { name: "Save employee" }));

    expect(await screen.findByText("Profile page")).toBeTruthy();
    const sent = lastInput("employee_update");
    expect(sent.id).toBe(7);
    expect(sent.input).toMatchObject({
      lastName: "Dela Cruz-Reyes",
      departmentId: 1,
      positionId: 10,
    });
  });
});
