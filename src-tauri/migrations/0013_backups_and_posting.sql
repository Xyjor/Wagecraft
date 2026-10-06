-- Backups (plan §5.3, §6.7). Posting a pay period takes one first (PRE_POST), so the
-- database can be put back exactly as it was. Manual and daily backups come with the
-- backup screen.
CREATE TABLE backup_history (
    id         INTEGER PRIMARY KEY,
    path       TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    kind       TEXT NOT NULL CHECK (kind IN ('AUTO', 'MANUAL', 'PRE_RESTORE', 'PRE_POST')),
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL
);

-- Posting locks a period's attendance by setting locked_by_period_id (plan §6.5).
CREATE INDEX idx_attendance_locked ON attendance_records(locked_by_period_id);
