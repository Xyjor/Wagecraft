import { useEffect, useSyncExternalStore } from "react";
import { applyTheme, loadTheme, THEME_KEY, type Theme } from "./theme";

// One theme store for the whole app, so the switch in the user menu, ThemeSync in main.tsx and
// any other caller always agree. The choice lives in localStorage; `fallback` holds it for this
// session when storage is blocked.
const listeners = new Set<() => void>();
let fallback: Theme = "SYSTEM";

function getSnapshot(): Theme {
  try {
    return loadTheme(localStorage.getItem(THEME_KEY));
  } catch {
    return fallback;
  }
}

function setTheme(theme: Theme) {
  fallback = theme;
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Storage can be unavailable; the theme still applies for this session.
  }
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Keeps the `dark` class on <html> in step with the user's choice and the OS setting. */
export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => applyTheme(theme, media.matches);
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  return [theme, setTheme] as const;
}

/** Applies the theme on every screen, including the ones shown before sign-in. */
export function ThemeSync() {
  useTheme();
  return null;
}
