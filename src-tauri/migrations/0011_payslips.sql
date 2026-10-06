-- Payslips (plan §5.3, §6.5). Compute writes one per employee in the period; recompute
-- deletes and rewrites them. Money is in centavos.
CREATE TABLE payslips (
    id                     INTEGER PRIMARY KEY,
    payroll_period_id      INTEGER NOT NULL REFERENCES payroll_periods(id),
    employee_id            INTEGER NOT NULL REFERENCES employees(id),
    pay_basis              TEXT NOT NULL CHECK (pay_basis IN ('MONTHLY', 'DAILY')),
    rate_cents             INTEGER NOT NULL CHECK (rate_cents > 0),
    gross_cents            INTEGER NOT NULL,
    taxable_cents          INTEGER NOT NULL,
    statutory_ee_cents     INTEGER NOT NULL CHECK (statutory_ee_cents >= 0),
    tax_cents              INTEGER NOT NULL CHECK (tax_cents >= 0),
    other_deductions_cents INTEGER NOT NULL CHECK (other_deductions_cents >= 0),
    net_cents              INTEGER NOT NULL,
    -- Notes for HR, as a JSON array of strings.
    warnings_json          TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(warnings_json)),
    -- Everything the engine was given, so a payslip can always be explained.
    inputs_json            TEXT NOT NULL CHECK (json_valid(inputs_json)),
    UNIQUE (payroll_period_id, employee_id)
);

CREATE INDEX idx_payslips_employee ON payslips(employee_id);

CREATE TABLE payslip_lines (
    id           INTEGER PRIMARY KEY,
    payslip_id   INTEGER NOT NULL REFERENCES payslips(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN ('EARNING', 'DEDUCTION', 'EMPLOYER_SHARE')),
    code         TEXT NOT NULL,
    label        TEXT NOT NULL,
    -- Days, hours or minutes as decimal text, such as "1.5".
    quantity     TEXT NOT NULL,
    unit         TEXT CHECK (unit IN ('DAYS', 'HOURS', 'MINUTES')),
    amount_cents INTEGER NOT NULL,
    taxable      INTEGER NOT NULL CHECK (taxable IN (0, 1)),
    sort_order   INTEGER NOT NULL
);

CREATE INDEX idx_payslip_lines_payslip ON payslip_lines(payslip_id);
