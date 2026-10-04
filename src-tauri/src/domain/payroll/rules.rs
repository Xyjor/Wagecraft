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
    /// SSS Monthly Salary Credit brackets, lowest first.
    pub sss_brackets: Vec<SssBracket>,
    pub sss_employee_rate: Decimal,
    pub sss_employer_rate: Decimal,
    pub philhealth: PhilHealthRule,
    pub pagibig: PagIbigRule,
    /// Withholding tax on semi-monthly compensation, lowest bracket first.
    pub tax_semi_monthly: Vec<TaxBracket>,
}

/// Monthly compensation from `from` up to the next bracket's `from` has this MSC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SssBracket {
    pub from: Decimal,
    pub msc: Decimal,
    /// Employees' Compensation, paid by the employer.
    pub ec: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhilHealthRule {
    pub rate: Decimal,
    pub floor: Decimal,
    pub ceiling: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagIbigRule {
    /// The employee rate drops to `low_rate` at or below this monthly pay.
    pub low_pay_limit: Decimal,
    pub low_rate: Decimal,
    pub employee_rate: Decimal,
    pub employer_rate: Decimal,
    pub max_base: Decimal,
}

/// Taxable income over `over` pays `base_tax` plus `rate` on the excess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxBracket {
    pub over: Decimal,
    pub base_tax: Decimal,
    pub rate: Decimal,
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
            sss_brackets: ph_2026_sss_brackets(),
            sss_employee_rate: dec!(0.05),
            sss_employer_rate: dec!(0.10),
            philhealth: PhilHealthRule {
                rate: dec!(0.05),
                floor: dec!(10000),
                ceiling: dec!(100000),
            },
            pagibig: PagIbigRule {
                low_pay_limit: dec!(1500),
                low_rate: dec!(0.01),
                employee_rate: dec!(0.02),
                employer_rate: dec!(0.02),
                max_base: dec!(10000),
            },
            tax_semi_monthly: [
                (dec!(0), dec!(0), dec!(0)),
                (dec!(10417), dec!(0), dec!(0.15)),
                (dec!(16667), dec!(937.50), dec!(0.20)),
                (dec!(33333), dec!(4270.70), dec!(0.25)),
                (dec!(83333), dec!(16770.70), dec!(0.30)),
                (dec!(333333), dec!(91770.70), dec!(0.35)),
            ]
            .into_iter()
            .map(|(over, base_tax, rate)| TaxBracket {
                over,
                base_tax,
                rate,
            })
            .collect(),
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

/// Plan Appendix A.2: MSC ₱5,000 to ₱35,000 in ₱500 steps. Each MSC covers compensation
/// from ₱250 below it (the lowest from zero); EC is ₱10 below an MSC of ₱15,000.
fn ph_2026_sss_brackets() -> Vec<SssBracket> {
    (0..=60)
        .map(|step| {
            let msc = Decimal::from(5000 + 500 * step);
            SssBracket {
                from: if step == 0 {
                    Decimal::ZERO
                } else {
                    msc - dec!(250)
                },
                msc,
                ec: if msc < dec!(15000) {
                    dec!(10)
                } else {
                    dec!(30)
                },
            }
        })
        .collect()
}
