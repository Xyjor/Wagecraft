//! Step 6 of the pipeline (plan §7.5): withholding tax on compensation, from the rule
//! pack's semi-monthly table.

use super::rates::round_line;
use super::rules::RulePack;
use rust_decimal::Decimal;

/// Tax on `taxable` pesos of semi-monthly compensation: base tax plus the rate on the
/// excess over the bracket's lower limit. Nothing is withheld on zero or less.
pub fn semi_monthly(taxable: Decimal, rules: &RulePack) -> Decimal {
    let Some(bracket) = rules
        .tax_semi_monthly
        .iter()
        .rev()
        .find(|b| taxable > b.over)
    else {
        return Decimal::ZERO;
    };
    round_line(bracket.base_tax + (taxable - bracket.over) * bracket.rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tax(taxable: Decimal) -> Decimal {
        semi_monthly(taxable, &RulePack::ph_2026())
    }

    /// Plan §7.7: (10,780.02 − 10,417) × 15% = 54.453.
    #[test]
    fn the_worked_example_tax() {
        assert_eq!(tax(dec!(10780.02)), dec!(54.45));
    }

    /// One amount inside each bracket of plan Appendix A.5, worked out by hand.
    #[test]
    fn each_bracket_applies_its_base_and_rate() {
        assert_eq!(tax(dec!(-50)), dec!(0));
        assert_eq!(tax(dec!(10417)), dec!(0));
        assert_eq!(tax(dec!(10418)), dec!(0.15));
        // 937.50 + (20,000 − 16,667) × 20% = 937.50 + 666.60
        assert_eq!(tax(dec!(20000)), dec!(1604.10));
        // 4,270.70 + (50,000 − 33,333) × 25% = 4,270.70 + 4,166.75
        assert_eq!(tax(dec!(50000)), dec!(8437.45));
        // 16,770.70 + (100,000 − 83,333) × 30% = 16,770.70 + 5,000.10
        assert_eq!(tax(dec!(100000)), dec!(21770.80));
        // 91,770.70 + (400,000 − 333,333) × 35% = 91,770.70 + 23,333.45
        assert_eq!(tax(dec!(400000)), dec!(115104.15));
    }

    /// Each bracket's base tax is the tax at the top of the one below, so the table has
    /// no jumps.
    #[test]
    fn tax_is_continuous_at_each_limit() {
        assert_eq!(tax(dec!(16667)), dec!(937.50));
        assert_eq!(tax(dec!(33333)), dec!(4270.70));
        assert_eq!(tax(dec!(83333)), dec!(16770.70));
        assert_eq!(tax(dec!(333333)), dec!(91770.70));
    }
}
