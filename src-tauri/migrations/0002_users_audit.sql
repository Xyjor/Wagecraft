-- Users, roles and the append-only audit log (plan §5.3, §5.4).
-- users.employee_id will point at employees(id). That table arrives with the employee
-- module, whose migration rebuilds this table to add the foreign key: with foreign keys on,
-- SQLite refuses any insert into a table whose REFERENCES target doesn't exist yet.

CREATE TABLE users (
    id                   INTEGER PRIMARY KEY,
    username             TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash        TEXT NOT NULL,
    role                 TEXT NOT NULL CHECK (role IN ('ADMIN', 'HR', 'STAFF')),
    employee_id          INTEGER UNIQUE,
    is_active            INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    must_change_password INTEGER NOT NULL DEFAULT 1 CHECK (must_change_password IN (0, 1)),
    failed_attempts      INTEGER NOT NULL DEFAULT 0,
    locked_until         TEXT,
    theme                TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (theme IN ('LIGHT', 'DARK', 'SYSTEM')),
    last_login_at        TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE audit_logs (
    id             INTEGER PRIMARY KEY,
    at             TEXT NOT NULL,
    actor_user_id  INTEGER REFERENCES users(id),
    actor_username TEXT NOT NULL,
    action         TEXT NOT NULL,
    entity_type    TEXT,
    entity_id      INTEGER,
    before_json    TEXT,
    after_json     TEXT
);

-- The audit log is append-only: the database itself refuses edits and deletes.
CREATE TRIGGER audit_logs_no_update BEFORE UPDATE ON audit_logs
BEGIN SELECT RAISE(ABORT, 'audit_logs is append-only'); END;

CREATE TRIGGER audit_logs_no_delete BEFORE DELETE ON audit_logs
BEGIN SELECT RAISE(ABORT, 'audit_logs is append-only'); END;

CREATE INDEX idx_audit_logs_at    ON audit_logs(at);
CREATE INDEX idx_audit_logs_owner ON audit_logs(entity_type, entity_id);
