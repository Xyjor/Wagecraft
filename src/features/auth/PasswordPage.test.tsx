import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Link, MemoryRouter, Route, Routes } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { PasswordPage } from "./PasswordPage";

const me: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function renderPage() {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MemoryRouter>
        <PasswordPage />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

function submit(current: string, next: string, confirm = next) {
  type("Current password", current);
  type("New password", next);
  type("Confirm new password", confirm);
  fireEvent.click(screen.getByRole("button", { name: "Change password" }));
}

afterEach(() => {
  cleanup();
  call.mockReset();
});

describe("PasswordPage", () => {
  it("changes the password and says so", async () => {
    call.mockResolvedValue(null);
    renderPage();
    expect(screen.getByRole("heading", { name: "Change password" })).toBeTruthy();

    submit("old password 1", "a brand new password");

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("auth_change_password", {
        currentPassword: "old password 1",
        newPassword: "a brand new password",
      }),
    );
    expect((await screen.findByRole("status")).textContent).toBe("Your password was changed.");
    expect(screen.queryByLabelText("Current password")).toBeNull();
  });

  it("shows the server's answer when the current password is wrong", async () => {
    call.mockRejectedValue({
      code: "VALIDATION",
      message: "Check the form",
      fields: [{ field: "currentPassword", message: "Your current password is not correct" }],
    });
    renderPage();

    submit("not it", "a brand new password");

    expect(await screen.findByText("Your current password is not correct")).toBeTruthy();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByLabelText("Current password")).toBeTruthy();
  });

  it("offers a fresh form when the link at the top is used again", async () => {
    call.mockResolvedValue(null);
    render(
      <SessionContext.Provider value={{ me, signOut: async () => {} }}>
        <MemoryRouter initialEntries={["/password"]}>
          <Link to="/password">Change password</Link>
          <Routes>
            <Route path="/password" element={<PasswordPage />} />
          </Routes>
        </MemoryRouter>
      </SessionContext.Provider>,
    );
    submit("old password 1", "a brand new password");
    await screen.findByRole("status");

    fireEvent.click(screen.getByRole("link", { name: "Change password" }));

    expect(screen.getByLabelText("Current password")).toBeTruthy();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("checks the new password before the round trip", () => {
    renderPage();

    submit("old password 1", "a brand new password", "a different password");

    expect(screen.getByText("The passwords don't match")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});
