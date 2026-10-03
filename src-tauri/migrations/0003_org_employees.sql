-- Organization and employee records (plan §5.3, Phase 2).
--
-- users.employee_id gets its integrity from triggers instead of a foreign key. Adding a
-- foreign key would mean rebuilding `users` with foreign keys switched off, and the audit
-- log already points at it. Employees are never deleted (plan §5.1), so checking inserts
-- and updates on `users` is enough.

CREATE TABLE departments (
    id          INTEGER PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    name        TEXT NOT NULL,
    description TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE positions (
    id             INTEGER PRIMARY KEY,
    department_id  INTEGER NOT NULL REFERENCES departments(id),
    title          TEXT NOT NULL COLLATE NOCASE,
    -- Optional salary range, as a monthly rate. A rate outside it is a warning, not an error.
    min_rate_cents INTEGER CHECK (min_rate_cents >= 0),
    max_rate_cents INTEGER CHECK (max_rate_cents >= 0),
    is_active      INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (department_id, title),
    CHECK (min_rate_cents IS NULL OR max_rate_cents IS NULL OR min_rate_cents <= max_rate_cents)
);

CREATE TABLE work_schedules (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE COLLATE NOCASE,
    start_time    TEXT NOT NULL,  -- 08:00, company local time
    end_time      TEXT NOT NULL,  -- 17:00; earlier than start_time means it ends the next day
    break_minutes INTEGER NOT NULL DEFAULT 60 CHECK (break_minutes >= 0),
    grace_minutes INTEGER NOT NULL DEFAULT 0 CHECK (grace_minutes >= 0),
    work_days     TEXT NOT NULL,  -- MON,TUE,WED,THU,FRI
    is_active     INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- A double regular holiday is two rows on the same date.
CREATE TABLE holidays (
    id         INTEGER PRIMARY KEY,
    date       TEXT NOT NULL,
    name       TEXT NOT NULL,
    type       TEXT NOT NULL CHECK (type IN ('REGULAR', 'SPECIAL_NON_WORKING', 'SPECIAL_WORKING')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (date, name)
);

CREATE TABLE employees (
    id                  INTEGER PRIMARY KEY,
    employee_no         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    first_name          TEXT NOT NULL,
    middle_name         TEXT,
    last_name           TEXT NOT NULL,
    suffix              TEXT,
    birth_date          TEXT,
    sex                 TEXT CHECK (sex IN ('MALE', 'FEMALE')),
    civil_status        TEXT CHECK (civil_status IN ('SINGLE', 'MARRIED', 'WIDOWED', 'SEPARATED')),
    email               TEXT,
    mobile              TEXT,
    address             TEXT,
    hire_date           TEXT NOT NULL,
    regularization_date TEXT,
    separation_date     TEXT,
    employment_status   TEXT NOT NULL CHECK (employment_status IN
                          ('PROBATIONARY', 'REGULAR', 'CONTRACTUAL', 'RESIGNED', 'TERMINATED')),
    department_id       INTEGER REFERENCES departments(id),
    position_id         INTEGER REFERENCES positions(id),
    schedule_id         INTEGER REFERENCES work_schedules(id),
    -- Government IDs and bank account, stored as digits only.
    tin                 TEXT UNIQUE,
    sss_no              TEXT UNIQUE,
    philhealth_no       TEXT UNIQUE,
    pagibig_no          TEXT UNIQUE,
    bank_name           TEXT,
    bank_account_no     TEXT,
    photo_path          TEXT,
    kiosk_pin_hash      TEXT,
    archived_at         TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Salary history. A change adds a row and closes the previous one; nothing is overwritten.
CREATE TABLE compensations (
    id             INTEGER PRIMARY KEY,
    employee_id    INTEGER NOT NULL REFERENCES employees(id),
    pay_basis      TEXT NOT NULL CHECK (pay_basis IN ('MONTHLY', 'DAILY')),
    rate_cents     INTEGER NOT NULL CHECK (rate_cents > 0),
    effective_from TEXT NOT NULL,
    effective_to   TEXT,
    reason         TEXT,
    created_by     INTEGER REFERENCES users(id),
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (employee_id, effective_from),
    CHECK (effective_to IS NULL OR effective_to >= effective_from)
);

CREATE TABLE recurring_items (
    id                      INTEGER PRIMARY KEY,
    employee_id             INTEGER NOT NULL REFERENCES employees(id),
    kind                    TEXT NOT NULL CHECK (kind IN ('ALLOWANCE', 'DEDUCTION')),
    code                    TEXT NOT NULL,
    label                   TEXT NOT NULL,
    amount_cents            INTEGER NOT NULL CHECK (amount_cents > 0),
    taxable                 INTEGER NOT NULL DEFAULT 0 CHECK (taxable IN (0, 1)),
    schedule                TEXT NOT NULL CHECK (schedule IN ('EVERY_CUTOFF', 'FIRST_CUTOFF', 'SECOND_CUTOFF')),
    start_date              TEXT NOT NULL,
    end_date                TEXT,
    remaining_balance_cents INTEGER CHECK (remaining_balance_cents >= 0),  -- loans only
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (end_date IS NULL OR end_date >= start_date)
);

CREATE INDEX idx_employees_name       ON employees(last_name, first_name);
CREATE INDEX idx_employees_department ON employees(department_id);
CREATE INDEX idx_employees_status     ON employees(employment_status);
CREATE INDEX idx_recurring_employee   ON recurring_items(employee_id);

-- Employees are archived, never deleted, so payroll history always has its people.
CREATE TRIGGER employees_no_delete BEFORE DELETE ON employees
BEGIN SELECT RAISE(ABORT, 'employees are archived, not deleted'); END;

-- users.employee_id must name a real employee (see the note at the top).
CREATE TRIGGER users_employee_exists_insert BEFORE INSERT ON users
WHEN NEW.employee_id IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM employees WHERE id = NEW.employee_id)
BEGIN SELECT RAISE(ABORT, 'users.employee_id must name an existing employee'); END;

CREATE TRIGGER users_employee_exists_update BEFORE UPDATE OF employee_id ON users
WHEN NEW.employee_id IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM employees WHERE id = NEW.employee_id)
BEGIN SELECT RAISE(ABORT, 'users.employee_id must name an existing employee'); END;
