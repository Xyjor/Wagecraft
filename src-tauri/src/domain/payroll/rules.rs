//! The rule pack (plan §7.1, ADR-005): the dated numbers the engine applies. In v1 the
//! values come from a migration; tests build one with [`RulePack::ph_2026`].

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// What kind of day a day of work was, for premium pay (plan §7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayType {
    Ordinary,
    RestDay,
    Special,
    SpecialRestDay,
    Regular,
    RegularRestDay,
    DoubleRegular,
    DoubleRegularRestDay,
}

impl DayType {
    pub fn is_rest_day(self) -> bool {
        matches!(
            self,
            Self::RestDay
                | Self::SpecialRestDay
                | Self::RegularRestDay
                | Self::DoubleRegularRestDay
        )
    }

    pub fn is_regular_holiday(self) -> bool {
        matches!(
            self,
            Self::Regular | Self::RegularRestDay | Self::DoubleRegular | Self::DoubleRegularRestDay
        )
    }
}

/// Pay for a day type as a multiple of the hourly rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PremiumRate {
    /// The first 8 hours of work.
    pub work: Decimal,
    /// Each approved overtime hour.
    pub overtime: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulePack {
    /// Paid days per year for a 5-day work week.
    pub factor_five_day: u32,
    /// Paid days per year for a 6-day work week.
    pub factor_six_day: u32,
    /// Extra share of the applicable hourly rate for each night hour (10 PM to 6 AM).
    pub night_differential: Decimal,
    premiums: [(DayType, PremiumRate); 8],
}

impl RulePack {
    /// The PH-2026 values in plan Appendix A.
    pub fn ph_2026() -> Self {
        let p = |work, overtime| PremiumRate { work, overtime };
        Self {
            factor_five_day: 261,
            factor_six_day: 313,
            night_differential: dec!(0.10),
            premiums: [
                (DayType::Ordinary, p(dec!(1.00), dec!(1.25))),
                (DayType::RestDay, p(dec!(1.30), dec!(1.69))),
                (DayType::Special, p(dec!(1.30), dec!(1.69))),
                (DayType::SpecialRestDay, p(dec!(1.50), dec!(1.95))),
                (DayType::Regular, p(dec!(2.00), dec!(2.60))),
                (DayType::RegularRestDay, p(dec!(2.60), dec!(3.38))),
                (DayType::DoubleRegular, p(dec!(3.00), dec!(3.90))),
                (DayType::DoubleRegularRestDay, p(dec!(3.90), dec!(5.07))),
            ],
        }
    }

    pub fn premium(&self, day: DayType) -> PremiumRate {
        self.premiums
            .iter()
            .find(|(d, _)| *d == day)
            .map(|(_, rate)| *rate)
            .expect("every day type has a premium rate")
    }

    /// Paid days per year for someone who works `days_per_week` days.
    pub fn factor(&self, days_per_week: u8) -> u32 {
        if days_per_week >= 6 {
            self.factor_six_day
        } else {
            self.factor_five_day
        }
    }
}
