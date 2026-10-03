import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";

const handlers: Record<string, (args?: Record<string, unknown>) => Promise<unknown>> = {};
const call = vi.fn((cmd: string, args?: Record<string, unknown>) => {
  const h = handlers[cmd];
  return h
    ? h(args)
    : Promise.reject({ code: "INTERNAL", message: `no handler for ${cmd}`, fields: [] });
});
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { AuthGate } from "./AuthGate";
import { SIGNED_OUT_EVENT } from "@/lib/ipc";

const admin: Me = {
  userId: 1,
  username: "admin",
  role: "ADMIN",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};
const signedOut = { code: "UNAUTHENTICATED", message: "Please sign in", fields: [] };

function setup(state: { needsSetup: boolean; me?: Me }, onUserChanged?: () => void) {
  handlers.auth_setup_status = async () => ({ needsSetup: state.needsSetup });
  handlers.auth_me = async () => {
    if (state.me) return state.me;
    throw signedOut;
  };
  render(
    <AuthGate onUserChanged={onUserChanged}>
      <p>App content</p>
    </AuthGate>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("AuthGate", () => {
  afterEach(() => {
    cleanup();
    call.mockClear();
    for (const k of Object.keys(handlers)) delete handlers[k];
  });

  it("shows the setup wizard on first run", async () => {
    setup({ needsSetup: true });
    expect(await screen.findByRole("heading", { name: "Set up Wagecraft" })).toBeTruthy();
  });

  it("shows the sign-in screen when nobody is signed in", async () => {
    setup({ needsSetup: false });
    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeTruthy();
    expect(screen.queryByText("App content")).toBeNull();
  });

  it("shows the app when a session already exists", async () => {
    setup({ needsSetup: false, me: admin });
    expect(await screen.findByText("App content")).toBeTruthy();
  });

  it("shows the server's message when sign-in fails", async () => {
    setup({ needsSetup: false });
    handlers.auth_login = async () => {
      throw { code: "INVALID_CREDENTIALS", message: "Wrong username or password", fields: [] };
    };
    await screen.findByRole("heading", { name: "Sign in" });

    type("Username", "admin");
    type("Password", "wrong password");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Wrong username or password");
    expect(call).toHaveBeenCalledWith("auth_login", {
      username: "admin",
      password: "wrong password",
    });
  });

  it("sends a user who must change their password to that screen first", async () => {
    setup({ needsSetup: false });
    handlers.auth_login = async () => ({ ...admin, mustChangePassword: true });
    await screen.findByRole("heading", { name: "Sign in" });

    type("Username", "admin");
    type("Password", "temporary pass");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByRole("heading", { name: "Change your password" })).toBeTruthy();
    expect(screen.queryByText("App content")).toBeNull();
  });

  it("goes back to sign-in when the session ends", async () => {
    setup({ needsSetup: false, me: admin });
    await screen.findByText("App content");

    act(() => {
      window.dispatchEvent(new Event(SIGNED_OUT_EVENT));
    });

    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeTruthy();
  });

  it("tells the app when a different person signs in after a sign-out", async () => {
    const onUserChanged = vi.fn();
    setup({ needsSetup: false, me: admin }, onUserChanged);
    await screen.findByText("App content");
    const signIn = async (me: Me) => {
      handlers.auth_login = async () => me;
      act(() => {
        window.dispatchEvent(new Event(SIGNED_OUT_EVENT));
      });
      await screen.findByRole("heading", { name: "Sign in" });
      type("Username", me.username);
      type("Password", "whatever password");
      fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
      await screen.findByText("App content");
    };

    await signIn(admin);
    expect(onUserChanged).not.toHaveBeenCalled();
    await signIn({ ...admin, userId: 5, username: "jose", role: "STAFF" });
    expect(onUserChanged).toHaveBeenCalledTimes(1);
  });

  it("opens the time clock from the sign-in screen and comes back", async () => {
    setup({ needsSetup: false });
    fireEvent.click(await screen.findByRole("button", { name: "Clock in or out" }));
    expect(screen.getByRole("heading", { name: "Time clock" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to sign in" }));
    expect(screen.getByRole("heading", { name: "Sign in" })).toBeTruthy();
  });
});
