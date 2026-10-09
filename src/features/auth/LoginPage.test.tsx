import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { LoginPage } from "./LoginPage";

const WRONG = { code: "INVALID_CREDENTIALS", message: "Wrong username or password", fields: [] };
const HINT = "Five wrong passwords in a row lock the account for 15 minutes.";

function signIn(password: string) {
  fireEvent.change(screen.getByLabelText("Username"), { target: { value: "admin" } });
  fireEvent.change(screen.getByLabelText("Password"), { target: { value: password } });
  fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
}

async function failTimes(n: number) {
  for (let i = 0; i < n; i++) {
    signIn(`wrong ${i}`);
    await waitFor(() => expect(call).toHaveBeenCalledTimes(i + 1));
    await screen.findByText(WRONG.message);
  }
}

describe("LoginPage", () => {
  beforeEach(() => {
    call.mockRejectedValue(WRONG);
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("shows the backend's message on a wrong password, without the lockout hint", async () => {
    render(<LoginPage onSignedIn={() => {}} />);

    await failTimes(4);

    expect(screen.getByText(WRONG.message)).toBeTruthy();
    expect(screen.queryByText(HINT)).toBeNull();
  });

  it("adds the lockout hint on the fifth wrong password in a row", async () => {
    render(<LoginPage onSignedIn={() => {}} />);

    await failTimes(5);

    expect(screen.getByText(HINT)).toBeTruthy();
  });

  it("only counts wrong passwords, not other errors, toward the hint", async () => {
    render(<LoginPage onSignedIn={() => {}} />);
    await failTimes(4);

    call.mockRejectedValue({ code: "INTERNAL", message: "The database is busy", fields: [] });
    signIn("wrong 4");
    await screen.findByText("The database is busy");

    expect(screen.queryByText(HINT)).toBeNull();
  });

  it("drops the hint once a sign-in works", async () => {
    const onSignedIn = vi.fn();
    render(<LoginPage onSignedIn={onSignedIn} />);
    await failTimes(5);
    expect(screen.getByText(HINT)).toBeTruthy();

    call.mockResolvedValue({ userId: 1, username: "admin", role: "ADMIN" });
    signIn("correct horse");

    await waitFor(() => expect(onSignedIn).toHaveBeenCalledTimes(1));
    expect(screen.queryByText(HINT)).toBeNull();
    expect(screen.queryByText(WRONG.message)).toBeNull();
  });

  it("sends the username and password to auth_login", () => {
    render(<LoginPage onSignedIn={() => {}} />);

    signIn("correct horse");

    expect(call).toHaveBeenCalledWith("auth_login", {
      username: "admin",
      password: "correct horse",
    });
  });
});
