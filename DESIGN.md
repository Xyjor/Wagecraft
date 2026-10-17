---
name: Wagecraft
description: Offline desktop payroll set on a green columnar accounting pad ("Ledger").
colors:
  accent: "#1f6b55"
  accent-hover: "#185645"
  accent-ink: "#ffffff"
  accent-soft: "#e3efe8"
  chart: "#1a7f5e"
  danger: "#b3261e"
  danger-soft: "#fbeae8"
  warn: "#92400e"
  warn-soft: "#fffbeb"
  warn-rule: "#fcd34d"
  sidebar: "#173d33"
  sidebar-active: "#24564a"
  sidebar-ink: "#cfe0d7"
  sidebar-ink-strong: "#ffffff"
  canvas: "#f5f7f2"
  surface: "#ffffff"
  sunken: "#eef2eb"
  ink: "#1b2420"
  ink-muted: "#4f5d55"
  ink-faint: "#606d65"
  rule: "#d5dfd4"
  rule-strong: "#a9b8ab"
  field: "#7d8f84"
  accent-dark: "#5db592"
  accent-hover-dark: "#79c4a8"
  accent-ink-dark: "#0b1a14"
  accent-soft-dark: "#1a3329"
  chart-dark: "#3fa57e"
  danger-dark: "#f28b82"
  danger-soft-dark: "#3a1714"
  warn-dark: "#e5b75a"
  warn-soft-dark: "#2e2410"
  warn-rule-dark: "#92400e"
  sidebar-dark: "#0b1411"
  sidebar-active-dark: "#1c3a30"
  sidebar-ink-dark: "#b9cec3"
  sidebar-ink-strong-dark: "#f0f6f2"
  canvas-dark: "#0f1714"
  surface-dark: "#16211c"
  sunken-dark: "#1b2823"
  ink-dark: "#e4ece6"
  ink-muted-dark: "#a9b9af"
  ink-faint-dark: "#8d9c93"
  rule-dark: "#34473d"
  rule-strong-dark: "#4c6357"
  field-dark: "#5f7468"
typography:
  headline:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.333
    letterSpacing: "-0.025em"
  figure:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.875rem"
    fontWeight: 600
    lineHeight: 1.2
    fontFeature: "tnum"
  title:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 600
    lineHeight: 1.5
  body:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.43
  reading:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.65
  label:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1.33
  numeric:
    fontFamily: "IBM Plex Sans Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    fontFeature: "tnum"
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "4px 10px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "4px 10px"
  button-secondary-hover:
    backgroundColor: "{colors.sunken}"
  button-quiet:
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.md}"
    padding: "4px 10px"
  button-quiet-hover:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink}"
  badge-good:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.accent}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "2px 6px"
  badge-bad:
    backgroundColor: "{colors.danger-soft}"
    textColor: "{colors.danger}"
    rounded: "{rounded.sm}"
    padding: "2px 6px"
  badge-warn:
    backgroundColor: "{colors.warn-soft}"
    textColor: "{colors.warn}"
    rounded: "{rounded.sm}"
    padding: "2px 6px"
  badge-muted:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.sm}"
    padding: "2px 6px"
  badge-final:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.sm}"
    padding: "2px 6px"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  input-filter:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "6px 8px"
    height: "36px"
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sm}"
    padding: "16px"
  tile-strip:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sm}"
  tile:
    typography: "{typography.figure}"
    padding: "12px 16px"
  popover:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
    padding: "8px"
    width: "256px"
  table-header:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink-muted}"
    typography: "{typography.label}"
    padding: "8px 12px"
  table-cell:
    typography: "{typography.body}"
    padding: "8px 12px"
  notice-info:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
  notice-warn:
    backgroundColor: "{colors.warn-soft}"
    textColor: "{colors.warn}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
  notice-danger:
    backgroundColor: "{colors.danger-soft}"
    textColor: "{colors.danger}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
  nav-sidebar:
    backgroundColor: "{colors.sidebar}"
    textColor: "{colors.sidebar-ink}"
    width: "240px"
  nav-item-active:
    backgroundColor: "{colors.sidebar-active}"
    textColor: "{colors.sidebar-ink-strong}"
    rounded: "{rounded.md}"
    padding: "4px 12px"
  topbar:
    backgroundColor: "{colors.surface}"
    height: "56px"
    padding: "0 16px"
  steps-strip:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sm}"
  step-cell:
    textColor: "{colors.ink-faint}"
    typography: "{typography.body}"
    padding: "10px 16px"
  step-cell-done:
    textColor: "{colors.ink}"
  step-cell-current:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.ink}"
