-- Leave requests (plan §5.3, §6.4). Dates are company calendar days, `YYYY-MM-DD`.
-- halfdays is the working time the request takes, worked out when it's filed: rest days
-- and holidays off don't count, and a half-day request is 1.
CREATE TABLE leave_requests (
    id            INTEGER PRIMARY KEY,
    employee_id   INTEGER NOT NULL REFERENCES employees(id),
    leave_type_id INTEGER NOT NULL REFERENCES leave_types(id),
    start_date    TEXT NOT NULL,
    end_date      TEXT NOT NULL,
    half_day      INTEGER NOT NULL DEFAULT 0 CHECK (half_day IN (0, 1)),
    halfdays      INTEGER NOT NULL CHECK (halfdays > 0),
    reason        TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'PENDING'
                  CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'CANCELLED')),
    filed_by      INTEGER REFERENCES users(id),
    decided_by    INTEGER REFERENCES users(id),
    decided_at    TEXT,
    decision_note TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (end_date >= start_date),
    CHECK (half_day = 0 OR (start_date = end_date AND halfdays = 1))
);

CREATE INDEX idx_leave_requests_status ON leave_requests(status);
CREATE INDEX idx_leave_requests_employee_dates ON leave_requests(employee_id, start_date);
