//! Step 5 of the pipeline (plan §7.4): SSS, PhilHealth and Pag-IBIG. Each is worked out
//! on a monthly base, then split across the two semi-monthly cutoffs (ADR-006).

use super::rates::round_line;
use super::rules::RulePack;
use rust_decimal::Decimal;

/// Which half of the month a semi-monthly payslip covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cutoff {
    First,
    Second,
}

/// One agency's monthly employee and employer shares, in pesos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monthly {
    pub employee: Decimal,
    pub employer: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contributions {
    pub sss: Monthly,
    /// The SSS Monthly Salary Credit the shares were taken from.
    pub sss_msc: Decimal,
    /// Employees' Compensation, paid by the employer on top of its SSS share.
    pub sss_ec: Decimal,
    pub philhealth: Monthly,
    pub pagibig: Monthly,
}

/// The monthly shares for a monthly contribution base in pesos (plan §7.4: the monthly
/// basic rate, or daily × factor ÷ 12 for daily-paid employees).
pub fn monthly(base: Decimal, rules: &RulePack) -> Contributions {
    let bracket = rules
        .sss_brackets
        .iter()
        .rev()
        .find(|b| base >= b.from)
        .or(rules.sss_brackets.first())
        .copied()
        .expect("the rule pack has SSS brackets");
    let ph = rules.philhealth;
    let premium = round_line(base.clamp(ph.floor, ph.ceiling) * ph.rate);
    let ph_employee = (premium / Decimal::from(2)).trunc_with_scale(2);
    let pi = rules.pagibig;
    let pi_base = base.min(pi.max_base);
    let pi_rate = if base <= pi.low_pay_limit {
        pi.low_rate
    } else {
        pi.employee_rate
    };
    Contributions {
        sss: Monthly {
            employee: round_line(bracket.msc * rules.sss_employee_rate),
            employer: round_line(bracket.msc * rules.sss_employer_rate),
        },
        sss_msc: bracket.msc,
        sss_ec: bracket.ec,
        philhealth: Monthly {
            employee: ph_employee,
            employer: premium - ph_employee,
        },
        pagibig: Monthly {
            employee: round_line(pi_base * pi_rate),
            employer: round_line(pi_base * pi.employer_rate),
        },
    }
}

/// This cutoff's part of a monthly amount: the first cutoff takes half rounded down to
/// the centavo, the second takes the rest, so the two always add up to the month.
pub fn for_cutoff(amount: Decimal, cutoff: Cutoff) -> Decimal {
    let first = (amount / Decimal::from(2)).trunc_with_scale(2);
    match cutoff {
        Cutoff::First => first,
        Cutoff::Second => amount - first,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ph() -> RulePack {
        RulePack::ph_2026()
    }

    #[test]
    fn the_worked_example_shares() {
        let c = monthly(dec!(25000), &ph());
        assert_eq!(c.sss_msc, dec!(25000));
        assert_eq!(
            c.sss,
            Monthly {
                employee: dec!(1250.00),
                employer: dec!(2500.00)
            }
        );
        assert_eq!(c.sss_ec, dec!(30));
        assert_eq!(
            c.philhealth,
            Monthly {
                employee: dec!(625.00),
                employer: dec!(625.00)
            }
        );
        assert_eq!(
            c.pagibig,
            Monthly {
                employee: dec!(200.00),
                employer: dec!(200.00)
            }
        );
    }

    /// Each MSC covers ₱250 below to just under ₱250 above it, from ₱5,000 to ₱35,000.
    #[test]
    fn sss_picks_the_salary_credit_bracket() {
        let msc = |base| monthly(base, &ph()).sss_msc;
        assert_eq!(msc(dec!(1000)), dec!(5000));
        assert_eq!(msc(dec!(5249.99)), dec!(5000));
        assert_eq!(msc(dec!(5250)), dec!(5500));
        assert_eq!(msc(dec!(25249.99)), dec!(25000));
        assert_eq!(msc(dec!(25250)), dec!(25500));
        assert_eq!(msc(dec!(34749.99)), dec!(34500));
        assert_eq!(msc(dec!(34750)), dec!(35000));
        assert_eq!(msc(dec!(90000)), dec!(35000));
    }

    #[test]
    fn sss_employees_compensation_steps_up_at_15000() {
        let c = monthly(dec!(14749.99), &ph());
        assert_eq!((c.sss_msc, c.sss_ec), (dec!(14500), dec!(10)));
        assert_eq!(
            c.sss,
            Monthly {
                employee: dec!(725.00),
                employer: dec!(1450.00)
            }
        );
        assert_eq!(monthly(dec!(14750), &ph()).sss_ec, dec!(30));
    }

    #[test]
    fn philhealth_is_clamped_between_its_floor_and_ceiling() {
        let ph_share = |base| monthly(base, &ph()).philhealth;
        assert_eq!(
            ph_share(dec!(6000)),
            Monthly {
                employee: dec!(250.00),
                employer: dec!(250.00)
            }
        );
        assert_eq!(
            ph_share(dec!(150000)),
            Monthly {
                employee: dec!(2500.00),
                employer: dec!(2500.00)
            }
        );
    }

    /// 5% of ₱18,333.33 is ₱916.6665, so ₱916.67; the employer takes the odd centavo.
    #[test]
    fn philhealth_gives_the_odd_centavo_to_the_employer() {
        assert_eq!(
            monthly(dec!(18333.33), &ph()).philhealth,
            Monthly {
                employee: dec!(458.33),
                employer: dec!(458.34)
            }
        );
    }

    #[test]
    fn pagibig_is_one_percent_up_to_1500_and_capped_at_10000() {
        let pi = |base| monthly(base, &ph()).pagibig;
        assert_eq!(
            pi(dec!(1500)),
            Monthly {
                employee: dec!(15.00),
                employer: dec!(30.00)
            }
        );
        assert_eq!(
            pi(dec!(1500.01)),
            Monthly {
                employee: dec!(30.00),
                employer: dec!(30.00)
            }
        );
        assert_eq!(
            pi(dec!(8000)),
            Monthly {
                employee: dec!(160.00),
                employer: dec!(160.00)
            }
        );
        assert_eq!(
            pi(dec!(50000)),
            Monthly {
                employee: dec!(200.00),
                employer: dec!(200.00)
            }
        );
    }

    #[test]
    fn the_second_cutoff_takes_the_odd_centavo() {
        assert_eq!(for_cutoff(dec!(625.00), Cutoff::First), dec!(312.50));
        assert_eq!(for_cutoff(dec!(625.00), Cutoff::Second), dec!(312.50));
        assert_eq!(for_cutoff(dec!(458.33), Cutoff::First), dec!(229.16));
        assert_eq!(for_cutoff(dec!(458.33), Cutoff::Second), dec!(229.17));
    }
}
