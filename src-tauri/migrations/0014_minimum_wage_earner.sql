-- Minimum wage earners have no tax withheld (plan §7.5). The flag sits on the pay rate,
-- not the employee, because it follows the rate: a raise above the minimum wage ends it,
-- and an old period recomputes with the flag it had then.
ALTER TABLE compensations ADD COLUMN minimum_wage_earner INTEGER NOT NULL DEFAULT 0
    CHECK (minimum_wage_earner IN (0, 1));
