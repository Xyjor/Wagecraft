//! Attendance time math (plan §6.3). Pure functions: no database, no clock, so every rule
//! here is pinned down by a unit test.
//!
//! Attendance times are company-local wall-clock times (`NaiveDateTime`, plan §5.1). A shift
//! whose end time is not after its start time ends the next day (22:00–06:00).
//!
//! Minutes are whole minutes, rounded down: 7 min 59 s late counts as 7 minutes late.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// Night shift differential window (Labor Code art. 86): 10:00 PM to 6:00 AM.
const NIGHT_START: NaiveTime = match NaiveTime::from_hms_opt(22, 0, 0) {
    Some(t) => t,
    None => panic!("22:00 is a valid time"),
};
const NIGHT_END: NaiveTime = match NaiveTime::from_hms_opt(6, 0, 0) {
    Some(t) => t,
    None => panic!("06:00 is a valid time"),
};

/// The parts of a work schedule the time math needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shift {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub break_minutes: i64,
    pub grace_minutes: i64,
}

impl Shift {
    /// True when the shift ends on the next calendar day (22:00–06:00).
    pub fn crosses_midnight(&self) -> bool {
        self.end <= self.start
    }

    /// When the shift starts and ends for the shift that begins on `work_date`.
    pub fn window(&self, work_date: NaiveDate) -> Interval {
        let start = work_date.and_time(self.start);
        let end_date = if self.crosses_midnight() {
            work_date + Duration::days(1)
        } else {
            work_date
        };
        Interval::new(start, end_date.and_time(self.end))
    }

    /// The unpaid break, assumed to sit in the middle of the shift (12:00–13:00 for an
    /// 08:00–17:00 shift with a 60-minute break). A half-day worker who leaves before it
    /// is not charged for it.
    pub fn break_window(&self, work_date: NaiveDate) -> Interval {
        let w = self.window(work_date);
        let brk = Duration::minutes(self.break_minutes.clamp(0, w.minutes()));
        let start = w.start + (w.end - w.start - brk) / 2;
        Interval::new(start, start + brk)
    }
}

/// A half-open span of local time, `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl Interval {
    /// An empty interval if `end` is before `start`.
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        Self {
            start,
            end: end.max(start),
        }
    }

    pub fn minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }

    fn overlap(&self, other: &Interval) -> Interval {
        Interval::new(self.start.max(other.start), self.end.min(other.end))
    }

    /// Whole minutes of `self` that fall inside `other`, rounded down.
    fn overlap_seconds(&self, other: &Interval) -> i64 {
        let o = self.overlap(other);
        (o.end - o.start).num_seconds()
    }
}

/// What the time math works out for one attendance record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Computed {
    pub late_minutes: i64,
    pub undertime_minutes: i64,
    pub worked_minutes: i64,
    pub night_minutes: i64,
}

/// Late, undertime, worked and night minutes for a scheduled work day.
///
/// - late: `max(0, time_in − (shift start + grace))`
/// - undertime: `max(0, shift end − time_out)`
/// - worked: time inside the shift, less the part that falls in the break
/// - night: the worked minutes between 10:00 PM and 6:00 AM
///
/// With no `time_out` yet (still clocked in) only lateness is known; the rest is 0.
/// Time outside the shift is not counted here: it is paid only through an approved
/// overtime request (plan §6.3).
pub fn compute(
    shift: &Shift,
    work_date: NaiveDate,
    time_in: NaiveDateTime,
    time_out: Option<NaiveDateTime>,
) -> Computed {
    let window = shift.window(work_date);
    let grace_end = window.start + Duration::minutes(shift.grace_minutes.max(0));
    let late_minutes = (time_in - grace_end).num_minutes().max(0);

    let Some(time_out) = time_out else {
        return Computed {
            late_minutes,
            ..Computed::default()
        };
    };
    let undertime_minutes = (window.end - time_out).num_minutes().max(0);

    let present = Interval::new(time_in, time_out).overlap(&window);
    let brk = shift.break_window(work_date);
    let worked: Vec<Interval> = minus(present, brk);

    let worked_seconds: i64 = worked.iter().map(|w| (w.end - w.start).num_seconds()).sum();
    let night_seconds: i64 = worked
        .iter()
        .flat_map(|w| {
            night_windows(w)
                .into_iter()
                .map(move |n| w.overlap_seconds(&n))
        })
        .sum();

    Computed {
        late_minutes,
        undertime_minutes,
        worked_minutes: worked_seconds / 60,
        night_minutes: night_seconds / 60,
    }
}

