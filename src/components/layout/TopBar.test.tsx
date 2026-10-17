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

  function openMenu() {
    fireEvent.click(screen.getByRole("button", { name: /maria/ }));
  }

  it("names who is signed in and links to changing their password", () => {
    renderBar();

    expect(screen.getByText("maria")).toBeTruthy();
    openMenu();
    const link = screen.getByRole("link", { name: "Change password" });
    expect(link.getAttribute("href")).toBe("/password");
  });

  it("signs out on request", () => {
    const signOut = renderBar();

    openMenu();
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    expect(signOut).toHaveBeenCalledTimes(1);
  });

  it("keeps the theme switch in the account menu", () => {
    renderBar();

    expect(screen.queryByRole("radio", { name: "Dark" })).toBeNull();
    const trigger = screen.getByRole("button", { name: /maria/ });
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    openMenu();

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("radio", { name: "Dark" })).toBeTruthy();
  });

  it("closes the menu on Escape and returns focus to the button", () => {
    renderBar();
    openMenu();

    fireEvent.keyDown(screen.getByRole("link", { name: "Change password" }), { key: "Escape" });

    expect(screen.queryByRole("link", { name: "Change password" })).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: /maria/ }));
  });
});
