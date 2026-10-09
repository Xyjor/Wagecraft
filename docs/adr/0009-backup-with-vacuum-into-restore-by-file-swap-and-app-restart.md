# ADR-009: Backup with VACUUM INTO; restore by file swap and app restart

- **Status:** Accepted
- **Context:** In WAL mode, copying the database file while the app runs can produce a corrupt backup.
- **Decision:** Back up with `VACUUM INTO`; restore by validating, closing the pool, replacing the file, and restarting the app so migrations run cleanly.
- **Alternatives:** _Plain file copy_ (unsafe in WAL mode), _SQLite online backup API_ (fine, but not exposed by sqlx), _hot-swapping the pool without a restart_ (more complex state handling).
- **Consequences:** ➕ consistent backups, simple and safe restore. ➖ the app restarts on restore (acceptable for a rare admin action).
