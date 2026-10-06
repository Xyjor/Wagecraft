-- Recurring allowances, loans and other deductions (plan §5.3, §7.3 step 4, §7.6).
-- Compute picks up every item in effect during the period and scheduled for its cutoff.
-- Money is in centavos.
--
-- 0003 made a first version of this table that nothing ever wrote to. It is replaced so
-- a loan is its own kind (instead of a DEDUCTION with a LOAN code) and so the database
-- refuses a loan with no balance or a taxable deduction.
DROP TABLE recurring_items;

CREATE TABLE recurring_items (
    id                      INTEGER PRIMARY KEY,
    employee_id             INTEGER NOT NULL REFERENCES employees(id),
    kind                    TEXT NOT NULL CHECK (kind IN ('ALLOWANCE', 'LOAN', 'DEDUCTION')),
    label                   TEXT NOT NULL CHECK (length(trim(label)) > 0),
    -- Per cutoff it applies to.
    amount_cents            INTEGER NOT NULL CHECK (amount_cents > 0),
    -- Only allowances can be taxable.
    taxable                 INTEGER NOT NULL DEFAULT 0 CHECK (taxable IN (0, 1)),
    schedule                TEXT NOT NULL
                            CHECK (schedule IN ('EVERY_CUTOFF', 'FIRST_CUTOFF', 'SECOND_CUTOFF')),
    start_date              TEXT NOT NULL,
    end_date                TEXT,
    -- What is still owed on a loan. Posting a period takes each cutoff's amount off it.
    remaining_balance_cents INTEGER CHECK (remaining_balance_cents >= 0),
    created_by              INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at              TEXT NOT NULL,
    CHECK (end_date IS NULL OR end_date >= start_date),
    CHECK ((kind = 'LOAN') = (remaining_balance_cents IS NOT NULL)),
    CHECK (kind = 'ALLOWANCE' OR taxable = 0)
);

CREATE INDEX idx_recurring_items_employee ON recurring_items(employee_id);

-- The part of a deduction that didn't fit in a payslip (plan §7.6). The next period's
-- compute takes open carryovers first and records which payslip took them. Recomputing
-- that payslip deletes it, which reopens the carryover.
CREATE TABLE deduction_carryovers (
    id                 INTEGER PRIMARY KEY,
    employee_id        INTEGER NOT NULL REFERENCES employees(id),
    source_payslip_id  INTEGER NOT NULL REFERENCES payslips(id) ON DELETE CASCADE,
    recurring_item_id  INTEGER REFERENCES recurring_items(id) ON DELETE SET NULL,
    code               TEXT NOT NULL CHECK (code IN ('LOAN', 'DEDUCTION')),
    label              TEXT NOT NULL,
    amount_cents       INTEGER NOT NULL CHECK (amount_cents > 0),
    applied_payslip_id INTEGER REFERENCES payslips(id) ON DELETE SET NULL
);

CREATE INDEX idx_carryovers_employee ON deduction_carryovers(employee_id);
CREATE INDEX idx_carryovers_source ON deduction_carryovers(source_payslip_id);
CREATE INDEX idx_carryovers_applied ON deduction_carryovers(applied_payslip_id);
