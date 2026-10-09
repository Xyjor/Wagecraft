# Architecture decision records

One file per decision, numbered in the order they were made. A decision that changes gets a new
ADR that supersedes the old one; the old file stays as history. The first eleven come from
[the plan](../wagecraft-plan.md#9-architecture-decision-records), §9.

| ADR                                                                          | Decision                                                               |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| [0001](0001-tauri-2-as-the-desktop-framework.md)                             | Tauri 2 as the desktop framework                                       |
| [0002](0002-all-business-logic-and-authorization-in-rust-no-sql-from-the.md) | All business logic and authorization in Rust; no SQL from the frontend |
| [0003](0003-embedded-sqlite-via-sqlx-single-machine-scope.md)                | Embedded SQLite via sqlx; single-machine scope                         |
| [0004](0004-money-as-integer-centavos-math-with-rust-decimal.md)             | Money as integer centavos, math with rust_decimal                      |
| [0005](0005-payroll-rules-as-data-rule-packs-with-effective-dates.md)        | Payroll rules as data (rule packs with effective dates)                |
| [0006](0006-statutory-contribution-base-and-schedule-v1.md)                  | Statutory contribution base and schedule (v1)                          |
| [0007](0007-sessions-held-in-rust-memory-with-idle-timeout.md)               | Sessions held in Rust memory with idle timeout                         |
| [0008](0008-append-only-audit-log-in-the-same-transaction.md)                | Append-only audit log in the same transaction                          |
| [0009](0009-backup-with-vacuum-into-restore-by-file-swap-and-app-restart.md) | Backup with VACUUM INTO; restore by file swap and app restart          |
| [0010](0010-pdf-templates-in-react-file-writing-in-rust.md)                  | PDF templates in React; file writing in Rust                           |
| [0011](0011-react-typescript-tanstack-query-shadcn-ui.md)                    | React + TypeScript + TanStack Query + shadcn/ui                        |
| [0012](0012-plain-react-data-loading-and-forms-tailwind-without-shadcn.md)   | Plain React data loading and forms; Tailwind without shadcn/ui         |
