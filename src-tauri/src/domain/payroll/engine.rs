//! Steps 1 to 4 of the pipeline (plan §7.2): rates, basic pay, premiums and allowances,
//! adding up to gross pay. Pure: the payroll service gathers one [`DayRecord`] per day.

use super::rates::{rates, round_line, PayBasis, Rates};
use super::rules::{DayType, RulePack};
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// What happened on one calendar day of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayRecord {
    pub date: NaiveDate,
    pub day_type: DayType,
    /// Whether the employee's schedule had them working this day (a holiday on a work day
    /// counts as scheduled).
    pub scheduled: bool,
    pub status: DayStatus,
    pub late_minutes: i64,
    pub undertime_minutes: i64,
    /// Minutes worked within the shift. Overtime comes only from approved requests.
    pub worked_minutes: i64,
    /// Of `worked_minutes`, those between 10 PM and 6 AM.
    pub night_minutes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayStatus {
    Worked,
    Absent,
    PaidLeave,
    UnpaidLeave,
    /// Not working and not expected to: a rest day, or a holiday off.
    Off,
}

/// Approved overtime, already split by day type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvertimeBlock {
    pub day_type: DayType,
    pub minutes: i64,
    /// Of `minutes`, those between 10 PM and 6 AM.
    pub night_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowance {
    pub label: String,
    pub amount_cents: i64,
    pub taxable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayslipInput {
    pub basis: PayBasis,
    pub rate_cents: i64,
    pub days_per_week: u8,
    /// Every calendar day of the period, in order.
    pub days: Vec<DayRecord>,
    /// The employee's status on their last scheduled work day before the period, for
    /// holiday pay on the first days of the period.
    pub status_before_period: Option<DayStatus>,
    pub overtime: Vec<OvertimeBlock>,
    pub allowances: Vec<Allowance>,
}

/// One payslip line. Deductions are negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub code: &'static str,
    pub label: String,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub taxable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Earnings {
    pub rates: Rates,
    pub lines: Vec<Line>,
    pub gross: Decimal,
}

/// Minutes of day work that earn day-type pay. Longer days are paid only through
/// approved overtime.
const DAY_MINUTES: i64 = 480;

pub fn compute_earnings(input: &PayslipInput, rules: &RulePack) -> Earnings {
    let r = rates(
        input.basis,
        input.rate_cents,
        rules.factor(input.days_per_week),
    );
    let mut lines = Lines::default();
    let minutes = |m: i64| Decimal::from(m);
    let hours = |m: i64| Decimal::from(m) / Decimal::from(60);

    match input.basis {
        PayBasis::Monthly => {
            let half = Decimal::new(input.rate_cents, 2) / Decimal::from(2);
            lines.add("BASIC", "Basic pay", Decimal::ONE, half);
            let absences = input
                .days
                .iter()
                .filter(|day| {
                    day.scheduled
                        && matches!(day.status, DayStatus::Absent | DayStatus::UnpaidLeave)
                })
                .count();
            let absences = Decimal::from(absences);
            lines.add("ABSENT", "Absences", absences, -absences * r.daily);
        }
        PayBasis::Daily => {
            let paid_days = input
                .days
                .iter()
                .filter(|day| match day.status {
                    DayStatus::Worked => day.day_type == DayType::Ordinary,
                    DayStatus::PaidLeave => day.scheduled,
                    _ => false,
                })
                .count();
            let paid_days = Decimal::from(paid_days);
            lines.add("BASIC", "Basic pay", paid_days, paid_days * r.daily);
        }
    }

    let late: i64 = input.days.iter().map(|day| day.late_minutes).sum();
    lines.add("LATE", "Late", minutes(late), -minutes(late) * r.per_minute);
    let undertime: i64 = input.days.iter().map(|day| day.undertime_minutes).sum();
    lines.add(
        "UNDERTIME",
        "Undertime",
        minutes(undertime),
        -minutes(undertime) * r.per_minute,
    );

    if input.basis == PayBasis::Daily {
        for (i, day) in input.days.iter().enumerate() {
            let unworked_holiday =
                matches!(day.day_type, DayType::Regular | DayType::DoubleRegular)
                    && day.status != DayStatus::Worked;
            if unworked_holiday && paid_day_before(input, i) {
                let share = rules.premium(day.day_type).work - Decimal::ONE;
                lines.add("HOLIDAY_PAY", "Holiday pay", Decimal::ONE, share * r.daily);
            }
        }
    }

    for day in input
        .days
        .iter()
        .filter(|day| day.status == DayStatus::Worked)
    {
        let premium = rules.premium(day.day_type);
        let worked = hours(day.worked_minutes.min(DAY_MINUTES));
        if day.day_type.is_rest_day() {
            lines.add(
                "REST_DAY",
                "Rest day work",
                worked,
                worked * r.hourly * premium.work,
            );
        } else if day.day_type != DayType::Ordinary {
            // A monthly salary already pays the holiday itself; only the premium is added.
            let share = match input.basis {
                PayBasis::Monthly => premium.work - Decimal::ONE,
                PayBasis::Daily => premium.work,
            };
            lines.add("HOLIDAY", "Holiday work", worked, worked * r.hourly * share);
        }
    }

    for ot in &input.overtime {
        let h = hours(ot.minutes);
        let rate = rules.premium(ot.day_type).overtime;
        lines.add("OT", "Overtime", h, h * r.hourly * rate);
    }

    for day in input
        .days
        .iter()
        .filter(|day| day.status == DayStatus::Worked)
    {
        let h = hours(day.night_minutes);
        let rate = rules.premium(day.day_type).work * rules.night_differential;
        lines.add("NIGHT_DIFF", "Night differential", h, h * r.hourly * rate);
    }
    for ot in &input.overtime {
        let h = hours(ot.night_minutes);
        let rate = rules.premium(ot.day_type).overtime * rules.night_differential;
        lines.add("NIGHT_DIFF", "Night differential", h, h * r.hourly * rate);
    }

    let mut lines = lines.finish();
    for a in &input.allowances {
        lines.push(Line {
            code: "ALLOWANCE",
            label: a.label.clone(),
            quantity: Decimal::ONE,
            amount: Decimal::new(a.amount_cents, 2),
            taxable: a.taxable,
        });
    }
    let gross = lines.iter().map(|l| l.amount).sum();
    Earnings {
        rates: r,
        lines,
        gross,
    }
}

/// Whether the employee was at work or on paid leave on the last scheduled work day
/// before `days[i]`, which earns a daily-paid employee the unworked regular holiday.
fn paid_day_before(input: &PayslipInput, i: usize) -> bool {
    let before = input.days[..i]
        .iter()
        .rev()
        .find(|day| day.scheduled && day.day_type == DayType::Ordinary)
        .map(|day| day.status)
        .or(input.status_before_period);
    matches!(before, Some(DayStatus::Worked | DayStatus::PaidLeave))
}

/// The order lines appear on a payslip.
const ORDER: [&str; 10] = [
    "BASIC",
    "ABSENT",
    "LATE",
    "UNDERTIME",
    "HOLIDAY_PAY",
    "HOLIDAY",
    "REST_DAY",
    "OT",
    "NIGHT_DIFF",
    "ALLOWANCE",
];

/// Collects amounts per line code at full precision, then rounds each line once.
#[derive(Default)]
struct Lines(Vec<(&'static str, &'static str, Decimal, Decimal)>);

impl Lines {
    fn add(&mut self, code: &'static str, label: &'static str, quantity: Decimal, amount: Decimal) {
        match self.0.iter_mut().find(|(c, ..)| *c == code) {
            Some((_, _, q, a)) => {
                *q += quantity;
                *a += amount;
            }
            None => self.0.push((code, label, quantity, amount)),
        }
    }

    fn finish(mut self) -> Vec<Line> {
        self.0
            .sort_by_key(|(code, ..)| ORDER.iter().position(|c| c == code));
        self.0
            .into_iter()
            .map(|(code, label, quantity, amount)| Line {
                code,
                label: label.to_string(),
                quantity,
                amount: round_line(amount),
                taxable: true,
            })
            .filter(|l| !l.amount.is_zero())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// Oct 16–31, 2026 on a Mon–Fri schedule, every work day worked in full.
    fn second_cutoff(basis: PayBasis, rate_cents: i64) -> PayslipInput {
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
        }
    }

    fn day<'a>(input: &'a mut PayslipInput, date: &str) -> &'a mut DayRecord {
        input
            .days
            .iter_mut()
            .find(|r| r.date == d(date))
            .expect("day in period")
    }

    fn absent(input: &mut PayslipInput, date: &str) {
        let r = day(input, date);
        r.status = DayStatus::Absent;
        r.worked_minutes = 0;
    }

    fn work_on(input: &mut PayslipInput, date: &str, day_type: DayType, minutes: i64) {
        let r = day(input, date);
        r.day_type = day_type;
        r.status = DayStatus::Worked;
        r.worked_minutes = minutes;
    }

    fn amounts(e: &Earnings) -> Vec<(&str, Decimal)> {
        e.lines.iter().map(|l| (l.code, l.amount)).collect()
    }

    fn ph() -> RulePack {
        RulePack::ph_2026()
    }

    /// Plan §7.7: ₱25,000 a month, one absence, 30 minutes late, 3 hours of overtime.
    #[test]
    fn the_worked_example_adds_up_to_its_gross() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        absent(&mut input, "2026-10-20");
        day(&mut input, "2026-10-21").late_minutes = 10;
        day(&mut input, "2026-10-22").late_minutes = 20;
        input.overtime.push(OvertimeBlock {
            day_type: DayType::Ordinary,
            minutes: 180,
            night_minutes: 0,
        });
        let e = compute_earnings(&input, &ph());
        assert_eq!(
            amounts(&e),
            [
                ("BASIC", dec!(12500.00)),
                ("ABSENT", dec!(-1149.43)),
                ("LATE", dec!(-71.84)),
                ("OT", dec!(538.79)),
            ]
        );
        assert_eq!(e.gross, dec!(11817.52));
        let late = e.lines.iter().find(|l| l.code == "LATE").unwrap();
        assert_eq!(late.quantity, dec!(30));
    }

    #[test]
    fn undertime_and_unpaid_leave_come_off_a_monthly_salary() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        day(&mut input, "2026-10-19").undertime_minutes = 60;
        day(&mut input, "2026-10-20").status = DayStatus::UnpaidLeave;
        day(&mut input, "2026-10-21").status = DayStatus::PaidLeave;
        let e = compute_earnings(&input, &ph());
        // 60 × 2.394636 = 143.68; one unpaid day = 1,149.43; paid leave costs nothing.
        assert_eq!(
            amounts(&e),
            [
                ("BASIC", dec!(12500.00)),
                ("ABSENT", dec!(-1149.43)),
                ("UNDERTIME", dec!(-143.68)),
            ]
        );
    }

    /// The 261 factor already pays holidays inside the salary, so holiday work adds only
    /// the premium. A rest day isn't in the salary, so its work is paid in full.
    #[test]
    fn a_monthly_salary_gets_holiday_premiums_and_full_rest_day_pay() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        work_on(&mut input, "2026-10-19", DayType::Regular, 480);
        work_on(&mut input, "2026-10-20", DayType::Special, 240);
        work_on(&mut input, "2026-10-17", DayType::RestDay, 480);
        let e = compute_earnings(&input, &ph());
        // hourly 143.678160…: regular 8 h × 1.00 = 1,149.43; special 4 h × 0.30 = 172.41;
        // rest day 8 h × 1.30 = 1,494.25.
        assert_eq!(
            amounts(&e),
            [
                ("BASIC", dec!(12500.00)),
                ("HOLIDAY", dec!(1321.84)),
                ("REST_DAY", dec!(1494.25)),
            ]
        );
        assert_eq!(e.gross, dec!(15316.09));
    }

    #[test]
    fn overtime_and_night_hours_are_priced_by_day_type() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        input.overtime = vec![
            OvertimeBlock {
                day_type: DayType::Ordinary,
                minutes: 60,
                night_minutes: 0,
            },
            OvertimeBlock {
                day_type: DayType::Regular,
                minutes: 120,
                night_minutes: 60,
            },
        ];
        day(&mut input, "2026-10-19").night_minutes = 120;
        let e = compute_earnings(&input, &ph());
        // OT: 1 h × 1.25 + 2 h × 2.60 = 6.45 h-rates × 143.678160 = 926.72.
        // Night: shift 2 h × 1.00 × 10% + OT 1 h × 2.60 × 10% = 0.46 × 143.678160 = 66.09.
        assert_eq!(
            amounts(&e),
            [
                ("BASIC", dec!(12500.00)),
                ("OT", dec!(926.72)),
                ("NIGHT_DIFF", dec!(66.09)),
            ]
        );
    }

    /// Only approved overtime is paid; hours past 8 in a day don't add holiday pay.
    #[test]
    fn day_pay_stops_at_eight_hours() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        work_on(&mut input, "2026-10-17", DayType::RestDay, 600);
        let e = compute_earnings(&input, &ph());
        assert_eq!(amounts(&e)[1], ("REST_DAY", dec!(1494.25)));
    }

    /// ₱610 a day. Oct 16–31, 2026 has 11 week days.
    #[test]
    fn a_daily_rate_pays_days_worked_and_paid_leave() {
        let mut input = second_cutoff(PayBasis::Daily, 61_000);
        absent(&mut input, "2026-10-19");
        day(&mut input, "2026-10-20").status = DayStatus::UnpaidLeave;
        day(&mut input, "2026-10-21").status = DayStatus::PaidLeave;
        day(&mut input, "2026-10-22").late_minutes = 15;
        work_on(&mut input, "2026-10-17", DayType::RestDay, 480);
        let e = compute_earnings(&input, &ph());
        // 8 worked + 1 paid leave = 9 × 610; late 15 × 1.270833 = 19.06;
        // rest day 8 × 76.25 × 1.30 = 793.00.
        assert_eq!(
            amounts(&e),
            [
                ("BASIC", dec!(5490.00)),
                ("LATE", dec!(-19.06)),
                ("REST_DAY", dec!(793.00)),
            ]
        );
        let basic = &e.lines[0];
        assert_eq!(basic.quantity, dec!(9));
    }

    #[test]
    fn a_daily_rate_pays_a_regular_holiday_worked_in_full() {
        let mut input = second_cutoff(PayBasis::Daily, 61_000);
        work_on(&mut input, "2026-10-19", DayType::Regular, 480);
        let e = compute_earnings(&input, &ph());
        // 10 ordinary days × 610 = 6,100; the holiday 8 × 76.25 × 2.00 = 1,220.
        assert_eq!(
            amounts(&e),
            [("BASIC", dec!(6100.00)), ("HOLIDAY", dec!(1220.00))]
        );
    }

    /// An unworked regular holiday is paid to a daily-paid employee who was at work, or on
    /// paid leave, on the work day before it.
    #[test]
    fn a_daily_rate_pays_an_unworked_regular_holiday_after_a_paid_day() {
        let holiday_off = |input: &mut PayslipInput, date: &str| {
            let r = day(input, date);
            r.day_type = DayType::Regular;
            r.status = DayStatus::Off;
            r.worked_minutes = 0;
        };
        let mut paid = second_cutoff(PayBasis::Daily, 61_000);
        holiday_off(&mut paid, "2026-10-21");
        day(&mut paid, "2026-10-20").status = DayStatus::PaidLeave;
        assert_eq!(
            amounts(&compute_earnings(&paid, &ph())),
            [("BASIC", dec!(6100.00)), ("HOLIDAY_PAY", dec!(610.00))]
        );

        let mut unpaid = second_cutoff(PayBasis::Daily, 61_000);
        holiday_off(&mut unpaid, "2026-10-21");
        absent(&mut unpaid, "2026-10-20");
        assert_eq!(
            amounts(&compute_earnings(&unpaid, &ph())),
            [("BASIC", dec!(5490.00))]
        );

        // The Monday after a weekend looks back to Friday; the first day looks back
        // before the period.
        let mut monday = second_cutoff(PayBasis::Daily, 61_000);
        holiday_off(&mut monday, "2026-10-19");
        absent(&mut monday, "2026-10-16");
        assert_eq!(
            amounts(&compute_earnings(&monday, &ph())),
            [("BASIC", dec!(5490.00))]
        );
        let mut first = second_cutoff(PayBasis::Daily, 61_000);
        holiday_off(&mut first, "2026-10-16");
        first.status_before_period = Some(DayStatus::Absent);
        assert_eq!(
            amounts(&compute_earnings(&first, &ph())),
            [("BASIC", dec!(6100.00))]
        );
    }

    #[test]
    fn allowances_add_to_gross_and_keep_their_tax_flag() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        input.allowances = vec![
            Allowance {
                label: "Rice".into(),
                amount_cents: 100_000,
                taxable: false,
            },
            Allowance {
                label: "Transport".into(),
                amount_cents: 50_050,
                taxable: true,
            },
        ];
        let e = compute_earnings(&input, &ph());
        let lines: Vec<_> = e
            .lines
            .iter()
            .map(|l| (l.code, l.label.as_str(), l.amount, l.taxable))
            .collect();
        assert_eq!(
            lines[1..],
            [
                ("ALLOWANCE", "Rice", dec!(1000.00), false),
                ("ALLOWANCE", "Transport", dec!(500.50), true),
            ]
        );
        assert_eq!(e.gross, dec!(14000.50));
    }

    /// A six-day schedule spreads the salary over 313 days, so a day off costs less.
    #[test]
    fn a_six_day_week_uses_its_own_factor() {
        let mut input = second_cutoff(PayBasis::Monthly, 2_500_000);
        input.days_per_week = 6;
        absent(&mut input, "2026-10-20");
        let e = compute_earnings(&input, &ph());
        assert_eq!(amounts(&e)[1], ("ABSENT", dec!(-958.47)));
    }
}
