import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AccountSummary } from "@/bindings/AccountSummary";
import type { Employee } from "@/bindings/Employee";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { AccountTab } from "./AccountTab";

const me: Me = {
  userId: 1,
  username: "hr",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const juan = { id: 7, archivedAt: null } as Employee;
const juanAccount: AccountSummary = {
  userId: 5,
  username: "juan",
  role: "STAFF",
  isActive: true,
};

let account: AccountSummary | null;

function renderTab(employee: Employee = juan) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <AccountTab employee={employee} />
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("AccountTab", () => {
  beforeEach(() => {
    account = null;
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_account") return account;
      if (cmd === "employee_linkable_accounts") {
        return [{ userId: 1, username: "hr", role: "HR", isActive: true }];
      }
      if (cmd === "employee_create_account") {
        account = juanAccount;
        return account;
      }
      if (cmd === "employee_unlink_user") {
        account = null;
        return null;
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("creates a Staff account and shows it", async () => {
    renderTab();
    await screen.findByRole("heading", { name: "Create a Staff account" });
    type("Username", "juan");
    type("Temporary password", "temporary password");
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("employee_create_account", {
        id: 7,
        input: { username: "juan", password: "temporary password" },
      }),
    );
    expect(await screen.findByText("juan")).toBeTruthy();
    expect(screen.getByText("Staff")).toBeTruthy();
  });

  it("offers unlinked accounts to link", async () => {
    renderTab();
    fireEvent.click(await screen.findByRole("button", { name: "Link account" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("employee_link_user", { id: 7, userId: 1 }),
    );
  });

  it("warns that unlinking turns a Staff account off", async () => {
    account = juanAccount;
    renderTab();
    fireEvent.click(await screen.findByRole("button", { name: "Unlink account" }));
    expect(screen.getByText(/it will be turned off/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Unlink" }));

    await waitFor(() => expect(call).toHaveBeenCalledWith("employee_unlink_user", { id: 7 }));
    expect(await screen.findByRole("heading", { name: "Create a Staff account" })).toBeTruthy();
  });

  it("doesn't offer to unlink your own account", async () => {
    account = { userId: 1, username: "hr", role: "HR", isActive: true };
    renderTab();
    await screen.findByText("hr");
    expect(screen.queryByRole("button", { name: "Unlink account" })).toBeNull();
  });

  it("offers nothing new for archived employees", async () => {
    renderTab({ ...juan, archivedAt: "2026-10-01T00:00:00Z" });
    expect(await screen.findByText(/Archived employees can't get one/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Create account" })).toBeNull();
  });
});
