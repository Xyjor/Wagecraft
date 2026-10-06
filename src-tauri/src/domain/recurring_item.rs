//! Recurring allowances, loans and other deductions on an employee (plan §5.3), and which
//! of them a pay period takes (plan §7.3 step 4, §7.6).

use super::payroll::engine::{Allowance, Deduction, DeductionKind};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

pub const KINDS: [&str; 3] = ["ALLOWANCE", "LOAN", "DEDUCTION"];
pub const SCHEDULES: [&str; 3] = ["EVERY_CUTOFF", "FIRST_CUTOFF", "SECOND_CUTOFF"];

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecurringItem {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    /// ALLOWANCE, LOAN or DEDUCTION.
    pub kind: String,
    pub label: String,
    /// Per cutoff it applies to.
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub taxable: bool,
    /// EVERY_CUTOFF, FIRST_CUTOFF or SECOND_CUTOFF.
    pub schedule: String,
    pub start_date: String,
    pub end_date: Option<String>,
    /// What is still owed, for loans only.
    #[ts(type = "number | null")]
    pub remaining_balance_cents: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecurringItemInput {
    pub kind: String,
    pub label: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub taxable: bool,
    pub schedule: String,
    pub start_date: String,
    pub end_date: Option<String>,
    #[ts(type = "number | null")]
    pub remaining_balance_cents: Option<i64>,
}

/// An unpaid deduction from an earlier payslip, waiting to be taken.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct OpenCarryover {
    pub id: i64,
    pub recurring_item_id: Option<i64>,
    /// LOAN or DEDUCTION.
    pub code: String,
    pub label: String,
    pub amount_cents: i64,
}

/// Where a deduction on a payslip came from, so its unpaid part can be carried over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeductionSource {
    pub recurring_item_id: Option<i64>,
    /// The carryover this deduction takes, if it is one.
    pub carryover_id: Option<i64>,
    /// The label without the "(carried over)" note.
    pub label: String,
}

/// What a period takes from the recurring items and open carryovers. `deductions` and
/// `sources` line up one to one, in the order the engine takes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scheduled {
    pub allowances: Vec<Allowance>,
    pub deductions: Vec<Deduction>,
    pub sources: Vec<DeductionSource>,
}

/// The items in effect at any time from `start` to `end` and scheduled for `cutoff_no`
/// (1 or 2). Deductions come in the order plan §7.6 takes them: open carryovers first,
/// then loans oldest first, then other deductions oldest first. A loan never takes more
/// than what is still owed, and a paid-off loan takes nothing.
pub fn for_cutoff(
    items: &[RecurringItem],
    carryovers: &[OpenCarryover],
    start: NaiveDate,
    end: NaiveDate,
    cutoff_no: i64,
) -> Scheduled {
    let schedule = if cutoff_no == 1 {
        "FIRST_CUTOFF"
    } else {
        "SECOND_CUTOFF"
    };
    let (start, end) = (start.to_string(), end.to_string());
    let mut due: Vec<&RecurringItem> = items
        .iter()
        .filter(|i| i.schedule == "EVERY_CUTOFF" || i.schedule == schedule)
        .filter(|i| i.start_date <= end && i.end_date.as_ref().is_none_or(|e| *e >= start))
        .collect();
    due.sort_by(|a, b| (&a.start_date, a.id).cmp(&(&b.start_date, b.id)));

    let mut out = Scheduled {
        allowances: due
            .iter()
            .filter(|i| i.kind == "ALLOWANCE")
            .map(|i| Allowance {
                label: i.label.clone(),
                amount_cents: i.amount_cents,
                taxable: i.taxable,
            })
            .collect(),
        ..Default::default()
    };
    for c in carryovers {
        out.push(
            kind_of(&c.code),
            format!("{} (carried over)", c.label),
            c.amount_cents,
            DeductionSource {
                recurring_item_id: c.recurring_item_id,
                carryover_id: Some(c.id),
                label: c.label.clone(),
            },
        );
    }
    for kind in ["LOAN", "DEDUCTION"] {
        for i in due.iter().filter(|i| i.kind == kind) {
            let amount = match i.remaining_balance_cents {
                Some(owed) => i.amount_cents.min(owed),
                None => i.amount_cents,
            };
            if amount <= 0 {
                continue;
            }
            out.push(
                kind_of(kind),
                i.label.clone(),
                amount,
                DeductionSource {
                    recurring_item_id: Some(i.id),
                    carryover_id: None,
                    label: i.label.clone(),
                },
            );
        }
    }
    out
}

