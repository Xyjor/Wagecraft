import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { UserSummary } from "@/bindings/UserSummary";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { UsersPage } from "./UsersPage";

const me: Me = {
  userId: 1,
  username: "admin",
  role: "ADMIN",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function user(id: number, username: string, extra: Partial<UserSummary> = {}): UserSummary {
  return {
    id,
    username,
    role: "HR",
    employeeId: null,
    employeeLabel: null,
    isActive: true,
    mustChangePassword: false,
    locked: false,
    lastLoginAt: null,
    ...extra,
  };
}

let users: UserSummary[];

function renderAs(who: Me = me) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <UsersPage />
    </SessionContext.Provider>,
  );
}

function rowFor(username: string) {
  return screen.getByRole("row", { name: new RegExp(`^${username}\\b`) });
}

describe("UsersPage", () => {
  beforeEach(() => {
    users = [
      user(1, "admin", { role: "ADMIN" }),
      user(2, "maria", { locked: true, mustChangePassword: true }),
    ];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "user_list") return users;
      if (cmd === "user_create") return users[users.length - 1];
      return undefined;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists every account with its role and status", async () => {
    renderAs();
    const maria = await screen.findByRole("row", { name: /^maria\b/ });
    expect(within(maria).getByText("Locked")).toBeTruthy();
    expect(within(maria).getByText("Must change password")).toBeTruthy();
    expect(within(rowFor("admin")).getByText("Active")).toBeTruthy();
  });

  it("tells non-Admins they can't manage users", async () => {
    renderAs({ ...me, role: "HR" });
    expect(await screen.findByText("Only Admins can manage user accounts.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("creates a user with a temporary password", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });

    fireEvent.click(screen.getByRole("button", { name: "Add user" }));
    fireEvent.change(screen.getByLabelText("Username"), { target: { value: "pedro" } });
    fireEvent.change(screen.getByLabelText("Temporary password"), {
      target: { value: "temporary password" },
    });
    users = [...users, user(3, "pedro", { mustChangePassword: true })];
    fireEvent.click(screen.getByRole("button", { name: "Create user" }));

    await screen.findByRole("row", { name: /^pedro\b/ });
    expect(call).toHaveBeenCalledWith("user_create", {
      input: { username: "pedro", role: "HR", password: "temporary password" },
    });
  });

  it("checks the form before calling the backend", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });

    fireEvent.click(screen.getByRole("button", { name: "Add user" }));
    fireEvent.change(screen.getByLabelText("Username"), { target: { value: "pedro" } });
    fireEvent.change(screen.getByLabelText("Temporary password"), { target: { value: "short" } });
    fireEvent.click(screen.getByRole("button", { name: "Create user" }));

    expect(await screen.findByText("Use at least 10 characters")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("user_create", expect.anything());
  });

  it("changes a role", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });
    fireEvent.change(screen.getByLabelText("Role for maria"), { target: { value: "ADMIN" } });
    await waitFor(() => expect(call).toHaveBeenCalledWith("user_update", { id: 2, role: "ADMIN" }));
  });

  it("deactivates someone else but never offers it on your own row", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });
    expect(screen.queryByRole("button", { name: "Deactivate admin" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Deactivate maria" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("user_set_active", { id: 2, active: false }),
    );
  });

  it("shows the reason when the backend refuses", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "user_list") return users;
      throw { code: "CONFLICT", message: "Wagecraft needs at least one active Admin", fields: [] };
    });
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });
    fireEvent.click(screen.getByRole("button", { name: "Deactivate maria" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Wagecraft needs at least one active Admin",
    );
  });

  it("resets a password", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^maria\b/ });

    fireEvent.click(screen.getByRole("button", { name: "Reset password for maria" }));
    fireEvent.change(screen.getByLabelText("New temporary password for maria"), {
      target: { value: "another temp pass" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save password" }));

    expect(
      await screen.findByText("maria must choose a new password at next sign-in."),
    ).toBeTruthy();
    expect(call).toHaveBeenCalledWith("user_reset_password", {
      id: 2,
      temporaryPassword: "another temp pass",
    });
  });
});
