-- Leave types and yearly balances (plan §5.3, §6.4). Balances count half days, so a
-- half-day leave is a whole number: 5 days is 10.

CREATE TABLE leave_types (
    id                       INTEGER PRIMARY KEY,
    code                     TEXT NOT NULL UNIQUE COLLATE NOCASE,
    name                     TEXT NOT NULL,
    is_paid                  INTEGER NOT NULL CHECK (is_paid IN (0, 1)),
    -- Granted each year. Unpaid leave uses no balance, so it stays 0.
    default_halfdays_per_year INTEGER NOT NULL DEFAULT 0 CHECK (default_halfdays_per_year >= 0),
    -- Service needed before the yearly grant: SIL needs one year (Labor Code art. 95).
    min_service_months       INTEGER NOT NULL DEFAULT 0 CHECK (min_service_months >= 0),
    is_active                INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (is_paid = 1 OR default_halfdays_per_year = 0)
);

CREATE TABLE leave_balances (
    id                INTEGER PRIMARY KEY,
    employee_id       INTEGER NOT NULL REFERENCES employees(id),
    leave_type_id     INTEGER NOT NULL REFERENCES leave_types(id),
    year              INTEGER NOT NULL CHECK (year BETWEEN 2000 AND 2100),
    entitled_halfdays INTEGER NOT NULL CHECK (entitled_halfdays >= 0),
    used_halfdays     INTEGER NOT NULL DEFAULT 0 CHECK (used_halfdays >= 0),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (employee_id, leave_type_id, year),
    -- The database itself refuses a negative balance (plan §6.4).
    CHECK (used_halfdays <= entitled_halfdays)
);

-- The plan's four types. Vacation and sick leave are company policy, not law: Admin can
-- change their yearly days.
INSERT INTO leave_types (code, name, is_paid, default_halfdays_per_year, min_service_months) VALUES
    ('SIL',  'Service Incentive Leave', 1, 10, 12),
    ('VL',   'Vacation Leave',          1, 10, 0),
    ('SL',   'Sick Leave',              1, 10, 0),
    ('LWOP', 'Leave Without Pay',       0, 0,  0);
