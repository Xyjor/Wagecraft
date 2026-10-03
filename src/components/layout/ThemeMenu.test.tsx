import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeMenu } from "./ThemeMenu";
import { THEME_KEY } from "@/lib/theme";

function mockOsPrefersDark(dark: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: dark,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

describe("ThemeMenu", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("switches to dark and remembers the choice", () => {
    mockOsPrefersDark(false);
    render(<ThemeMenu />);

    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));

    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem(THEME_KEY)).toBe("DARK");
  });

  it("starts from the saved choice", () => {
    mockOsPrefersDark(true);
    localStorage.setItem(THEME_KEY, "LIGHT");
    render(<ThemeMenu />);

    expect(screen.getByRole("radio", { name: "Light" })).toHaveProperty("checked", true);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("follows the OS when set to System", () => {
    mockOsPrefersDark(true);
    render(<ThemeMenu />);

    expect(screen.getByRole("radio", { name: "System" })).toHaveProperty("checked", true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