---

# Design System: Wagecraft

## Overview

**Creative North Star: "The Ledger Pad"**

Wagecraft is payroll written on a green columnar accounting pad. A deep ledger-green sidebar holds the book's spine; the page is a grey-green paper canvas with white ruled surfaces on it; amounts sit in tabular figures in right-aligned columns; deductions and negatives are in red ink and parentheses; a double rule closes each page header and sits above every total. The mood is calm, exact and trustworthy, the way a well-kept book looks to an auditor.

It is a dense desktop operating tool (Tauri window, minimum 1024x640): small type, tight rows, one accent. It deliberately refuses the generic grey SaaS card kit; structure comes from rules and columns, not from floating cards and shadows. Light and dark are both designed: every colour is a semantic `--wc-*` token with its own value under `.dark`, so components never carry `dark:` classes.

**Key Characteristics:**

- Ruled, flat, near-square surfaces (4px corners): 1px rules for structure, a 3px double rule for closure (page headers, totals rows, the totals column, the figure strip, the wordmark).
- One ledger-green accent; red ink only for deductions, negatives, lateness and errors; amber only for drafts and warnings.
- IBM Plex Sans throughout, tabular figures in every table and number.
- lucide outline icons at 1.75 stroke, always paired with a text label or accessible name.
- A designed dark theme, set before first paint.

## Colors

A green-tinted neutral paper with one ledger-green ink, a red ink, and an amber caution. Values are the frontmatter tokens (`name` = light, `name-dark` = `.dark`); source of truth is `src/index.css`.

### Primary

- **Ledger Green** (`accent`): primary buttons, links, the current-step bar and done-step checks, focus ring (`--color-focus` maps to it), selection tint (`accent-soft`), "good" badges, confirmation text. `accent-hover` deepens it on hover; `accent-ink` is the text on it.
- **Chart Green** (`chart`): the single series colour for Recharts; a slightly brighter green than the accent so bars and lines read on white.

### Secondary

- **Red Ink** (`danger`, `danger-soft`): deduction columns, negative amounts (in parentheses), late counts, error notices, form alerts, "bad" badges. Notice/alert borders are `danger` at 30% opacity.

### Tertiary

- **Draft Amber** (`warn`, `warn-soft`, `warn-rule`): draft status, warning notices. `warn-rule` is the notice border.

### Neutral

- **Pad Paper** (`canvas`): the app background behind everything.
- **White Sheet** (`surface`): cards, panels, tables, top bar, inputs, popovers.
- **Sunken Grey-Green** (`sunken`): table header rows, row hover (at 60%), quiet-button hover, skeleton bars, the theme switch track, info notices.
- **Ink / Muted Ink / Faint Ink** (`ink`, `ink-muted`, `ink-faint`): body text; secondary text, table headers, descriptions; hints, upcoming steps, icon tint. `ink-faint` still passes 4.5:1 on canvas in both themes.
- **Rule / Strong Rule** (`rule`, `rule-strong`): ordinary row and card borders and the dividers between tiles; the strong rule draws every double rule (page header, totals row, totals column, figure strip), the table-header underline, money column rules, the secondary-button border and chart axes.
- **Field Edge** (`field`): input and select borders and empty checklist boxes, darker than `rule` so form controls meet 3:1.
- **Spine Green** (`sidebar`, `sidebar-active`, `sidebar-ink`, `sidebar-ink-strong`): the navigation column only.

### Named Rules

