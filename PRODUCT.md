# Product

<!-- impeccable:product-schema 1 -->

_Written from the README, the plan and the user manual. Facts marked (inferred) still need the
owner's confirmation._

## Platform

web

(Desktop app built with Tauri 2: a web UI in WebView2 on Windows.)

## Users

- **HR** keeps employee records, attendance, leave and overtime, and runs semi-monthly payroll.
  This is the main daily user, at a desk in a small office.
- **Admin** sets the company up and manages accounts, settings, backups and the audit log. Admins
  can also do everything HR can.
- **Staff** check their own attendance, leave balances and payslips, and file leave and overtime.
  They use the app now and then, often on the same shared PC.
- **The time clock** is used by every employee to clock in and out with a PIN, with no sign-in.

## Product Purpose

Wagecraft runs a small Philippine company's attendance, leave and payroll on one Windows PC,
offline. Payroll computes SSS, PhilHealth, Pag-IBIG and withholding tax under dated rule packs,
correct to the centavo. Success means HR can close a cutoff without a spreadsheet, and every
figure can be traced back to its source.

## Positioning

Everything runs on one machine, with no server and no subscription. It is built for Philippine
statutory payroll: semi-monthly cutoffs, the government contribution tables, and BIR withholding.
Every change is audited.

## Operating Context

- An office PC, in daylight or under fluorescent light. The shared PC doubles as a time clock.
- HR works through cutoffs. For each pay period: create it, compute, approve, then post. The
  outputs are payslip PDFs and the payroll register (PDF and CSV).
- The vocabulary is accounting and paper records: the register, payslips, the masterlist, and
  leave forms.

## Capabilities and Constraints

- Offline. The production CSP allows only same-origin scripts, styles, fonts and images. Nothing
  may load from a CDN.
- Role-based access is enforced in Rust. The UI only hides what a role can't use.
- Money is held as integer centavos.
- v1 has no 13th-month pay (a known gap). This is a training project, and the rates must be
  checked against official issuances before anyone uses it to pay real people.

## Brand Commitments

- The name is Wagecraft. There is no logo yet.
- The interface voice is plain and direct, in sentence case, as used in the user manual
  (inferred).

## Evidence on Hand

- The demo company from `pnpm seed`: 200 employees.
- The user manual and its 29 screenshots in `docs/screenshots/`.
- There are no customers, testimonials or published benchmarks. Don't invent any.

## Product Principles

1. Payroll figures must be correct and traceable. Never trade clarity for decoration.
2. Each role sees only what it can act on.
3. Work fully offline on one modest PC.
4. Show plainly what changed, who changed it, and what happens next.

## Accessibility & Inclusion

Target WCAG 2.1 AA:

- contrast checked in both themes
- visible focus rings
- every input labelled
- status never shown by colour alone
- full keyboard use
