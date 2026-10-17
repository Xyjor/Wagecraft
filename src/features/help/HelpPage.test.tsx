import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { Role } from "@/bindings/Role";
import { SessionContext } from "@/features/auth/session";
import { HelpPage } from "./HelpPage";

function renderAs(role: Role, path = "/help") {
  const me: Me = {
    userId: 1,
    username: "ana",
    role,
    employeeId: null,
    mustChangePassword: false,
    theme: "SYSTEM",
  };
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MemoryRouter initialEntries={[path]}>
        <HelpPage />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("HelpPage", () => {
  it("shows Staff their sections and none of HR's", () => {
    renderAs("STAFF");

    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Help");
    expect(screen.getByRole("heading", { level: 2, name: "3. For staff" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "4. For HR and Admin" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "5.1 Users" })).toBeNull();
  });

  it("shows Admin the Admin sections, with anchors the ? button can open", () => {
    renderAs("ADMIN");

    const users = screen.getByRole("heading", { level: 3, name: "5.1 Users" });
    expect(users.id).toBe("51-users");
  });

  it("lists the sections in a contents box that links to each", () => {
    renderAs("HR");

    const toc = screen.getByRole("navigation", { name: "Contents" });
    const link = toc.querySelector('a[href="#48-payroll"]');
    expect(link?.textContent).toBe("4.8 Payroll");
  });

  it("shows the bundled screenshots", () => {
    renderAs("STAFF");

    const img = screen.getByRole("img", { name: "Sign in" }) as HTMLImageElement;
    expect(img.getAttribute("src")).toMatch(/sign-in.*\.png/);
  });

  it("scrolls to the section in the address", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    renderAs("HR", "/help#48-payroll");

    await waitFor(() => expect(scroll).toHaveBeenCalled());
    expect((scroll.mock.contexts[0] as Element).id).toBe("48-payroll");
  });
});