**The Token-Only Rule.** Every colour in a component is a semantic token class (`bg-surface`, `text-ink`, `border-rule`, `text-danger`…). Raw Tailwind palette classes (`zinc-*`, `emerald-*`, `white`, `black`…) are banned and `src/test/palette.test.ts` fails on any of them.

**The Red Ink Rule.** Red means money going out or something wrong: deductions, negatives, lateness, errors. It is never decoration and never a primary action.

**The Not-Colour-Alone Rule.** Status always carries words: badges have labels, tiles keep their label when the number turns red, negatives keep parentheses and a screen-reader "minus", done steps have a check and ", done".

## Typography

**Body Font:** IBM Plex Sans Variable (with Segoe UI, system-ui), bundled locally via `@fontsource-variable/ibm-plex-sans`. It is the only family; it was chosen because it has the peso sign (U+20B1) and tabular figures.

**Character:** One sober workhorse sans at small sizes; hierarchy comes from weight (400/500/600) and rules, not from a display face.

### Hierarchy

- **Headline** (600, 24px, tracking-tight): page titles in the page header only. Auth cards use 20px.
- **Figure** (600, 30px, tabular): the number in a dashboard tile.
- **Title** (600, 16px; 14px in panels): section and panel headings.
- **Body** (400, 14px): almost all UI text: tables, forms, buttons, notices, nav.
- **Reading** (400, 15px / 1.65, max 70ch): the Help manual (`.manual`), with h2 22px closed by a double rule, h3 18px.
- **Label** (500, 12px): table header cells, badges, sidebar group labels, tile footnotes. Sentence case, no tracking.

### Named Rules

**The Tabular Rule.** Every table and every displayed amount uses tabular figures (`table` has `font-variant-numeric: tabular-nums` globally; numeric cells and tiles add `tabular-nums`). Money columns are right-aligned.

## Layout

App shell: fixed sidebar (240px, collapsing to 56px with a 200ms width transition, remembered in localStorage), a 56px top bar holding only Help and the user menu, and a scrolling main area padded 24px on the pad-paper canvas. Pages are left-aligned and width-capped by content: forms `max-w-md` (448px), prose and descriptions `max-w-prose`/`max-w-2xl`, wide tables up to `max-w-6xl`. Spacing runs on a 4px grid: 4/8/12/16/24px; page header bottom margin 24px, card padding 16px, cell padding 8px x 12px, page sections `space-y-8`. Desktop only; no mobile layout.

## Elevation & Depth

Flat by default. Depth is tonal (canvas, surface, sunken) and drawn with rules, never with layered shadows. Only three small shadows exist, all for things that float or are pressed:

### Shadow Vocabulary

- **Sheet lift** (`0 1px 2px rgb(0 0 0 / 0.06)`): the auth card on the bare canvas.
- **Popover** (`0 8px 24px -8px rgb(0 0 0 / 0.25)`): the user menu panel.
- **Selected segment** (`0 1px 2px rgb(0 0 0 / 0.12)`): the checked option in the theme switch.

### Named Rules

**The Ruled-Not-Raised Rule.** Cards, figure strips, panels and tables sit on the page with a 1px rule border and no shadow.

## Shapes

Near-square, like paper: 4px corners for cards, panels, the figure strip, form cards, auth cards, badges and skeleton bars; 6px for the controls (buttons, inputs, notices, nav items, theme switch); 8px only for the floating user-menu popover; square-ish 4px check boxes in the setup checklist and a flat, unrounded progress bar; a full circle only for the avatar initial. Lines carry the system: 1px `rule` borders, a `rule-strong` underline under table headers, 1px `rule-strong` column rules to the left of money columns, and the **3px double rule** (`border-double`, `rule-strong`) as the signature closing line.

## Components

### Buttons

Compact and quiet; the accent is spent once per view.

