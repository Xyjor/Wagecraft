# Changelog

What changed in each version of Wagecraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- New "Ledger" look, part 1: green-tinted neutrals, ledger green for success and focus, red ink for
  errors, and the IBM Plex Sans typeface bundled with the app. See
  [ADR-013](docs/adr/0013-ledger-design-tokens.md).
- The sidebar is grouped (Me, People, Time and leave, Payroll, Admin), with icons, and collapses to
  an icon rail; the choice is remembered. A section stays highlighted on its detail pages.
- The theme switch, Change password and Sign out moved into an account menu under your name at
  the top right.
- The window can't be made smaller than 1024 × 640.
- Buttons, form fields, badges and the sign-in card use the Ledger colours: ledger green for the
  main action on each screen, outlined buttons for second actions, and a status badge per payroll
  step (draft amber, approved green, posted solid green).
- The dashboard and payroll screens use the ledger look. Late arrivals show in red ink and people
  not clocked in show in amber. The charts follow Light and Dark. The payroll register has ruled
  money columns with deductions in red and a double rule above the totals. Negative amounts show
  as (₱1,149.43) on screen. A dashboard that fails to load offers Retry.
- The employee and attendance screens use the ledger look, with page headers and breadcrumbs
  (Employees › name › Edit). Late and undertime minutes show in red ink.

### Fixed

- The sign-in, setup, time clock and forced password-change screens now follow the Light, Dark or
  System choice, and the app no longer flashes white before dark mode applies.

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

[Unreleased]: https://github.com/Xyjor/Wagecraft/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/Xyjor/Wagecraft/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/Xyjor/Wagecraft/releases/tag/v1.0.0
