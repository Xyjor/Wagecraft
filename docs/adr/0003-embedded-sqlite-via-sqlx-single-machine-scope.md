# ADR-003: Embedded SQLite via sqlx; single-machine scope

- **Status:** Accepted
- **Context:** The app must use a local database and work offline.
- **Decision:** One SQLite file in the app data folder, accessed with sqlx, in WAL mode, with versioned SQL migrations.
- **Alternatives:** _rusqlite_ is simpler and synchronous but has no built-in migrations or async. _Diesel/SeaORM_ are full ORMs with more to learn and hide the SQL. _PostgreSQL_ needs a server, which breaks the local-only constraint.
- **Consequences:** ➕ zero setup, transactional, easy backups (one file). ➖ data lives on one PC; sharing across several PCs would need a server-mode redesign (future ADR). ➖ no encryption at rest by default; SQLCipher is a possible later upgrade.
