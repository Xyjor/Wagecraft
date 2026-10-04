//! Test inputs shared by the payroll engine's tests.

use super::contributions::Cutoff;
use super::engine::{DayRecord, DayStatus, PayslipInput};
use super::rates::PayBasis;
use super::rules::DayType;
use chrono::NaiveDate;

pub fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Oct 16–31, 2026 on a Mon–Fri schedule, every work day worked in full.
pub fn second_cutoff(basis: PayBasis, rate_cents: i64) -> PayslipInput {
    let days = (16..=31)
        .map(|day| {
            let date = d(&format!("2026-10-{day:02}"));
            let weekend = matches!(date.format("%a").to_string().as_str(), "Sat" | "Sun");
            DayRecord {
                date,
                day_type: if weekend {
                    DayType::RestDay
                } else {
                    DayType::Ordinary
                },
                scheduled: !weekend,
                status: if weekend {
                    DayStatus::Off
                } else {
                    DayStatus::Worked
                },
                late_minutes: 0,
                undertime_minutes: 0,
                worked_minutes: if weekend { 0 } else { 480 },
                night_minutes: 0,
            }
        })
        .collect();
    PayslipInput {
        basis,
        rate_cents,
        days_per_week: 5,
        days,
        status_before_period: Some(DayStatus::Worked),
        overtime: vec![],
        allowances: vec![],
        cutoff: Cutoff::Second,
        minimum_wage_earner: false,
    }
}

pub fn day<'a>(input: &'a mut PayslipInput, date: &str) -> &'a mut DayRecord {
    input
        .days
        .iter_mut()
        .find(|r| r.date == d(date))
        .expect("day in period")
}

pub fn absent(input: &mut PayslipInput, date: &str) {
    let r = day(input, date);
    r.status = DayStatus::Absent;
    r.worked_minutes = 0;
}

pub fn work_on(input: &mut PayslipInput, date: &str, day_type: DayType, minutes: i64) {
    let r = day(input, date);
    r.day_type = day_type;
    r.status = DayStatus::Worked;
    r.worked_minutes = minutes;
}
