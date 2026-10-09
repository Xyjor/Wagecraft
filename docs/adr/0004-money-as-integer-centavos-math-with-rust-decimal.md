# ADR-004: Money as integer centavos, math with rust_decimal

- **Status:** Accepted
- **Context:** Floating-point numbers cannot represent most decimal amounts exactly (0.1 + 0.2 ≠ 0.3), which causes centavo errors in payroll.
- **Decision:** Store amounts as `INTEGER` centavos; compute with `Decimal`; round each line explicitly, half away from zero, to 2 decimal places.
- **Alternatives:** _f64_ (rounding drift), _REAL columns_ (same problem in the database), _integers only_ (correct, but harder to read in rate formulas).
- **Consequences:** ➕ exact, reproducible results. ➖ conversions at the edges (`money.ts` and `Decimal::new(cents, 2)`).
