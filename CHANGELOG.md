# Changelog

What changed in each version of Wagecraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

Everything below is on `main` and goes out as 1.0.0, planned for Nov 1, 2026.

### Added

- **Sign-in and accounts:** first-run setup, sign-in with Argon2id passwords and a 15-minute
  lockout after five wrong passwords, idle timeout, Admin, HR and Staff roles, user management
  with last-Admin protection, and an audit log of every sign-in and change.
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
  period's attendance.
- **Payslips and reports:** My payslips for staff once a payroll is posted, payslip PDFs with the
  company logo, address and TIN, the payroll register as PDF and CSV, and a read-only rule pack
  viewer.
- **Backups and settings:** daily and manual backups, restore, and the Settings screen for the
  company details, logo, idle timeout and backup folder.
- **Dashboards:** Admin and HR see the team today, requests to review, employees by department
  and payroll cost by period; Staff see their own day, this cutoff, leave left, latest payslip
  and pending requests.
- **App shell:** light, dark and system themes, and a Windows NSIS installer built by the
  `release` workflow.
