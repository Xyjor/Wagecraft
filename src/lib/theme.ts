/** The user's choice. Week 2 stores it in users.theme; until then it lives in localStorage. */
export type Theme = "LIGHT" | "DARK" | "SYSTEM";

const THEMES: readonly Theme[] = ["LIGHT", "DARK", "SYSTEM"];
export const THEME_KEY = "wagecraft.theme";

/** Turns the user's choice into what the screen shows. */
export function resolveTheme(theme: Theme, prefersDark: boolean): "light" | "dark" {
  if (theme === "SYSTEM") return prefersDark ? "dark" : "light";
  return theme === "DARK" ? "dark" : "light";
}

/** Reads a saved value, falling back to SYSTEM for anything unknown. */
export function loadTheme(saved: string | null): Theme {
  return THEMES.find((t) => t === saved) ?? "SYSTEM";
}