fn kind_of(code: &str) -> DeductionKind {
    if code == "LOAN" {
        DeductionKind::Loan
    } else {
        DeductionKind::Other
    }
}

impl Scheduled {
    fn push(
        &mut self,
        kind: DeductionKind,
        label: String,
        amount_cents: i64,
        source: DeductionSource,
    ) {
        self.deductions.push(Deduction {
            kind,
            label,
            amount_cents,
        });
        self.sources.push(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn item(id: i64, kind: &str, label: &str, amount_cents: i64) -> RecurringItem {
        RecurringItem {
            id,
            employee_id: 1,
            kind: kind.into(),
            label: label.into(),
            amount_cents,
            taxable: false,
            schedule: "EVERY_CUTOFF".into(),
            start_date: "2026-01-01".into(),
            end_date: None,
            remaining_balance_cents: (kind == "LOAN").then_some(10_000_000),
        }
    }

    /// Oct 16–31, 2026: the second cutoff.
    fn oct_second(items: &[RecurringItem], carryovers: &[OpenCarryover]) -> Scheduled {
        for_cutoff(items, carryovers, d("2026-10-16"), d("2026-10-31"), 2)
    }

    fn labels(s: &Scheduled) -> Vec<(DeductionKind, &str, i64)> {
        s.deductions
            .iter()
            .map(|x| (x.kind, x.label.as_str(), x.amount_cents))
            .collect()
    }

    #[test]
    fn allowances_keep_their_amount_and_tax_flag() {
        let mut rice = item(1, "ALLOWANCE", "Rice subsidy", 100_000);
        let mut transport = item(2, "ALLOWANCE", "Transport", 50_000);
        transport.taxable = true;
        rice.taxable = false;
        let s = oct_second(&[rice, transport], &[]);
        assert_eq!(
            s.allowances,
            vec![
                Allowance {
                    label: "Rice subsidy".into(),
                    amount_cents: 100_000,
                    taxable: false
                },
                Allowance {
                    label: "Transport".into(),
                    amount_cents: 50_000,
                    taxable: true
                },
            ]
        );
        assert!(s.deductions.is_empty() && s.sources.is_empty());
    }

    #[test]
    fn items_follow_their_cutoff_schedule() {
        let mut first = item(1, "ALLOWANCE", "First only", 100);
        first.schedule = "FIRST_CUTOFF".into();
        let mut second = item(2, "ALLOWANCE", "Second only", 200);
        second.schedule = "SECOND_CUTOFF".into();
        let every = item(3, "ALLOWANCE", "Every", 300);
        let items = [first, second, every];

        let names =
            |s: Scheduled| -> Vec<String> { s.allowances.into_iter().map(|a| a.label).collect() };
        assert_eq!(
            names(for_cutoff(&items, &[], d("2026-10-01"), d("2026-10-15"), 1)),
            ["First only", "Every"]
        );
        assert_eq!(names(oct_second(&items, &[])), ["Second only", "Every"]);
    }

    #[test]
    fn items_count_when_in_effect_on_any_day_of_the_period() {
        let mut ends_on_first_day = item(1, "ALLOWANCE", "Ends Oct 16", 100);
        ends_on_first_day.end_date = Some("2026-10-16".into());
        let mut ended_before = item(2, "ALLOWANCE", "Ended Oct 15", 100);
        ended_before.end_date = Some("2026-10-15".into());
        let mut starts_on_last_day = item(3, "ALLOWANCE", "Starts Oct 31", 100);
        starts_on_last_day.start_date = "2026-10-31".into();
        let mut starts_after = item(4, "ALLOWANCE", "Starts Nov 1", 100);
        starts_after.start_date = "2026-11-01".into();

        let s = oct_second(
            &[
                ends_on_first_day,
                ended_before,
                starts_on_last_day,
                starts_after,
            ],
            &[],
        );
        let names: Vec<_> = s.allowances.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(names, ["Ends Oct 16", "Starts Oct 31"]);
    }

    #[test]
    fn carryovers_then_loans_oldest_first_then_other_deductions() {
        let mut new_loan = item(1, "LOAN", "Company loan", 200_000);
        new_loan.start_date = "2026-09-01".into();
        let mut old_loan = item(2, "LOAN", "SSS salary loan", 150_000);
        old_loan.start_date = "2026-03-01".into();
        let uniform = item(3, "DEDUCTION", "Uniform", 50_000);
        let allowance = item(4, "ALLOWANCE", "Rice subsidy", 100_000);
        let carried = OpenCarryover {
            id: 7,
            recurring_item_id: Some(3),
            code: "DEDUCTION".into(),
            label: "Cash advance".into(),
            amount_cents: 30_000,
        };

        let s = oct_second(&[uniform, new_loan, allowance, old_loan], &[carried]);
        assert_eq!(
            labels(&s),
            [
                (DeductionKind::Other, "Cash advance (carried over)", 30_000),
                (DeductionKind::Loan, "SSS salary loan", 150_000),
                (DeductionKind::Loan, "Company loan", 200_000),
                (DeductionKind::Other, "Uniform", 50_000),
            ]
        );
        assert_eq!(
            s.sources,
            [
                DeductionSource {
                    recurring_item_id: Some(3),
                    carryover_id: Some(7),
                    label: "Cash advance".into()
                },
                DeductionSource {
                    recurring_item_id: Some(2),
                    carryover_id: None,
                    label: "SSS salary loan".into()
                },
                DeductionSource {
                    recurring_item_id: Some(1),
                    carryover_id: None,
                    label: "Company loan".into()
                },
                DeductionSource {
                    recurring_item_id: Some(3),
                    carryover_id: None,
                    label: "Uniform".into()
                },
            ]
        );
    }

    #[test]
    fn a_carried_over_loan_is_still_a_loan() {
        let carried = OpenCarryover {
            id: 1,
            recurring_item_id: None,
            code: "LOAN".into(),
            label: "Company loan".into(),
            amount_cents: 5_000,
        };
        assert_eq!(
            labels(&oct_second(&[], &[carried])),
            [(DeductionKind::Loan, "Company loan (carried over)", 5_000)]
        );
    }

    #[test]
    fn a_loan_takes_no_more_than_what_is_owed() {
        let mut almost_paid = item(1, "LOAN", "Company loan", 200_000);
        almost_paid.remaining_balance_cents = Some(75_000);
        let mut paid_off = item(2, "LOAN", "Old loan", 200_000);
        paid_off.remaining_balance_cents = Some(0);
        assert_eq!(
            labels(&oct_second(&[almost_paid, paid_off], &[])),
            [(DeductionKind::Loan, "Company loan", 75_000)]
        );
    }

    #[test]
    fn same_start_date_keeps_the_order_items_were_added() {
        let b = item(2, "DEDUCTION", "Second added", 100);
        let a = item(1, "DEDUCTION", "First added", 100);
        let names: Vec<_> = oct_second(&[b, a], &[])
            .deductions
            .into_iter()
            .map(|x| x.label)
            .collect();
        assert_eq!(names, ["First added", "Second added"]);
    }
}