/// `a` with the part inside `b` cut out: zero, one or two pieces.
fn minus(a: Interval, b: Interval) -> Vec<Interval> {
    [
        Interval::new(a.start, b.start.min(a.end)),
        Interval::new(b.end.max(a.start), a.end),
    ]
    .into_iter()
    .filter(|i| i.end > i.start)
    .collect()
}

/// Every 22:00–06:00 window that could touch `span`.
fn night_windows(span: &Interval) -> Vec<Interval> {
    let first = span.start.date() - Duration::days(1);
    let last = span.end.date();
    first
        .iter_days()
        .take_while(|d| *d <= last)
        .map(|d| {
            Interval::new(
                d.and_time(NIGHT_START),
                (d + Duration::days(1)).and_time(NIGHT_END),
            )
        })
        .collect()
}

/// The work date a punch belongs to. For a shift that crosses midnight it is the day whose
/// shift starts closest to the punch: a night worker who clocks in at 00:30 is late for the
/// shift that started at 22:00 the day before, not early for tonight's. Any other shift,
/// or no schedule at all, uses the punch's own date.
pub fn work_date_for(shift: Option<&Shift>, punch: NaiveDateTime) -> NaiveDate {
    let today = punch.date();
    let Some(shift) = shift.filter(|s| s.crosses_midnight()) else {
        return today;
    };
    let yesterday = today - Duration::days(1);
    let gap = |d: NaiveDate| (punch - d.and_time(shift.start)).num_seconds().abs();
    if gap(yesterday) < gap(today) {
        yesterday
    } else {
        today
    }
}

/// Holiday types (plan §5.3). A special working day is an ordinary work day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolidayKind {
    Regular,
    SpecialNonWorking,
    SpecialWorking,
}

impl HolidayKind {
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "REGULAR" => Some(Self::Regular),
            "SPECIAL_NON_WORKING" => Some(Self::SpecialNonWorking),
            "SPECIAL_WORKING" => Some(Self::SpecialWorking),
            _ => None,
        }
    }
}

/// Attendance status for one day (plan §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayStatus {
    Present,
    Absent,
    RestDay,
    Holiday,
    OnLeave,
}

impl DayStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Present => "PRESENT",
            Self::Absent => "ABSENT",
            Self::RestDay => "REST_DAY",
            Self::Holiday => "HOLIDAY",
            Self::OnLeave => "ON_LEAVE",
        }
    }
}

/// The facts that decide a day's status.
#[derive(Debug, Clone, Copy, Default)]
pub struct DayFacts {
    /// The employee's schedule includes this weekday.
    pub scheduled: bool,
    pub holiday: Option<HolidayKind>,
    pub on_approved_leave: bool,
    pub clocked_in: bool,
}

/// Approved leave wins, then a punch, then a holiday, then the rest day. A scheduled
/// day with none of these is an absence. Working on a holiday or rest day still shows
/// PRESENT; the premium comes from the day type in payroll.
pub fn day_status(f: DayFacts) -> DayStatus {
    let day_off_holiday = matches!(
        f.holiday,
        Some(HolidayKind::Regular | HolidayKind::SpecialNonWorking)
    );
    if f.on_approved_leave {
        DayStatus::OnLeave
    } else if f.clocked_in {
        DayStatus::Present
    } else if day_off_holiday {
        DayStatus::Holiday
    } else if !f.scheduled {
        DayStatus::RestDay
    } else {
        DayStatus::Absent
    }
}

/// Parses a schedule's work days, `MON,TUE,WED,THU,FRI`. Unknown names are skipped.
pub fn parse_work_days(s: &str) -> Vec<Weekday> {
    s.split(',')
        .filter_map(|d| match d.trim().to_ascii_uppercase().as_str() {
            "MON" => Some(Weekday::Mon),
            "TUE" => Some(Weekday::Tue),
            "WED" => Some(Weekday::Wed),
            "THU" => Some(Weekday::Thu),
            "FRI" => Some(Weekday::Fri),
            "SAT" => Some(Weekday::Sat),
            "SUN" => Some(Weekday::Sun),
            _ => None,
        })
        .collect()
}

