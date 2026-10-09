import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";
import { TopBar } from "./TopBar";

const me: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function renderBar(signOut = vi.fn(async () => {})) {
  render(
    <SessionContext.Provider value={{ me, signOut }}>
      <MemoryRouter>
        <TopBar />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
  return signOut;
}

describe("TopBar", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
    }));
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("names who is signed in and links to changing their password", () => {
    renderBar();

    expect(screen.getByText("maria")).toBeTruthy();
    const link = screen.getByRole("link", { name: "Change password" });
    expect(link.getAttribute("href")).toBe("/password");
  });

  it("signs out on request", () => {
    const signOut = renderBar();

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    expect(signOut).toHaveBeenCalledTimes(1);
  });
});
