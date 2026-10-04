-- Payroll rule packs (plan §5.3, ADR-005): the rates and tables a payroll period is
-- computed with. New government rates arrive as a new pack in a new migration; a pack
-- a posted period used is never edited. Money is in centavos, rates in basis points.
CREATE TABLE rule_packs (
    id             INTEGER PRIMARY KEY,
    code           TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    effective_from TEXT NOT NULL,
    effective_to   TEXT,
    is_active      INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    -- Rate factors, SSS shares and the PhilHealth and Pag-IBIG rules, as JSON.
    settings_json  TEXT NOT NULL CHECK (json_valid(settings_json)),
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (effective_to IS NULL OR effective_to >= effective_from)
);

CREATE TABLE contribution_brackets (
    id               INTEGER PRIMARY KEY,
    rule_pack_id     INTEGER NOT NULL REFERENCES rule_packs(id),
    agency           TEXT NOT NULL CHECK (agency IN ('SSS')),
    range_from_cents INTEGER NOT NULL CHECK (range_from_cents >= 0),
    range_to_cents   INTEGER,  -- inclusive; NULL for the top bracket
    base_cents       INTEGER NOT NULL CHECK (base_cents > 0),  -- the Monthly Salary Credit
    ee_cents         INTEGER NOT NULL CHECK (ee_cents >= 0),
    er_cents         INTEGER NOT NULL CHECK (er_cents >= 0),
    ec_cents         INTEGER NOT NULL CHECK (ec_cents >= 0),
    UNIQUE (rule_pack_id, agency, range_from_cents),
    CHECK (range_to_cents IS NULL OR range_to_cents >= range_from_cents)
);

CREATE TABLE tax_brackets (
    id             INTEGER PRIMARY KEY,
    rule_pack_id   INTEGER NOT NULL REFERENCES rule_packs(id),
    frequency      TEXT NOT NULL CHECK (frequency IN ('SEMI_MONTHLY', 'MONTHLY')),
    over_cents     INTEGER NOT NULL CHECK (over_cents >= 0),
    not_over_cents INTEGER,  -- NULL for the top bracket
    base_tax_cents INTEGER NOT NULL CHECK (base_tax_cents >= 0),
    rate_bp        INTEGER NOT NULL CHECK (rate_bp BETWEEN 0 AND 10000),
    UNIQUE (rule_pack_id, frequency, over_cents),
    CHECK (not_over_cents IS NULL OR not_over_cents > over_cents)
);

CREATE TABLE premium_rates (
    id            INTEGER PRIMARY KEY,
    rule_pack_id  INTEGER NOT NULL REFERENCES rule_packs(id),
    day_type      TEXT NOT NULL CHECK (day_type IN (
                      'ORDINARY', 'REST_DAY', 'SPECIAL', 'SPECIAL_REST_DAY', 'REGULAR',
                      'REGULAR_REST_DAY', 'DOUBLE_REGULAR', 'DOUBLE_REGULAR_REST_DAY')),
    work_bp       INTEGER NOT NULL CHECK (work_bp > 0),
    ot_bp         INTEGER NOT NULL CHECK (ot_bp > 0),
    night_diff_bp INTEGER NOT NULL CHECK (night_diff_bp >= 0),
    UNIQUE (rule_pack_id, day_type)
);

-- PH-2026 (plan Appendix A). Verify against the agencies' issuances before real use.
INSERT INTO rule_packs (code, name, effective_from, settings_json) VALUES (
    'PH-2026',
    'Philippines 2026',
    '2026-01-01',
    json('{
        "factorFiveDay": 261,
        "factorSixDay": 313,
        "sssEmployeeBp": 500,
        "sssEmployerBp": 1000,
        "philhealth": {"rateBp": 500, "floorCents": 1000000, "ceilingCents": 10000000},
        "pagibig": {
            "lowPayLimitCents": 150000,
            "lowRateBp": 100,
            "employeeBp": 200,
            "employerBp": 200,
            "maxBaseCents": 1000000
        }
    }')
);

-- SSS: MSC ₱5,000 to ₱35,000 in ₱500 steps (61 brackets). Each band runs from ₱250
-- below its MSC to ₱249.99 above; EC is ₱10 below an MSC of ₱15,000 and ₱30 from there.
WITH RECURSIVE msc(v) AS (
    SELECT 500000
    UNION ALL
    SELECT v + 50000 FROM msc WHERE v < 3500000
)
INSERT INTO contribution_brackets
    (rule_pack_id, agency, range_from_cents, range_to_cents, base_cents, ee_cents, er_cents, ec_cents)
SELECT (SELECT id FROM rule_packs WHERE code = 'PH-2026'),
       'SSS',
       CASE WHEN v = 500000 THEN 0 ELSE v - 25000 END,
       CASE WHEN v = 3500000 THEN NULL ELSE v + 24999 END,
       v,
       v * 5 / 100,
       v * 10 / 100,
       CASE WHEN v < 1500000 THEN 1000 ELSE 3000 END
FROM msc;

-- BIR withholding tax on compensation (TRAIN law, from January 1, 2023).
INSERT INTO tax_brackets (rule_pack_id, frequency, over_cents, not_over_cents, base_tax_cents, rate_bp)
SELECT (SELECT id FROM rule_packs WHERE code = 'PH-2026'), f, o, n, b, r
FROM (
    SELECT 'SEMI_MONTHLY' AS f, 0 AS o, 1041700 AS n, 0 AS b, 0 AS r
    UNION ALL SELECT 'SEMI_MONTHLY', 1041700, 1666700, 0, 1500
    UNION ALL SELECT 'SEMI_MONTHLY', 1666700, 3333300, 93750, 2000
    UNION ALL SELECT 'SEMI_MONTHLY', 3333300, 8333300, 427070, 2500
    UNION ALL SELECT 'SEMI_MONTHLY', 8333300, 33333300, 1677070, 3000
    UNION ALL SELECT 'SEMI_MONTHLY', 33333300, NULL, 9177070, 3500
    UNION ALL SELECT 'MONTHLY', 0, 2083300, 0, 0
    UNION ALL SELECT 'MONTHLY', 2083300, 3333300, 0, 1500
    UNION ALL SELECT 'MONTHLY', 3333300, 6666700, 187500, 2000
    UNION ALL SELECT 'MONTHLY', 6666700, 16666700, 854180, 2500
    UNION ALL SELECT 'MONTHLY', 16666700, 66666700, 3354180, 3000
    UNION ALL SELECT 'MONTHLY', 66666700, NULL, 18354180, 3500
);

-- DOLE premium pay (plan §7.3), with a 10% night differential on every day type.
INSERT INTO premium_rates (rule_pack_id, day_type, work_bp, ot_bp, night_diff_bp)
SELECT (SELECT id FROM rule_packs WHERE code = 'PH-2026'), d, w, o, 1000
FROM (
    SELECT 'ORDINARY' AS d, 10000 AS w, 12500 AS o
    UNION ALL SELECT 'REST_DAY', 13000, 16900
    UNION ALL SELECT 'SPECIAL', 13000, 16900
    UNION ALL SELECT 'SPECIAL_REST_DAY', 15000, 19500
    UNION ALL SELECT 'REGULAR', 20000, 26000
    UNION ALL SELECT 'REGULAR_REST_DAY', 26000, 33800
    UNION ALL SELECT 'DOUBLE_REGULAR', 30000, 39000
    UNION ALL SELECT 'DOUBLE_REGULAR_REST_DAY', 39000, 50700
);
