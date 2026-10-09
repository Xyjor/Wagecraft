# Release checklist

Run this before tagging a release, together with the [manual test script](manual-test-script.md).
It is the plan's security checklist (§8.3) and performance targets (§2) with one thing added: each
line says what proves it, so a release never depends on someone remembering to look.

## Checked by CI on every pull request and on main

| Check (plan §8.3)                                                                          | Proof                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Every command checks a permission, except sign-in, first-run setup and the kiosk clock-in. | `every_non_public_command_checks_a_permission` and `every_command_is_registered` in `src-tauri/src/commands/mod.rs` read the source of every command.                                                                                                                      |
| The RBAC test covers every command against every role.                                     | `every_command_fits_the_role_matrix` (same file) checks each permission a command asks for against Admin, HR and Staff. `matches_the_permission_matrix_in_the_plan` in `src-tauri/src/auth/permissions.rs` is the §3.2 matrix, row by row.                                 |
| No SQL is built from user input; clippy is clean.                                          | Every value reaches SQL as a bound parameter. The only `format!` calls around SQL join constant fragments, such as `format!("{SELECT} WHERE id = ?")`, and `cargo clippy --all-targets -- -D warnings` runs in CI.                                                         |
| The capabilities file grants only `core:default`.                                          | `the_webview_gets_only_core_permissions` in `src-tauri/src/release_checklist.rs`.                                                                                                                                                                                          |
| Release builds have devtools off and a content security policy.                            | `the_window_has_a_content_security_policy` and `release_builds_leave_devtools_out` in the same file. Debug builds keep devtools, which is where the manual test script's console checks run.                                                                               |
| `cargo audit` and `pnpm audit` show no high-severity issues.                               | The `audit` job in `.github/workflows/ci.yml`. An advisory that cannot apply to the shipped app is listed with its reason in `src-tauri/.cargo/audit.toml`; read that file when the job turns red to see whether a new entry belongs there or a dependency needs updating. |
| The database refuses UPDATE and DELETE on `audit_logs`.                                    | `the_database_refuses_edits_and_deletes` in `src-tauri/src/audit.rs`.                                                                                                                                                                                                      |

## By hand before each release

1. **Logs contain no personal data.** After the manual test script, open
   `C:\Users\<you>\AppData\Local\io.github.xyjor.wagecraft\logs\Wagecraft.log`. It should hold
   start-up lines, errors and backup messages, and never a name, salary, PIN or password. To see
   every line the app can write, run `rg -n "log::(info|warn|error)" src-tauri/src`.
2. **Performance on a full-size company.** The plan's targets: any screen in under 1 second,
   employee search in under 200 ms, payroll for 200 employees in under 5 seconds. `pnpm seed`
   builds a 200-employee company with a year of attendance. On the release build, open the
   attendance grid, search for an employee, compute a period and open Activity filtered by one
   user; each should feel instant. If one does not, run the query with `EXPLAIN QUERY PLAN` in
   `sqlite3` on a copy of the database and look for a `SCAN` of a large table.
3. **The installer and the release workflow.** Covered by the Releases section of the README:
   the workflow refuses a tag that disagrees with the app version, and the installer is run
   through the manual test script on a clean PC.

### Measured on Oct 9, 2026

A copy of the demo database with 49,612 attendance rows and 120,623 audit rows, timed in
`sqlite3` on the development machine:

| Query                                         | Time    | Plan                                                    |
| --------------------------------------------- | ------- | ------------------------------------------------------- |
| Attendance grid, one cutoff, 200 employees    | 0.7 ms  | index on (employee_id, day)                             |
| My attendance, one employee, one month        | 0.2 ms  | same index                                              |
| Employee list and search                      | 0.1 ms  | index on names                                          |
| Dashboard, who is present today               | 0.1 ms  | index on day                                            |
| Activity, newest page                         | 0.2 ms  | index on `at`                                           |
| Activity, one record's history                | 0.1 ms  | index on (entity_type, entity_id)                       |
| Activity, filtered by user (count of matches) | 13.6 ms | scan of `audit_logs`; acceptable, so no index was added |
| Activity, one day range and one action area   | 18.8 ms | index on `at`, then a filter                            |

The two slowest are the audit filters, and both are far below anything a person notices. Add an
index only when a measurement on a real database says so.

Recomputing the Oct 1 to 15 period for the demo company's 188 employees, in the unoptimized debug
build, finished within the same second as the click: the audit entry carries the click's second,
and a screenshot 0.2 s later already showed the new register. The target is 5 seconds.
