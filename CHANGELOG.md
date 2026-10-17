# Changelog

What changed in each version of Wagecraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] - 2026-10-17

A new look, and help inside the app.

### Added

- **Help.** The user manual is built into the app, with its screenshots, and shows only the
  sections for your role. The **?** at the top of every screen opens the part about that screen,
  and Help is at the foot of the sidebar. It works offline.
- **Payroll steps.** Each pay period shows Draft, Computed, Approved and Posted, marks where it
  is, and says what to do next, with the buttons for it underneath.
- **Finish setting up.** A checklist on the Admin dashboard for a new company: company address and
  TIN, departments and positions, a work schedule, this year's holidays, employees, and the first
  pay period. It ticks itself off and goes away once everything is done, or when hidden.
- A "Skip to content" link for keyboard users, shown when it has focus.

### Changed

- **The "Ledger" look**, after a green columnar accounting pad. It has a ledger-green sidebar,
  grey-green paper, ruled tables, square-cornered boxes, and page headers closed by a double rule.
  Deductions are in red ink, and a double rule sits above totals and before net pay. It ships
  with the IBM Plex Sans typeface. Light and Dark are both designed. See
  [ADR-013](docs/adr/0013-ledger-design-tokens.md) and [DESIGN.md](DESIGN.md).
- **Negative amounts** show as (₱1,149.43) in red on screen and in the PDFs, so they still read
  when printed in black and white.
- **Sidebar.** It is grouped into Me, People, Time and leave, Payroll and Admin, with icons. It
  collapses to icons, and the choice is remembered. A section stays highlighted on its detail
  pages.
- **Account menu.** The Light/Dark/System switch, Change password and Sign out are now in a
  menu under your name at the top right.
- **Dashboard.** The figures sit in one ruled strip. Late arrivals show in red and people not
  clocked in show in amber, only when there are some. The charts follow Light and Dark. A
  dashboard that fails to load offers **Retry**.
- **Page headers.** Pages have breadcrumbs back to their list, for example Employees › Bea
  Torres › Edit. On the register, Download CSV and Download PDF are in the header.
- **Payslip and register PDFs** have a green rule under the company, ruled lines, double rules
  above totals and net pay on a green band.
- **Window size.** The window can't be made smaller than 1024 × 640.

### Fixed

- The sign-in, setup, time clock and forced password-change screens follow the Light, Dark or
  System choice, and the app no longer flashes white before dark mode applies.
- Muted text and the faint table text meet the WCAG AA contrast minimum in both themes.
- The user manual no longer says Wagecraft ships with the 2026 holidays; a new install starts with
  none.

## [1.0.1] - 2026-10-17

### Fixed

- Download PDF (the payroll register and payslips) failed in the installed app: the content
  security policy refused the PDF library's WebAssembly layout engine. `script-src` now allows
  `'wasm-unsafe-eval'`, which permits only that; `eval` and outside scripts stay blocked.
- The manual test script now sets a work schedule before Juan clocks in (leave can't be filed
  without one), names the first-time PIN button correctly, and posts a cutoff that has ended.

## [1.0.0] - 2026-10-09

The first release.

### Added

- **Sign-in and accounts:** first-run setup, sign-in with Argon2id passwords and a 15-minute
  lockout after five wrong passwords, idle timeout, Admin, HR and Staff roles, user management
  with last-Admin protection, a Change password page for everyone signed in, and an audit log of
  every sign-in and change.
- **Organization and employees:** departments, positions, employee records with search, profile,
  edit and archive, salary history with dated rate changes, the minimum wage earner flag, linked
  Staff accounts, My profile, and a masterlist CSV export. `pnpm seed` adds a 200-employee demo
  company.
- **Attendance:** work schedules with a dated history per employee, a kiosk time clock with PINs,
  My attendance, HR's attendance grid with corrections and a review queue, and the holiday
  calendar with 2026 Philippine holidays seeded.
- **Leave and overtime:** leave types with yearly balances, leave requests with approval and
  cancelling, approved leave on the attendance grid and a leave calendar, and overtime requests
  with approval. Leave, overtime, holidays, separations and schedule moves stay consistent with
  each other and lock once a payroll is posted.
- **Payroll:** semi-monthly pay periods with pre-check warnings, a payroll engine for earnings,
  SSS, PhilHealth, Pag-IBIG and withholding tax under the PH-2026 rule pack, 20 hand-worked
  golden cases, recurring allowances, loans and deduction carryovers, computing with a register,
  approve, send back with a reason, and post. Posting backs up the database and locks the
  period's attendance. A note reminds HR when a payroll is computed before its cutoff ends.
- **Payslips and reports:** My payslips for staff once a payroll is posted, payslip PDFs with the
  company logo, address and TIN, the payroll register as PDF and CSV, and a read-only rule pack
  viewer.
- **Activity log:** Admin can browse every recorded sign-in and change, filter it by day, user,
  action and record, see what changed field by field, and export the list as CSV.
- **Backups and settings:** daily and manual backups, restore, and the Settings screen for the
  company details, logo, idle timeout and backup folder.
- **Dashboards:** Admin and HR see the team today, requests to review, employees by department
  and payroll cost by period; Staff see their own day, this cutoff, leave left, latest payslip
  and pending requests.
- **App shell:** light, dark and system themes, and a Windows NSIS installer built by the
  `release` workflow.
- **Documentation:** a user manual with screenshots, the manual test script for releases, the
  release checklist, and the architecture decision records.
- **Hardening:** a content security policy on the window, a sign-in hint about the lockout after
  five wrong passwords, a dependency audit in CI, and tests that keep every command inside the
  role matrix and the Tauri config locked down.
- **License:** MIT.

[Unreleased]: https://github.com/Xyjor/Wagecraft/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/Xyjor/Wagecraft/compare/v1.0.1...v1.1.0
[1.0.1]: https://github.com/Xyjor/Wagecraft/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/Xyjor/Wagecraft/releases/tag/v1.0.0
