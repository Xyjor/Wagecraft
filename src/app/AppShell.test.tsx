import { cleanup, render, screen } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";
import { AppShell } from "./AppShell";

const me: Me = {
  userId: 1,
  username: "ana",
  role: "STAFF",
  employeeId: 3,
  mustChangePassword: false,
  theme: "SYSTEM",
};

afterEach(cleanup);

describe("AppShell", () => {
  it("starts with a link that skips the menu to the page", () => {
    vi.stubGlobal("matchMedia", () => ({
      matches: false,
      addEventListener: () => {},
      removeEventListener: () => {},
    }));
    const router = createMemoryRouter([
      { element: <AppShell />, children: [{ index: true, element: <p>Page</p> }] },
    ]);
    render(
      <SessionContext.Provider value={{ me, signOut: async () => {} }}>
        <RouterProvider router={router} />
      </SessionContext.Provider>,
    );

    const links = screen.getAllByRole("link");
    expect(links[0].textContent).toBe("Skip to content");
    expect(links[0].getAttribute("href")).toBe("#content");
    const main = screen.getByRole("main");
    expect(main.id).toBe("content");
    expect(main.getAttribute("tabindex")).toBe("-1");
    vi.unstubAllGlobals();
  });
});
