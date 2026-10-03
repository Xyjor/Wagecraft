import { afterEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import { call, SIGNED_OUT_EVENT } from "./ipc";

describe("call", () => {
  afterEach(() => invoke.mockReset());

  it("passes the result through", async () => {
    invoke.mockResolvedValueOnce({ dbOk: true });
    await expect(call("system_health")).resolves.toEqual({ dbOk: true });
    expect(invoke).toHaveBeenCalledWith("system_health", undefined);
  });

  it.each(["SESSION_EXPIRED", "UNAUTHENTICATED"])(
    "announces a sign-out when a command fails with %s",
    async (code) => {
      const err = { code, message: "Please sign in", fields: [] };
      invoke.mockRejectedValueOnce(err);
      const heard = vi.fn();
      window.addEventListener(SIGNED_OUT_EVENT, heard);

      await expect(call("auth_me")).rejects.toEqual(err);
      expect(heard).toHaveBeenCalledTimes(1);
      window.removeEventListener(SIGNED_OUT_EVENT, heard);
    },
  );

  it("does not announce a sign-out for other errors", async () => {
    invoke.mockRejectedValueOnce({ code: "FORBIDDEN", message: "No", fields: [] });
    const heard = vi.fn();
    window.addEventListener(SIGNED_OUT_EVENT, heard);

    await expect(call("user_list")).rejects.toMatchObject({ code: "FORBIDDEN" });
    expect(heard).not.toHaveBeenCalled();
    window.removeEventListener(SIGNED_OUT_EVENT, heard);
  });

  it("turns a non-AppError rejection into an AppError", async () => {
    invoke.mockRejectedValueOnce("command auth_nope not found");
    await expect(call("auth_nope")).rejects.toMatchObject({
      code: "INTERNAL",
      message: "Something went wrong. Please try again",
    });
  });
});
