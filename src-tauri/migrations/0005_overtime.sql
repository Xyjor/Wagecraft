-- Overtime requests (plan §5.3, §6.3). Overtime is paid only once approved.
-- start_at and end_at are company-local wall-clock text, `YYYY-MM-DDTHH:MM:SS`, like
-- attendance; work_date is the day the overtime belongs to.
CREATE TABLE overtime_requests (
    id            INTEGER PRIMARY KEY,
    employee_id   INTEGER NOT NULL REFERENCES employees(id),
    work_date     TEXT NOT NULL,
    start_at      TEXT NOT NULL,
    end_at        TEXT NOT NULL,
    minutes       INTEGER NOT NULL CHECK (minutes > 0 AND minutes <= 720),
    reason        TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'PENDING'
                  CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'CANCELLED')),
    filed_by      INTEGER REFERENCES users(id),
    decided_by    INTEGER REFERENCES users(id),
    decided_at    TEXT,
    decision_note TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (end_at > start_at)
);

CREATE INDEX idx_overtime_status ON overtime_requests(status);
CREATE INDEX idx_overtime_employee_date ON overtime_requests(employee_id, work_date);