- **Shape:** 6px radius, 4px x 10px padding, 14px text, 6px icon gap.
- **Primary:** Ledger Green fill, `accent-ink` text, weight 500; hover `accent-hover`. At most one per view. Form submits stretch full width with 8px vertical padding.
- **Secondary:** white sheet, 1px `rule-strong` border, ink text; hover `sunken`. For a second action that must stand out (Export CSV).
- **Quiet:** no fill, `ink-muted` text; hover `sunken` + `ink`. Row and inline actions, icon-only dismiss buttons (with `aria-label`).
- **Focus / Disabled:** one shared ring: 2px `focus` ring offset 2px from canvas. Disabled is 60% opacity with a not-allowed cursor.

### Badges

- **Style:** 4px radius, 2px x 6px, 12px weight 500, soft fill with matching ink. Tones: `good` (accent-soft/accent), `bad` (danger-soft/danger), `warn` (warn-soft/warn), `muted` (sunken/ink-muted), `final` (solid accent, for a state that can't change).
- **Payroll status:** Draft = warn, Computed = muted, Approved = good, Posted = final.

### Cards / Containers

- **Card:** white sheet, 1px `rule` border, 4px radius, 16px padding, no shadow. Form cards are a 448px grid with 16px gaps.
- **Figure strip (signature):** dashboard figures sit as cells in one ruled band, like the totals strip across the top of a ledger page: a white sheet with a 1px `rule-strong` border closed by a 3px double rule at the bottom, cells divided by 1px `rule` column rules, 4px radius. Never a grid of separate floating tiles.
- **Tile:** a strip cell, 12px x 16px, with a 14px muted label, a 30px tabular figure, and an optional 12px faint footnote; `danger`/`warn` tone colours only the figure.
- **Panel:** a card with a 14px semibold heading, named as a landmark.
- **Setup checklist:** a card with an `accent` 30% border, a 6px flat progress bar (accent on sunken, ruled top and bottom with `rule`, no rounding) and 20px square check boxes (4px radius; accent-filled with a check when done, `field` border when not).
- **Payslip inside a register row:** no box of its own; it sits indented 24px under the row between ruled lines.

### Inputs / Fields

- **Style:** white sheet, 1px `field` border, 6px radius, 8px x 12px, 14px ink, `ink-faint` placeholder. Label above in 14px weight 500 with a 4px gap.
- **Focus:** border turns `accent` with a 2px `focus` ring at 40%.
- **Error:** `aria-invalid` turns the border `danger`; the message sits below in 14px red ink and is linked by `aria-describedby`.
- **Filter controls:** 36px tall, 6px x 8px, same colours, in the row above a table.

### Tables

- Full width, 14px, plain `<table>` markup with shared class strings.
- **Header row:** `sunken` fill, 12px weight 500 `ink-muted`, `rule-strong` underline.
- **Rows:** 8px x 12px cells, `rule` underline, hover `sunken` at 60%.
- **Money columns:** right-aligned, tabular, a 1px `rule-strong` column rule on the left. Deduction columns in red ink.
- **Totals column:** the column that totals a row (Net pay) is semibold and set off by a 3px double `rule-strong` rule on its left.
- **Totals row:** semibold, closed above by a 3px double `rule-strong` rule.
- **Empty:** one centered muted row, 24px vertical padding.

### Notices and States

- **Notice:** 6px radius, 1px border, 10px x 12px, 14px, a 16px lucide icon then content. Info: rule/sunken/ink with an accent icon. Warn: warn-rule/warn-soft/warn. Danger: danger 30%/danger-soft/danger. Optional `label` makes it a named region.
- **Status message:** a single 14px accent line with `role="status"`.
- **Loading:** pulsing 14px `sunken` skeleton bars at decreasing widths (90%, 72%, 54%…), pulse disabled under reduced motion.
- **Empty state:** one muted sentence with an optional action beside it.
- **Error state:** a danger notice-style box with the message as an alert and a quiet red Retry beside it.

### Page Header

Optional breadcrumbs (14px muted, lucide chevrons, accent on hover), a 24px semibold title with an optional status badge beside it, a muted description up to 672px, and page actions on the right. It is closed by a 3px double `rule-strong` with 16px below the content and 24px before the page. Page-wide secondary actions (downloads, Save) sit here as secondary buttons.

### Navigation

- **Sidebar:** Spine Green column, `sidebar-ink` 14px items with 16px lucide icons at 1.75 stroke, 6px radius, 4px x 12px. Active item: `sidebar-active` fill, `sidebar-ink-strong`, weight 500. Hover: `sidebar-active` at 60%. Group labels 12px weight 500 at 70% ink; collapsed groups become a thin rule. Its focus ring is `sidebar-ink` offset from the sidebar.
- **Wordmark:** "Wagecraft" 18px semibold, tracking-tight, underlined by a 3px double rule (sidebar ink at 50%; on auth screens in `rule-strong` with accent text).
- **Top bar:** white sheet, 1px `rule` underline, a 32px Help icon button, then the user menu (accent-soft avatar initial, name, faint role, chevron) opening an 8px-radius popover with the theme switch and account actions.
- **Theme switch:** a three-segment radio group (Light, Dark, System) on a sunken track; the checked segment is a white sheet with the small selected shadow.

### Payroll Steps (signature)

A ruled strip like the column heads of a ledger page: a white sheet with a 1px `rule-strong` border and 4px radius, four equal cells (Draft, Computed, Approved, Posted) divided by 1px `rule` column rules and closed by a 3px double `rule-strong` rule. Each cell, 10px x 16px, shows a faint 12px tabular step number and the 14px label. Upcoming steps are `ink-faint`; done steps are `ink` with a 16px accent check pushed right (and ", done" for screen readers); the current step is semibold on `accent-soft` with a 3px `accent` bar along its top (an inset rule, not elevation). Under the rule, a muted sentence says what to do next, and the period's action buttons sit beneath it. A posted period shows every step done.

### Money (signature)

Amounts use accounting format. Negatives are in parentheses and red ink, with a screen-reader-only "minus".

### Charts

Recharts can't resolve CSS variables in SVG attributes, so charts read the live `--wc-*` values from the root element and re-read them when the `dark` class changes. Series use `chart`: square-ended bars and a 2px line without dots, the value axis starting at zero with 25% headroom; grid lines `rule`; axes and ticks `rule-strong` with `ink-muted` 12px labels; tooltips are a white sheet with a `rule` border and 6px radius; hover cursor is `sunken`. One series colour per chart. Charts are `aria-hidden` because a table beside them carries the same data.

## Do's and Don'ts

### Do:

- **Do** use only the semantic token classes (`bg-canvas`, `bg-surface`, `bg-sunken`, `text-ink*`, `border-rule*`, `bg-accent`, `text-danger`, `bg-warn-soft`…); add a new `--wc-*` token with both light and dark values rather than a raw colour.
- **Do** close page headers, totals rows, the totals column and the figure strip with a 3px double `rule-strong` rule.
- **Do** put money in right-aligned tabular columns, deductions and negatives in red ink, negatives in parentheses.
- **Do** say every status in words (badge label, tile label, ", done"), with colour as a second cue.
- **Do** use the shared `primaryButton` / `secondaryButton` / `quietButton`, `focusRing`, `Badge`, `Field`, `Notice`, `PageHeader` and table class strings rather than restyling locally.
- **Do** use lucide outline icons at 1.75 stroke, `aria-hidden`, beside a label or with an accessible name.
- **Do** give charts colours from the live tokens, never hard-coded hexes.
- **Do** respect reduced motion on every transition and pulse.

### Don't:

- **Don't** use raw Tailwind palette classes (`zinc`, `emerald`, `sky`, `red`, `amber`, `slate`, `gray`, `white`, `black`…); `src/test/palette.test.ts` fails the build on them.
- **Don't** add `dark:` classes; dark mode lives in the `.dark` token values.
- **Don't** put more than one primary button in a view.
- **Don't** signal status by colour alone.
- **Don't** use red for anything but money out, lateness and errors, or amber for anything but drafts and warnings.
- **Don't** lift cards, tiles or tables with shadows; shadows are only for the auth sheet, popovers and the selected theme segment.
- **Don't** round containers past 4px; 6px is for controls, 8px only for the floating popover.
- **Don't** add a second typeface or a display face.
