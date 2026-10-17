// Runs before the app so the first frame is already in the right theme, with no white flash.
// It repeats resolveTheme from src/lib/theme.ts; src/lib/themeBoot.test.ts keeps them in step.
// A plain file rather than an inline script, because the CSP only allows scripts from 'self'.
(function () {
  var saved = null;
  try {
    saved = localStorage.getItem("wagecraft.theme");
  } catch (e) {
    // Storage can be blocked; fall back to the OS setting.
  }
  var prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  var dark = saved === "DARK" || (saved !== "LIGHT" && prefersDark);
  document.documentElement.classList.toggle("dark", dark);
})();
