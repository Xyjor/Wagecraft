-- Attendance (plan §5.3, §6.3). Times are company-local wall-clock text,
-- `YYYY-MM-DDTHH:MM:SS`; work_date is the day the shift starts.
CREATE TABLE attendance_records (
    id                  INTEGER PRIMARY KEY,
    employee_id         INTEGER NOT NULL REFERENCES employees(id),
    work_date           TEXT NOT NULL,
    time_in             TEXT,
    time_out            TEXT,
    status              TEXT NOT NULL
                        CHECK (status IN ('PRESENT', 'ABSENT', 'ON_LEAVE', 'REST_DAY', 'HOLIDAY')),
    late_minutes        INTEGER NOT NULL DEFAULT 0 CHECK (late_minutes >= 0),
    undertime_minutes   INTEGER NOT NULL DEFAULT 0 CHECK (undertime_minutes >= 0),
    worked_minutes      INTEGER NOT NULL DEFAULT 0 CHECK (worked_minutes >= 0),
    night_minutes       INTEGER NOT NULL DEFAULT 0 CHECK (night_minutes >= 0),
    source              TEXT NOT NULL CHECK (source IN ('CLOCK', 'MANUAL')),
    -- Set when something about the punch needs an HR look, such as the PC clock going backwards.
    needs_review        INTEGER NOT NULL DEFAULT 0 CHECK (needs_review IN (0, 1)),
    review_note         TEXT,
    edit_reason         TEXT,
    edited_by           INTEGER REFERENCES users(id),
    -- Points at payroll_periods once payroll lands; a posted period locks its records.
    locked_by_period_id INTEGER,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (employee_id, work_date)
);

CREATE INDEX idx_attendance_work_date ON attendance_records(work_date);
