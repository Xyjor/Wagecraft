import { afterEach, describe, expect, it, vi } from "vitest";
import bootSource from "../../public/theme-boot.js?raw";
import { resolveTheme, THEME_KEY, loadTheme } from "./theme";

// public/theme-boot.js repeats resolveTheme in plain JS so it can run before the app loads.
// This keeps the two copies in step.
function runBoot(saved: string | null, prefersDark: boolean) {
  document.documentElement.classList.remove("dark");
  localStorage.clear();
  if (saved !== null) localStorage.setItem(THEME_KEY, saved);
  vi.stubGlobal("matchMedia", (query: string) => ({ matches: prefersDark, media: query }));
  new Function(bootSource)();
  return document.documentElement.classList.contains("dark") ? "dark" : "light";
}

describe("theme-boot.js", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
    document.documentElement.classList.remove("dark");
  });

  for (const saved of ["LIGHT", "DARK", "SYSTEM", "junk", null]) {
    for (const prefersDark of [false, true]) {
      it(`matches resolveTheme for ${saved} with the OS ${prefersDark ? "dark" : "light"}`, () => {
        expect(runBoot(saved, prefersDark)).toBe(resolveTheme(loadTheme(saved), prefersDark));
      });
    }
  }

  it("still runs when storage throws", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
    });
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    document.documentElement.classList.remove("dark");

    new Function(bootSource)();

    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
