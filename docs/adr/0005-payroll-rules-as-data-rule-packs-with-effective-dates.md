# ADR-005: Payroll rules as data (rule packs with effective dates)

- **Status:** Accepted
- **Context:** Contribution rates and tax tables change by law; the trainer did not mandate a specific country's rules.
- **Decision:** Keep all rates, brackets, and factors in rule-pack tables. Ship a **PH-2026** pack as the default. Each payroll period records which pack it used.
- **Alternatives:** _Hard-coded constants_: simpler at first, but every legal change becomes a code change and a new release, and old payrolls cannot be recomputed with old rules.
- **Consequences:** ➕ new rates are a data update, shipped as a new migration, with no engine code change; history stays reproducible; another country's rules could be added as a new pack. ➖ seed migrations are needed. v1 has a read-only rule-pack viewer; an editor screen is a stretch goal.