/// True if `work_days` (as stored) includes the weekday of `date`.
pub fn is_work_day(work_days: &str, date: NaiveDate) -> bool {
    parse_work_days(work_days).contains(&date.weekday())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
            .unwrap()
    }
    fn shift(start: &str, end: &str, brk: i64, grace: i64) -> Shift {
        Shift {
            start: t(start),
            end: t(end),
            break_minutes: brk,
            grace_minutes: grace,
        }
    }
    /// 08:00–17:00 with a 60-minute break and 10 minutes' grace.
    fn day() -> Shift {
        shift("08:00", "17:00", 60, 10)
    }
    /// 22:00–06:00 with a 60-minute break and no grace.
    fn night() -> Shift {
        shift("22:00", "06:00", 60, 0)
    }
    const MON: &str = "2026-10-05";

    fn run(s: &Shift, date: &str, time_in: &str, time_out: Option<&str>) -> Computed {
        compute(s, d(date), at(time_in), time_out.map(at))
    }

    #[test]
    fn on_time_full_day_has_no_late_or_undertime() {
        let c = run(&day(), MON, "2026-10-05 07:55", Some("2026-10-05 17:00"));
        assert_eq!(
            c,
            Computed {
                late_minutes: 0,
                undertime_minutes: 0,
                worked_minutes: 480,
                night_minutes: 0
            }
        );
    }

    #[test]
    fn arriving_inside_the_grace_period_is_not_late() {
        let c = run(&day(), MON, "2026-10-05 08:10", Some("2026-10-05 17:00"));
        assert_eq!(c.late_minutes, 0);
        // The 10 minutes are still not worked.
        assert_eq!(c.worked_minutes, 470);
    }

    #[test]
    fn late_is_counted_from_the_end_of_the_grace_period() {
        // Plan §6.3: max(0, time_in − (start + grace)).
        let c = run(&day(), MON, "2026-10-05 08:25", Some("2026-10-05 17:00"));
        assert_eq!(c.late_minutes, 15);
        assert_eq!(c.worked_minutes, 455);
    }

    #[test]
    fn seconds_are_rounded_down() {
        let c = run(&day(), MON, "2026-10-05 08:17:59", Some("2026-10-05 17:00"));
        assert_eq!(c.late_minutes, 7);
    }

    #[test]
    fn with_no_grace_one_minute_late_counts() {
        let s = shift("08:00", "17:00", 60, 0);
        let c = run(&s, MON, "2026-10-05 08:01", Some("2026-10-05 17:00"));
        assert_eq!(c.late_minutes, 1);
    }

    #[test]
    fn leaving_early_is_undertime() {
        let c = run(&day(), MON, "2026-10-05 08:00", Some("2026-10-05 16:30"));
        assert_eq!(c.undertime_minutes, 30);
        assert_eq!(c.worked_minutes, 450);
    }

    #[test]
    fn staying_late_adds_no_worked_minutes_without_overtime() {
        let c = run(&day(), MON, "2026-10-05 08:00", Some("2026-10-05 19:30"));
        assert_eq!(c.undertime_minutes, 0);
        assert_eq!(c.worked_minutes, 480);
    }

    #[test]
    fn a_morning_half_day_is_not_charged_the_break() {
        // Break sits at 12:00–13:00; leaving at 12:00 means 4 hours worked, 5 h undertime.
        let c = run(&day(), MON, "2026-10-05 08:00", Some("2026-10-05 12:00"));
        assert_eq!(day().break_window(d(MON)).start, at("2026-10-05 12:00"));
        assert_eq!(c.worked_minutes, 240);
        assert_eq!(c.undertime_minutes, 300);
    }

    #[test]
    fn an_afternoon_half_day_is_late_and_not_charged_the_break() {
        let c = run(&day(), MON, "2026-10-05 13:00", Some("2026-10-05 17:00"));
        assert_eq!(c.late_minutes, 290);
        assert_eq!(c.worked_minutes, 240);
    }

    #[test]
    fn leaving_during_the_break_counts_only_the_morning() {
        let c = run(&day(), MON, "2026-10-05 08:00", Some("2026-10-05 12:30"));
        assert_eq!(c.worked_minutes, 240);
    }

    #[test]
    fn still_clocked_in_shows_only_lateness() {
        let c = run(&day(), MON, "2026-10-05 08:30", None);
        assert_eq!(
            c,
            Computed {
                late_minutes: 20,
                ..Computed::default()
            }
        );
    }

    #[test]
    fn a_night_shift_crosses_midnight() {
        let n = night();
        assert!(n.crosses_midnight());
        let w = n.window(d(MON));
        assert_eq!(
            (w.start, w.end),
            (at("2026-10-05 22:00"), at("2026-10-06 06:00"))
        );
    }

    #[test]
    fn a_full_night_shift_is_all_night_minutes_less_the_break() {
        let c = run(&night(), MON, "2026-10-05 21:58", Some("2026-10-06 06:02"));
        assert_eq!(
            c,
            Computed {
                late_minutes: 0,
                undertime_minutes: 0,
                worked_minutes: 420,
                night_minutes: 420
            }
        );
    }

    #[test]
    fn late_and_undertime_work_across_midnight() {
        let c = run(&night(), MON, "2026-10-06 00:15", Some("2026-10-06 05:00"));
        assert_eq!(c.late_minutes, 135);
        assert_eq!(c.undertime_minutes, 60);
        // 00:15–05:00 minus the 01:30–02:30 break.
        assert_eq!(c.worked_minutes, 225);
        assert_eq!(c.night_minutes, 225);
    }

    #[test]
    fn an_evening_shift_earns_night_minutes_only_after_ten() {
        // 14:00–23:00, break 18:00–19:00: only 22:00–23:00 is night time.
        let s = shift("14:00", "23:00", 60, 0);
        let c = run(&s, MON, "2026-10-05 14:00", Some("2026-10-05 23:00"));
        assert_eq!(c.worked_minutes, 480);
        assert_eq!(c.night_minutes, 60);
    }

    #[test]
    fn an_early_shift_earns_night_minutes_before_six() {
        // 04:00–13:00, break 08:00–09:00: 04:00–06:00 is night time.
        let s = shift("04:00", "13:00", 60, 0);
        let c = run(&s, MON, "2026-10-05 04:00", Some("2026-10-05 13:00"));
        assert_eq!(c.night_minutes, 120);
    }

    #[test]
    fn a_day_shift_has_no_night_minutes() {
        let c = run(&day(), MON, "2026-10-05 08:00", Some("2026-10-05 17:00"));
        assert_eq!(c.night_minutes, 0);
    }

    #[test]
    fn a_shift_with_no_break_counts_every_minute() {
        let s = shift("09:00", "13:00", 0, 0);
        let c = run(&s, MON, "2026-10-05 09:00", Some("2026-10-05 13:00"));
        assert_eq!(c.worked_minutes, 240);
    }

    #[test]
    fn punches_belong_to_the_nearest_shift_start() {
        let n = night();
        // 21:50 Monday: Monday's shift.
        assert_eq!(
            work_date_for(Some(&n), at("2026-10-05 21:50")),
            d("2026-10-05")
        );
        // 00:30 Tuesday: still Monday's shift, just late.
        assert_eq!(
            work_date_for(Some(&n), at("2026-10-06 00:30")),
            d("2026-10-05")
        );
        // A day shift is always the same date.
        assert_eq!(
            work_date_for(Some(&day()), at("2026-10-05 07:40")),
            d("2026-10-05")
        );
        assert_eq!(work_date_for(None, at("2026-10-06 00:30")), d("2026-10-06"));
    }

    #[test]
    fn a_late_evening_punch_on_a_day_shift_keeps_its_own_date() {
        // 21:00 is closer to tomorrow's 08:00 start, but a day shift never moves the date.
        assert_eq!(
            work_date_for(Some(&day()), at("2026-10-05 21:00")),
            d("2026-10-05")
        );
        // A night worker leaving at 06:05 is still on Monday's shift.
        assert_eq!(
            work_date_for(Some(&night()), at("2026-10-06 06:05")),
            d("2026-10-05")
        );
    }

    #[test]
    fn day_status_follows_leave_punch_holiday_rest_day_order() {
        let base = DayFacts {
            scheduled: true,
            ..DayFacts::default()
        };
        assert_eq!(day_status(base), DayStatus::Absent);
        assert_eq!(
            day_status(DayFacts {
                clocked_in: true,
                ..base
            }),
            DayStatus::Present
        );
        assert_eq!(
            day_status(DayFacts {
                on_approved_leave: true,
                clocked_in: true,
                ..base
            }),
            DayStatus::OnLeave
        );
        assert_eq!(
            day_status(DayFacts {
                holiday: Some(HolidayKind::Regular),
                ..base
            }),
            DayStatus::Holiday
        );
        assert_eq!(
            day_status(DayFacts {
                holiday: Some(HolidayKind::Regular),
                clocked_in: true,
                ..base
            }),
            DayStatus::Present
        );
        // A special working day is an ordinary work day.
        assert_eq!(
            day_status(DayFacts {
                holiday: Some(HolidayKind::SpecialWorking),
                ..base
            }),
            DayStatus::Absent
        );
        assert_eq!(day_status(DayFacts::default()), DayStatus::RestDay);
    }

    #[test]
    fn work_days_parse_from_the_stored_list() {
        assert_eq!(
            parse_work_days("MON,TUE, wed,XYZ"),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed]
        );
        assert!(is_work_day("MON,TUE,WED,THU,FRI", d("2026-10-05")));
        assert!(!is_work_day("MON,TUE,WED,THU,FRI", d("2026-10-04")));
        assert_eq!(HolidayKind::from_db("REGULAR"), Some(HolidayKind::Regular));
        assert_eq!(HolidayKind::from_db("x"), None);
    }
}
