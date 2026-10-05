-- Payroll periods (plan §5.3, §6.5). A period is one semi-monthly cutoff: the 1st to the
-- 15th, or the 16th to the end of the month. It moves DRAFT → COMPUTED → APPROVED → POSTED,
-- and a posted period is read-only forever.
CREATE TABLE payroll_periods (
    id           INTEGER PRIMARY KEY,
    period_start TEXT NOT NULL UNIQUE,
    period_end   TEXT NOT NULL,
    pay_date     TEXT NOT NULL,
    frequency    TEXT NOT NULL DEFAULT 'SEMI_MONTHLY' CHECK (frequency IN ('SEMI_MONTHLY')),
    cutoff_no    INTEGER NOT NULL CHECK (cutoff_no IN (1, 2)),
    status       TEXT NOT NULL DEFAULT 'DRAFT'
                 CHECK (status IN ('DRAFT', 'COMPUTED', 'APPROVED', 'POSTED')),
    rule_pack_id INTEGER NOT NULL REFERENCES rule_packs(id),
    created_by   INTEGER REFERENCES users(id),
    created_at   TEXT NOT NULL,
    computed_by  INTEGER REFERENCES users(id),
    computed_at  TEXT,
    approved_by  INTEGER REFERENCES users(id),
    approved_at  TEXT,
    posted_by    INTEGER REFERENCES users(id),
    posted_at    TEXT,
    CHECK (period_end > period_start),
    CHECK (pay_date >= period_end)
);
