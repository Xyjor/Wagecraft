import { useEffect, useState } from "react";
import { loadTheme, resolveTheme, THEME_KEY, type Theme } from "./theme";

function readSaved(): string | null {
  try {
    return localStorage.getItem(THEME_KEY);
  } catch {
    return null;
  }
}

/** Keeps the `dark` class on <html> in step with the user's choice and the OS setting. */
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => loadTheme(readSaved()));

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = resolveTheme(theme, media.matches) === "dark";
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      // Storage can be unavailable; the theme still applies for this session.
    }
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  return [theme, setTheme] as const;
}
