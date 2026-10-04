-- Which work schedule an employee follows from which date. Each row applies from its
-- effective_from until the next row's, so a past day is always worked out with the
-- schedule it had, even after HR moves the employee to another one.
-- schedule_id NULL means no schedule from that date.
CREATE TABLE schedule_assignments (
    id             INTEGER PRIMARY KEY,
    employee_id    INTEGER NOT NULL REFERENCES employees(id),
    schedule_id    INTEGER REFERENCES work_schedules(id),
    effective_from TEXT NOT NULL,
    reason         TEXT,
    created_by     INTEGER REFERENCES users(id),
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (employee_id, effective_from)
);

-- Everyone keeps the schedule they have now, from their hire date.
INSERT INTO schedule_assignments (employee_id, schedule_id, effective_from, reason)
SELECT id, schedule_id, hire_date, 'Schedule when history started'
FROM employees WHERE schedule_id IS NOT NULL;

-- employees.schedule_id is now only the schedule someone starts with. A new employee's
-- first assignment runs from their hire date.
CREATE TRIGGER employees_first_schedule AFTER INSERT ON employees
WHEN NEW.schedule_id IS NOT NULL
BEGIN
    INSERT INTO schedule_assignments (employee_id, schedule_id, effective_from, reason)
    VALUES (NEW.id, NEW.schedule_id, NEW.hire_date, 'Hired');
END;
