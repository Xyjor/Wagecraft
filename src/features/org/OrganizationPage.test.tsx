import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Department } from "@/bindings/Department";
import type { Me } from "@/bindings/Me";
import type { Position } from "@/bindings/Position";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { OrganizationPage } from "./OrganizationPage";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

let departments: Department[];
let positions: Position[];

function renderAs(who: Me = hr) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <OrganizationPage />
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("OrganizationPage", () => {
  beforeEach(() => {
    departments = [
      { id: 1, code: "OPS", name: "Operations", description: null, isActive: true },
      { id: 2, code: "OLD", name: "Old Unit", description: null, isActive: false },
    ];
    positions = [
      {
        id: 10,
        departmentId: 1,
        departmentName: "Operations",
        title: "Driver",
        minRateCents: 1800000,
        maxRateCents: 2500000,
        isActive: true,
      },
      {
        id: 11,
        departmentId: 1,
        departmentName: "Operations",
        title: "Dispatcher",
        minRateCents: null,
        maxRateCents: null,
        isActive: true,
      },
    ];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "department_list") return departments;
      if (cmd === "position_list") return positions;
      return {};
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists departments and positions with salary ranges in pesos", async () => {
    renderAs();
    const driver = await screen.findByRole("row", { name: /^Driver\b/ });
    expect(within(driver).getByText("₱18,000.00 – ₱25,000.00")).toBeTruthy();
    expect(
      within(screen.getByRole("row", { name: /^Dispatcher\b/ })).getByText("No range"),
    ).toBeTruthy();
    expect(within(screen.getByRole("row", { name: /^OLD\b/ })).getByText("Inactive")).toBeTruthy();
  });

  it("tells Staff they can't manage the organization", async () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(await screen.findByText("Only Admin and HR can manage the organization.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("creates a department", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^OPS\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Add department" }));
    type("Code", "fin");
    type("Name", "Finance");
    fireEvent.click(screen.getByRole("button", { name: "Save department" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("department_create", {
        input: { code: "fin", name: "Finance", description: null },
      }),
    );
  });

  it("edits a department in place", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^OPS\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Edit Operations" }));
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Operations");
    type("Name", "Field Ops");
    fireEvent.click(screen.getByRole("button", { name: "Save department" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("department_update", {
        id: 1,
        input: { code: "OPS", name: "Field Ops", description: null },
      }),
    );
  });

  it("deactivates a department", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^OPS\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Deactivate Operations" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("department_set_active", { id: 1, active: false }),
    );
  });

  it("creates a position with a salary range typed in pesos", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Driver\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Add position" }));
    type("Department", "1");
    type("Title", "Mechanic");
    type("Minimum monthly rate", "18,000");
    type("Maximum monthly rate", "25000.50");
    fireEvent.click(screen.getByRole("button", { name: "Save position" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("position_create", {
        input: { departmentId: 1, title: "Mechanic", minRateCents: 1800000, maxRateCents: 2500050 },
      }),
    );
  });

  it("only offers active departments for a position", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Driver\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Add position" }));
    const options = within(screen.getByLabelText("Department")).getAllByRole("option");
    expect(options.map((o) => o.textContent)).toEqual(["Pick a department", "Operations"]);
  });

  it("checks amounts before calling the backend", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Driver\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Add position" }));
    type("Department", "1");
    type("Title", "Mechanic");
    type("Minimum monthly rate", "about 18k");
    fireEvent.click(screen.getByRole("button", { name: "Save position" }));

    expect(await screen.findByText("Enter an amount like 18,000.00")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("position_create", expect.anything());
  });

  it("shows the backend's message next to the field it names", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "department_list") return departments;
      if (cmd === "position_list") return positions;
      throw {
        code: "VALIDATION",
        message: "Some fields are invalid",
        fields: [{ field: "code", message: "Another department already uses this code" }],
      };
    });
    renderAs();
    await screen.findByRole("row", { name: /^OPS\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Add department" }));
    type("Code", "OPS");
    type("Name", "Ops again");
    fireEvent.click(screen.getByRole("button", { name: "Save department" }));

    expect(await screen.findByText("Another department already uses this code")).toBeTruthy();
  });
});
