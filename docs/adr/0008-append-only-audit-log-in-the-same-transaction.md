# ADR-008: Append-only audit log in the same transaction

- **Status:** Accepted (revised Oct 3, 2026: hash chain moved to stretch goals)
- **Context:** HR and payroll changes must be traceable.
- **Decision:** `audit::record()` is called inside each service transaction; SQLite triggers block UPDATE and DELETE. There is no hash chain in v1. The audit log records what happens through the app; it does not protect against someone with access to the database file changing data outside the app.
- **Alternatives:** _Separate log file_: can drift from the data and is easy to edit. _Logging after commit_: a crash can lose entries. _SHA-256 hash chain with a Verify button_: detects edits made outside the app, but adds work for little gain on a single PC; it is the first stretch goal (§15).
- **Consequences:** ➕ every change and its trail succeed or fail together; simple to build and test. ➖ direct edits to the database file are not detected. ➖ the audit table only grows (about 1 KB per entry is fine for years of use).
