//! The whole pipeline (plan §7.2): earnings, then statutory contributions and withholding
//! tax, then loans and other deductions, down to net pay.

use super::contributions;
use super::engine::{compute_earnings, DeductionKind, Line, PayslipInput};
use super::rates::{PayBasis, Rates};
use super::rules::RulePack;
use super::tax;
use rust_decimal::Decimal;

/// What was left of `input.deductions[index]` after net pay reached its minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unpaid {
    pub index: usize,
    pub amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayslipResult {
    pub rates: Rates,
    /// Earnings, then the employee's contributions and tax as negative lines.
    pub lines: Vec<Line>,
    /// What the employer pays on top. Not part of net pay.
    pub employer: Vec<Line>,
    pub gross: Decimal,
    /// The employee's SSS, PhilHealth and Pag-IBIG for this cutoff.
    pub statutory_ee: Decimal,
    pub taxable: Decimal,
    pub withholding_tax: Decimal,
    /// Loans and other deductions actually taken.
    pub deductions: Decimal,
    pub net: Decimal,
    /// The part of each deduction that didn't fit, to carry over to the next period.
    pub unpaid: Vec<Unpaid>,
    /// Notes for HR on the review screen and the payslip.
    pub warnings: Vec<String>,
}

pub fn compute_payslip(input: &PayslipInput, rules: &RulePack) -> PayslipResult {
    let earnings = compute_earnings(input, rules);
    let mut lines = earnings.lines;
    let r = earnings.rates;

    let base = match input.basis {
        PayBasis::Monthly => Decimal::new(input.rate_cents, 2),
        PayBasis::Daily => {
            r.daily * Decimal::from(rules.factor(input.days_per_week)) / Decimal::from(12)
        }
    };
    let c = contributions::monthly(base, rules);
    let part = |amount| contributions::for_cutoff(amount, input.cutoff);
    let line = |code, label: &str, amount| Line {
        code,
        label: label.to_string(),
        quantity: Decimal::ONE,
        amount,
        taxable: false,
    };
    let employee = [
        line("SSS", "SSS", -part(c.sss.employee)),
        line("PHILHEALTH", "PhilHealth", -part(c.philhealth.employee)),
        line("PAGIBIG", "Pag-IBIG", -part(c.pagibig.employee)),
    ];
    let statutory_ee = -employee.iter().map(|l| l.amount).sum::<Decimal>();
    lines.extend(employee);
    let employer = vec![
        line("SSS_ER", "SSS (employer)", part(c.sss.employer)),
        line("SSS_EC", "Employees' Compensation", part(c.sss_ec)),
        line(
            "PHILHEALTH_ER",
            "PhilHealth (employer)",
            part(c.philhealth.employer),
        ),
        line(
            "PAGIBIG_ER",
            "Pag-IBIG (employer)",
            part(c.pagibig.employer),
        ),
    ];

    let taxable_earnings: Decimal = lines.iter().filter(|l| l.taxable).map(|l| l.amount).sum();
    let taxable = taxable_earnings - statutory_ee;
    let mut warnings = Vec::new();
    let withholding_tax = if input.minimum_wage_earner {
        warnings.push("Minimum wage earner: no tax withheld on this payslip.".to_string());
        Decimal::ZERO
    } else {
        tax::semi_monthly(taxable, rules)
    };
    if !withholding_tax.is_zero() {
        lines.push(line("TAX", "Withholding tax", -withholding_tax));
    }

    // Step 7 (plan §7.6): take each deduction in order while net pay stays at or above
    // the minimum; whatever doesn't fit is carried over to the next period.
    let minimum = Decimal::new(input.minimum_net_cents, 2);
    let mut net = earnings.gross - statutory_ee - withholding_tax;
    let mut unpaid = Vec::new();
    for (index, d) in input.deductions.iter().enumerate() {
        let amount = Decimal::new(d.amount_cents, 2);
        let taken = amount.min(net - minimum).max(Decimal::ZERO);
        if !taken.is_zero() {
            let code = match d.kind {
                DeductionKind::Loan => "LOAN",
                DeductionKind::Other => "DEDUCTION",
            };
            lines.push(line(code, &d.label, -taken));
            net -= taken;
        }
        if taken < amount {
            unpaid.push(Unpaid {
                index,
                amount: amount - taken,
            });
            warnings.push(format!(
                "{}: ₱{} of ₱{} carried over to the next period so net pay stays at ₱{}.",
                d.label,
                amount - taken,
                amount,
                minimum
            ));
        }
    }

    PayslipResult {
        rates: r,
        net,
        lines,
        employer,
        gross: earnings.gross,
        statutory_ee,
        taxable,
        withholding_tax,
        deductions: earnings.gross - statutory_ee - withholding_tax - net,
        unpaid,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::payroll::contributions::Cutoff;
    use crate::domain::payroll::engine::{Allowance, Deduction, DeductionKind, OvertimeBlock};
    use crate::domain::payroll::fixtures::{absent, day, second_cutoff};
    use crate::domain::payroll::rules::DayType;
    use rust_decimal_macros::dec;

    fn ph() -> RulePack {
        RulePack::ph_2026()
    }

    /// Plan §7.7, golden test #1.
    fn worked_example() -> PayslipInput {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        absent(&mut input, "2026-10-20");
        day(&mut input, "2026-10-21").late_minutes = 30;
        input.overtime.push(OvertimeBlock {
            day_type: DayType::Ordinary,
            minutes: 180,
            night_minutes: 0,
        });
        input
    }

    fn amounts(lines: &[Line]) -> Vec<(&str, Decimal)> {
        lines.iter().map(|l| (l.code, l.amount)).collect()
    }

    #[test]
    fn monthly_25k_second_cutoff_with_absence_late_and_ot() {
        let slip = compute_payslip(&worked_example(), &ph());
        assert_eq!(slip.gross, dec!(11817.52));
        assert_eq!(slip.statutory_ee, dec!(1037.50));
        assert_eq!(slip.taxable, dec!(10780.02));
        assert_eq!(slip.withholding_tax, dec!(54.45));
        assert_eq!(slip.net, dec!(10725.57));
        assert_eq!(
            amounts(&slip.lines)[4..],
            [
                ("SSS", dec!(-625.00)),
                ("PHILHEALTH", dec!(-312.50)),
                ("PAGIBIG", dec!(-100.00)),
                ("TAX", dec!(-54.45)),
            ]
        );
        assert_eq!(
            amounts(&slip.employer),
            [
                ("SSS_ER", dec!(1250.00)),
                ("SSS_EC", dec!(15.00)),
                ("PHILHEALTH_ER", dec!(312.50)),
                ("PAGIBIG_ER", dec!(100.00)),
            ]
        );
        let sum: Decimal = slip.lines.iter().map(|l| l.amount).sum();
        assert_eq!(sum, slip.net, "the payslip adds up line by line");
    }

    #[test]
    fn a_non_taxable_allowance_is_paid_but_not_taxed() {
        let mut input = worked_example();
        input.allowances.push(Allowance {
            label: "Rice".into(),
            amount_cents: 100_000,
            taxable: false,
        });
        let slip = compute_payslip(&input, &ph());
        assert_eq!(slip.gross, dec!(12817.52));
        assert_eq!(slip.taxable, dec!(10780.02));
        assert_eq!(slip.net, dec!(11725.57));
    }

    #[test]
    fn a_minimum_wage_earner_has_no_tax_withheld() {
        let mut input = worked_example();
        input.minimum_wage_earner = true;
        let slip = compute_payslip(&input, &ph());
        assert_eq!(slip.withholding_tax, dec!(0));
        assert!(!slip.lines.iter().any(|l| l.code == "TAX"));
        assert_eq!(slip.net, dec!(10780.02));
        assert_eq!(slip.warnings.len(), 1, "{:?}", slip.warnings);
    }

    /// ₱610 a day on a 5-day week is a monthly base of 610 × 261 ÷ 12 = 13,267.50:
    /// SSS MSC 13,500 (₱675 a month), PhilHealth 663.38 (employee 331.69), Pag-IBIG 200.
    #[test]
    fn a_daily_rate_contributes_on_its_monthly_equivalent() {
        let second = compute_payslip(&second_cutoff(PayBasis::Daily, 61_000), &ph());
        fn statutory(slip: &PayslipResult) -> Vec<(&str, Decimal)> {
            amounts(&slip.lines)
                .into_iter()
                .filter(|(c, _)| ["SSS", "PHILHEALTH", "PAGIBIG"].contains(c))
                .collect()
        }
        assert_eq!(
            statutory(&second),
            [
                ("SSS", dec!(-337.50)),
                ("PHILHEALTH", dec!(-165.85)),
                ("PAGIBIG", dec!(-100.00)),
            ]
        );
        let mut first_input = second_cutoff(PayBasis::Daily, 61_000);
        first_input.cutoff = Cutoff::First;
        let first = compute_payslip(&first_input, &ph());
        assert_eq!(statutory(&first)[1], ("PHILHEALTH", dec!(-165.84)));
    }

    fn deduction(kind: DeductionKind, label: &str, amount_cents: i64) -> Deduction {
        Deduction {
            kind,
            label: label.into(),
            amount_cents,
        }
    }

    /// The worked example nets 10,725.57 before deductions.
    #[test]
    fn a_loan_within_net_pay_is_taken_in_full() {
        let mut input = worked_example();
        input
            .deductions
            .push(deduction(DeductionKind::Loan, "SSS salary loan", 200_000));
        let slip = compute_payslip(&input, &ph());
        assert_eq!(
            slip.lines.last().map(|l| (l.code, l.amount)),
            Some(("LOAN", dec!(-2000.00)))
        );
        assert_eq!(slip.deductions, dec!(2000.00));
        assert_eq!(slip.net, dec!(8725.57));
        assert!(slip.unpaid.is_empty());
        assert!(slip.warnings.is_empty());
    }

    /// 10,725.57 − 6,000 leaves 4,725.57 for the 7,000 advance; 2,274.43 is carried over.
    /// The 500 after it gets nothing and is carried over whole.
    #[test]
    fn a_deduction_that_would_go_below_zero_is_cut_and_the_rest_carried_over() {
        let mut input = worked_example();
        input.deductions = vec![
            deduction(DeductionKind::Loan, "Company loan", 600_000),
            deduction(DeductionKind::Other, "Cash advance", 700_000),
            deduction(DeductionKind::Other, "Uniform", 50_000),
        ];
        let slip = compute_payslip(&input, &ph());
        assert_eq!(
            amounts(&slip.lines)[8..],
            [("LOAN", dec!(-6000.00)), ("DEDUCTION", dec!(-4725.57))]
        );
        assert_eq!(slip.net, dec!(0));
        assert_eq!(slip.deductions, dec!(10725.57));
        assert_eq!(
            slip.unpaid,
            [
                Unpaid {
                    index: 1,
                    amount: dec!(2274.43)
                },
                Unpaid {
                    index: 2,
                    amount: dec!(500.00)
                },
            ]
        );
        assert_eq!(slip.warnings.len(), 2, "{:?}", slip.warnings);
        let sum: Decimal = slip.lines.iter().map(|l| l.amount).sum();
        assert_eq!(sum, slip.net, "the payslip adds up line by line");
    }

    #[test]
    fn deductions_stop_at_the_configured_minimum_net_pay() {
        let mut input = worked_example();
        input.minimum_net_cents = 100_000;
        input.deductions = vec![deduction(DeductionKind::Other, "Cash advance", 1_000_000)];
        let slip = compute_payslip(&input, &ph());
        assert_eq!(slip.net, dec!(1000.00));
        assert_eq!(
            slip.unpaid,
            [Unpaid {
                index: 0,
                amount: dec!(274.43)
            }]
        );
    }

    /// Three absences on ₱12,000 a month: gross 4,344.83 less 550 in contributions,
    /// under the 4,000 minimum.
    #[test]
    fn nothing_is_deducted_when_net_pay_is_already_at_the_minimum() {
        let mut input = second_cutoff(PayBasis::Monthly, 1_200_000);
        input.minimum_net_cents = 400_000;
        absent(&mut input, "2026-10-20");
        absent(&mut input, "2026-10-21");
        absent(&mut input, "2026-10-22");
        input.deductions = vec![deduction(DeductionKind::Loan, "Company loan", 50_000)];
        let slip = compute_payslip(&input, &ph());
        assert_eq!(slip.net, dec!(3794.83));
        assert!(!slip.lines.iter().any(|l| l.code == "LOAN"));
        assert_eq!(
            slip.unpaid,
            [Unpaid {
                index: 0,
                amount: dec!(500.00)
            }]
        );
    }
}
