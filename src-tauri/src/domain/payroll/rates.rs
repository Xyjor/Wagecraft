//! Step 1 of the pipeline (plan §7.3): daily, hourly and per-minute rates.
//!
//! Rates stay at full precision. Only a payslip line is rounded, with [`round_line`], so
//! 30 minutes late costs 30 × 2.394636… and not 30 × ₱2.39.

use rust_decimal::{Decimal, RoundingStrategy};

/// How the employee's rate is stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayBasis {
    Monthly,
    Daily,
}

/// An employee's rates at full precision, in pesos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rates {
    pub daily: Decimal,
    pub hourly: Decimal,
    pub per_minute: Decimal,
}

/// Works out the rates from the employee's pay rate in centavos. `factor` is the rule
/// pack's paid days per year (261 for a 5-day week, 313 for a 6-day week); it is used
/// only for monthly-paid employees.
pub fn rates(basis: PayBasis, rate_cents: i64, factor: u32) -> Rates {
    let rate = Decimal::new(rate_cents, 2);
    let daily = match basis {
        PayBasis::Monthly => rate * Decimal::from(12) / Decimal::from(factor),
        PayBasis::Daily => rate,
    };
    let hourly = daily / Decimal::from(HOURS_PER_DAY);
    Rates {
        daily,
        hourly,
        per_minute: hourly / Decimal::from(60),
    }
}

/// Plan §7.3: hourly is daily ÷ 8.
const HOURS_PER_DAY: u32 = 8;

/// Rounds a payslip line to the centavo, with half a centavo going away from zero.
pub fn round_line(amount: Decimal) -> Decimal {
    amount.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// The worked example in plan §7.7: ₱25,000 a month on a 5-day week.
    #[test]
    fn monthly_rates_match_the_worked_example() {
        let r = rates(PayBasis::Monthly, 2_500_000, 261);
        assert_eq!(round_line(r.daily), dec!(1149.43));
        assert_eq!(round_line(r.hourly), dec!(143.68));
        assert_eq!(round_line(r.per_minute), dec!(2.39));
    }

    /// Lines use the full-precision rate. Rounding the rate first would give
    /// 30 × 2.39 = 71.70 and 3 × 143.68 × 1.25 = 538.80.
    #[test]
    fn lines_are_priced_from_unrounded_rates() {
        let r = rates(PayBasis::Monthly, 2_500_000, 261);
        assert_eq!(round_line(r.daily), dec!(1149.43));
        assert_eq!(round_line(r.per_minute * dec!(30)), dec!(71.84));
        assert_eq!(round_line(r.hourly * dec!(3) * dec!(1.25)), dec!(538.79));
    }

    #[test]
    fn a_six_day_week_spreads_the_salary_over_more_days() {
        let r = rates(PayBasis::Monthly, 2_500_000, 313);
        // 25,000 × 12 ÷ 313 = 958.466…
        assert_eq!(round_line(r.daily), dec!(958.47));
    }

    #[test]
    fn a_daily_rate_is_used_as_given_whatever_the_factor() {
        let r = rates(PayBasis::Daily, 61_000, 261);
        assert_eq!(r.daily, dec!(610));
        assert_eq!(r.hourly, dec!(76.25));
        // 76.25 ÷ 60 = 1.270833…
        assert_eq!(round_line(r.per_minute), dec!(1.27));
        assert_eq!(rates(PayBasis::Daily, 61_000, 313), r);
    }

    #[test]
    fn half_a_centavo_rounds_away_from_zero() {
        assert_eq!(round_line(dec!(0.005)), dec!(0.01));
        assert_eq!(round_line(dec!(-0.005)), dec!(-0.01));
        assert_eq!(round_line(dec!(1.0049)), dec!(1.00));
        assert_eq!(round_line(dec!(2.675)), dec!(2.68));
    }
}
