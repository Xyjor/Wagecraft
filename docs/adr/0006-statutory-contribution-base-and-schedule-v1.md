# ADR-006: Statutory contribution base and schedule (v1)

- **Status:** Accepted for v1; revisit in v2
- **Context:** SSS, PhilHealth, and Pag-IBIG are monthly obligations, but pay is semi-monthly. Computing on actual monthly compensation means cutoff 1 cannot know the final amount.
- **Decision:** Base = monthly basic rate; split the employee share evenly across both cutoffs.
- **Alternatives:** _Actual compensation, all deducted in cutoff 2_: more exact, but cutoff 2's net pay drops sharply and tax withholding becomes uneven. _Projected base with month-end true-up_: most exact, but too complex for v1.
- **Consequences:** ➕ predictable, even net pay; simple to test. ➖ for employees with large monthly swings (big OT), the base can differ from actual compensation; documented as a known limitation.
