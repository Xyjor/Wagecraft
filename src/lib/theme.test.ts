import { describe, expect, it } from "vitest";
import { loadTheme, resolveTheme } from "./theme";

describe("resolveTheme", () => {
  it("follows the OS when set to SYSTEM", () => {
    expect(resolveTheme("SYSTEM", true)).toBe("dark");
    expect(resolveTheme("SYSTEM", false)).toBe("light");
  });
  it("ignores the OS when set explicitly", () => {
    expect(resolveTheme("LIGHT", true)).toBe("light");
    expect(resolveTheme("DARK", false)).toBe("dark");
  });
});

describe("loadTheme", () => {
  it("returns the saved choice", () => {
    expect(loadTheme("DARK")).toBe("DARK");
  });
  it("falls back to SYSTEM when nothing or garbage is saved", () => {
    expect(loadTheme(null)).toBe("SYSTEM");
    expect(loadTheme("purple")).toBe("SYSTEM");
  });
});
