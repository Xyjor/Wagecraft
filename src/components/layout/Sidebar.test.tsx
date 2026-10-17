import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { Role } from "@/bindings/Role";
import { SessionContext } from "@/features/auth/session";
import { Sidebar, SIDEBAR_KEY } from "./Sidebar";

function renderSidebar(role: Role, path = "/") {
  const me: Me = {
    userId: 1,
    username: "ana",
    role,
    employeeId: null,
    mustChangePassword: false,
    theme: "SYSTEM",
  };
  render(
    <SessionContext.Provider value={{ me, signOut: vi.fn(async () => {}) }}>
      <MemoryRouter initialEntries={[path]}>
        <Sidebar />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

describe("Sidebar", () => {
  beforeEach(() => localStorage.clear());
  afterEach(cleanup);

  it("shows staff only their own pages", () => {
    renderSidebar("STAFF");

    expect(screen.getByRole("link", { name: "My payslips" })).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Employees" })).toBeNull();
    expect(screen.queryByRole("link", { name: "Settings" })).toBeNull();
  });

  it("shows Admin the Admin group", () => {
    renderSidebar("ADMIN");

    expect(screen.getByRole("heading", { name: "Admin" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Backups" })).toBeTruthy();
  });

  it("marks a section as current on its detail pages", () => {
    renderSidebar("HR", "/employees/5");

    expect(screen.getByRole("link", { name: "Employees" }).getAttribute("aria-current")).toBe(
      "page",
    );
    expect(screen.getByRole("link", { name: "Home" }).getAttribute("aria-current")).toBeNull();
  });

  it("collapses to icons, keeps the link names and remembers the choice", () => {
    renderSidebar("HR");

    const toggle = screen.getByRole("button", { name: "Collapse menu" });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(toggle);

    expect(screen.getByRole("button", { name: "Expand menu" }).getAttribute("aria-expanded")).toBe(
      "false",
    );
    expect(screen.getByRole("link", { name: "Payroll" })).toBeTruthy();
    expect(localStorage.getItem(SIDEBAR_KEY)).toBe("collapsed");

    cleanup();
    renderSidebar("HR");
    expect(screen.getByRole("button", { name: "Expand menu" })).toBeTruthy();
  });
});
