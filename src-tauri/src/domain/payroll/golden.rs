//! Runs the golden payroll cases in `tests/payroll_cases/` (plan §7.8). Each JSON file is
//! one employee's cutoff with expected amounts worked out by hand, so a case can be added
//! without writing Rust. A case is never edited to make a failing run pass (plan §11.2).

use super::contributions::Cutoff;
use super::engine::{Allowance, DayRecord, DayStatus, OvertimeBlock, PayslipInput};
use super::fixtures::d;
use super::payslip::{compute_payslip, PayslipResult};
use super::rates::PayBasis;
use super::rules::{DayType, RulePack};
use chrono::{Datelike, Duration, Weekday};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    name: String,
    #[allow(dead_code)]
    about: String,
    employee: Employee,
    period: Period,
    /// Changes to the default day: by default each work day of the week is an ordinary
    /// day worked in full and every other day is a rest day off.
    #[serde(default)]
    days: BTreeMap<String, DayChange>,
    /// A change applied to every default work day before `days`.
    #[serde(default)]
    every_work_day: Option<DayChange>,
    #[serde(default)]
    status_before_period: Option<Status>,
    #[serde(default)]
    overtime: Vec<Overtime>,
    #[serde(default)]
    allowances: Vec<CaseAllowance>,
    expected: Expected,
    #[allow(dead_code)]
    workings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Employee {
    basis: Basis,
    rate: String,
    days_per_week: u8,
    minimum_wage_earner: bool,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum Basis {
    Monthly,
    Daily,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Period {
    from: String,
    to: String,
    cutoff: u8,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DayChange {
    day_type: Option<Kind>,
    status: Option<Status>,
    late: Option<i64>,
    undertime: Option<i64>,
    worked: Option<i64>,
    night: Option<i64>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum Kind {
    Ordinary,
    RestDay,
    Special,
    SpecialRestDay,
    Regular,
    RegularRestDay,
    DoubleRegular,
    DoubleRegularRestDay,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum Status {
    Worked,
    Absent,
    PaidLeave,
    UnpaidLeave,
    Off,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Overtime {
    day_type: Kind,
    minutes: i64,
    #[serde(default)]
    night: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseAllowance {
    label: String,
    amount: String,
    taxable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Expected {
    lines: Vec<(String, String)>,
    employer: Vec<(String, String)>,
    gross: String,
    statutory_ee: String,
    taxable: String,
    withholding_tax: String,
    net: String,
    warnings: usize,
}

impl From<Kind> for DayType {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Ordinary => DayType::Ordinary,
            Kind::RestDay => DayType::RestDay,
            Kind::Special => DayType::Special,
            Kind::SpecialRestDay => DayType::SpecialRestDay,
            Kind::Regular => DayType::Regular,
            Kind::RegularRestDay => DayType::RegularRestDay,
            Kind::DoubleRegular => DayType::DoubleRegular,
            Kind::DoubleRegularRestDay => DayType::DoubleRegularRestDay,
        }
    }
}

impl From<Status> for DayStatus {
    fn from(s: Status) -> Self {
        match s {
            Status::Worked => DayStatus::Worked,
            Status::Absent => DayStatus::Absent,
            Status::PaidLeave => DayStatus::PaidLeave,
            Status::UnpaidLeave => DayStatus::UnpaidLeave,
            Status::Off => DayStatus::Off,
        }
    }
}

fn pesos(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|_| panic!("{s:?} is not an amount"))
}

fn cents(s: &str) -> i64 {
    let amount = pesos(s) * Decimal::from(100);
    assert!(amount.fract().is_zero(), "{s:?} has fractions of a centavo");
    i64::try_from(amount).expect("amount fits in centavos")
}

fn apply(day: &mut DayRecord, change: &DayChange) {
    if let Some(kind) = change.day_type {
        day.day_type = kind.into();
    }
    if let Some(status) = change.status {
        day.status = status.into();
        day.worked_minutes = if day.status == DayStatus::Worked {
            480
        } else {
            0
        };
    }
    day.late_minutes = change.late.unwrap_or(day.late_minutes);
    day.undertime_minutes = change.undertime.unwrap_or(day.undertime_minutes);
    day.worked_minutes = change.worked.unwrap_or(day.worked_minutes);
    day.night_minutes = change.night.unwrap_or(day.night_minutes);
}

impl Case {
    fn input(&self) -> PayslipInput {
        let e = &self.employee;
        assert!(
            matches!(e.days_per_week, 5 | 6),
            "daysPerWeek is 5 (Mon–Fri) or 6 (Mon–Sat)"
        );
        let (from, to) = (d(&self.period.from), d(&self.period.to));
        let changes: BTreeMap<_, _> = self.days.iter().map(|(k, v)| (d(k), v)).collect();
        let mut days = Vec::new();
        let mut date = from;
        while date <= to {
            let work_day = match date.weekday() {
                Weekday::Sun => false,
                Weekday::Sat => e.days_per_week == 6,
                _ => true,
            };
            let mut day = DayRecord {
                date,
                day_type: if work_day {
                    DayType::Ordinary
                } else {
                    DayType::RestDay
                },
                scheduled: work_day,
                status: if work_day {
                    DayStatus::Worked
                } else {
                    DayStatus::Off
                },
                late_minutes: 0,
                undertime_minutes: 0,
                worked_minutes: if work_day { 480 } else { 0 },
                night_minutes: 0,
            };
            if let (true, Some(change)) = (work_day, &self.every_work_day) {
                apply(&mut day, change);
            }
            if let Some(change) = changes.get(&date) {
                apply(&mut day, change);
            }
            days.push(day);
            date += Duration::days(1);
        }
        for date in changes.keys() {
            assert!((from..=to).contains(date), "{date} is outside the period");
        }
        PayslipInput {
            basis: match e.basis {
                Basis::Monthly => PayBasis::Monthly,
                Basis::Daily => PayBasis::Daily,
            },
            rate_cents: cents(&e.rate),
            days_per_week: e.days_per_week,
            days,
            status_before_period: Some(
                self.status_before_period
                    .map_or(DayStatus::Worked, Into::into),
            ),
            overtime: self
                .overtime
                .iter()
                .map(|o| OvertimeBlock {
                    day_type: o.day_type.into(),
                    minutes: o.minutes,
                    night_minutes: o.night,
                })
                .collect(),
            allowances: self
                .allowances
                .iter()
                .map(|a| Allowance {
                    label: a.label.clone(),
                    amount_cents: cents(&a.amount),
                    taxable: a.taxable,
                })
                .collect(),
            cutoff: match self.period.cutoff {
                1 => Cutoff::First,
                2 => Cutoff::Second,
                n => panic!("cutoff {n} is not 1 or 2"),
            },
            minimum_wage_earner: e.minimum_wage_earner,
        }
    }

    /// Every way the payslip differs from what was worked out by hand.
    fn differences(&self, slip: &PayslipResult) -> Vec<String> {
        let x = &self.expected;
        let mut out = Vec::new();
        let mut check = |what: &str, want: &str, got: Decimal| {
            if pesos(want) != got {
                out.push(format!("{what}: expected {want}, got {got}"));
            }
        };
        check("gross", &x.gross, slip.gross);
        check("statutoryEe", &x.statutory_ee, slip.statutory_ee);
        check("taxable", &x.taxable, slip.taxable);
        check("withholdingTax", &x.withholding_tax, slip.withholding_tax);
        check("net", &x.net, slip.net);
        let lines = |lines: &[(String, String)]| -> Vec<(String, Decimal)> {
            lines.iter().map(|(c, a)| (c.clone(), pesos(a))).collect()
        };
        let got = |lines: &[super::engine::Line]| -> Vec<(String, Decimal)> {
            lines
                .iter()
                .map(|l| (l.code.to_string(), l.amount))
                .collect()
        };
        if lines(&x.lines) != got(&slip.lines) {
            out.push(format!(
                "lines: expected {:?}, got {:?}",
                lines(&x.lines),
                got(&slip.lines)
            ));
        }
        if lines(&x.employer) != got(&slip.employer) {
            out.push(format!(
                "employer: expected {:?}, got {:?}",
                lines(&x.employer),
                got(&slip.employer)
            ));
        }
        if x.warnings != slip.warnings.len() {
            out.push(format!(
                "warnings: expected {}, got {:?}",
                x.warnings, slip.warnings
            ));
        }
        out
    }
}

fn case_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/payroll_cases");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_golden_case_matches_its_hand_worked_payslip() {
    let rules = RulePack::ph_2026();
    let files = case_files();
    let mut failures = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("readable case");
        let case: Case =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let slip = compute_payslip(&case.input(), &rules);
        let differences = case.differences(&slip);
        if !differences.is_empty() {
            let name = file.file_name().unwrap().to_string_lossy();
            failures.push(format!(
                "{name} ({})\n  {}",
                case.name,
                differences.join("\n  ")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} golden cases failed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

/// The plan's release gate is 20 cases (§2); the 20th comes with loans and carryover.
#[test]
fn there_are_at_least_19_golden_cases() {
    assert!(case_files().len() >= 19, "found {}", case_files().len());
}
