# ADR-013: Ledger design tokens, with a palette bridge for existing pages

- **Status:** Accepted (Oct 2026). Builds on [ADR-012](0012-plain-react-data-loading-and-forms-tailwind-without-shadcn.md): still Tailwind v4 without shadcn/ui.

- **Context:**
  - v1.0 styled every screen with raw Tailwind palette classes: about 480 `zinc`, `emerald`, `sky`, `red` and `amber` classes across 13 feature folders, with no tokens.
  - The app looked grey and generic, and dark mode was a hand-paired `dark:` class on every element.
  - 1.1.0 moves the app to the "Ledger" look, based on the green columnar accounting pad. Rewriting every page in one PR would be large and hard to review.

- **Decision:**
  1. **Semantic tokens.** `src/index.css` defines them as CSS variables (`--wc-*`) on `:root`, with dark values on `.dark`.
     - Tailwind sees them through `@theme inline` as `bg-canvas`, `bg-surface`, `bg-sunken`, `text-ink`, `text-ink-muted`, `border-rule`, `bg-accent`, `text-danger`, `bg-sidebar` and so on.
     - New and reworked code uses only these, so a theme change is one variable swap.
  2. **A palette bridge.** The same file recolours Tailwind's `zinc`, `emerald`, `sky` and `red` scales to Ledger values, so pages not yet reworked match the new look straight away.
     - zinc becomes ledger neutrals: Tailwind's lightness at each step, with the hue turned to green.
     - emerald becomes ledger green, sky becomes the accent, and red becomes red ink.
     - Each step is a fixed value and never changes under `.dark`, because pages already pair them with `dark:` variants.
     - The one exception is `zinc-500`, which follows the theme. It is used 94 times without a `dark:` pair, and it has to pass 4.5:1 contrast on the canvas in both themes.
  3. **A ratchet.** `src/test/palette.test.ts` counts raw palette classes per area and fails if any count goes up.
     - Each PR that reworks a page lowers the limit.
     - `src/components` and `src/app` go to zero first.
  4. **Font.** One workhorse sans, IBM Plex Sans, bundled through `@fontsource-variable/ibm-plex-sans`. It has the peso sign (U+20B1) and tabular figures. Public Sans was tried first and dropped because it has no peso sign. The CSP allows fonts only from `'self'`, and the app runs offline.
  5. **Theme before first paint.** `public/theme-boot.js` sets the `dark` class before React loads. A file is needed because the CSP blocks inline scripts. `src/lib/themeBoot.test.ts` keeps it in step with `resolveTheme`.

- **Form:** the columnar accounting pad.
  - Every ruled surface follows it: square-cornered boxes, visible column rules, a double rule closing page headers, above totals and before the Net pay column, and red ink for deductions.
  - Figures sit in one ruled strip, not separate cards.
  - The payroll steps are ruled column heads.
  - The direction was chosen from four colour options (Ledger, Bundy clock, Peso, Triplicate) rather than from a design-skill concept roll. The form is written down here so later screens keep to it.

- **Alternatives:**
  - _Rewrite every page in one PR_: too large to review, and it blocks other work.
  - _Leave zinc alone and only add tokens_: the app would look half-new until every page was done.
  - _shadcn/ui's token set_: still unreachable from the build machine, and its neutral defaults are the generic look we are leaving.

- **Consequences:**
  - ➕ The new look appears everywhere from the first PR. Dark mode is designed once in the tokens. The ratchet makes progress visible and stops raw classes coming back.
  - ➖ While the bridge existed, "zinc" in a class name meant "ledger neutral", not zinc.
  - **Update (1.1.0, same release):** every page moved to the semantic tokens, so the bridge and the remaining `dark:` classes were removed. `palette.test.ts` now fails on any raw palette class.
  - ➖ The PDFs and Recharts don't use Tailwind classes, so they are recoloured separately.
