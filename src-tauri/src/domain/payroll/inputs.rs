//! Turning what the app records (attendance, leave, holidays, schedules) into the engine's
//! [`DayRecord`]s (plan §6.5, step "Compute"). Pure: the payroll service looks the facts
//! up and calls these.

use super::engine::{DayRecord, DayStatus};
use super::rules::DayType;
use crate::domain::attendance_calc::HolidayKind;
use chrono::NaiveDate;

/// Half of an 8-hour day, in minutes.
const HALF_DAY_MINUTES: i64 = 240;

/// Everything known about one employee on one calendar day.
#[derive(Debug, Clone, Default)]
pub struct DayFacts {
    pub date: NaiveDate,
    /// Their schedule had them working this weekday.
    pub scheduled: bool,
    /// Every holiday on the date (two regular holidays make a double holiday).
    pub holidays: Vec<HolidayKind>,
    /// They were employed this day: on or after the hire date, on or before separation.
    pub employed: bool,
    pub record: Option<RecordFacts>,
    pub leave: Option<LeaveFacts>,
}

/// The attendance record, if there is one.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecordFacts {
    pub clocked_in: bool,
    pub late_minutes: i64,
    pub undertime_minutes: i64,
    pub worked_minutes: i64,
    pub night_minutes: i64,
}

/// Approved leave covering the day.
#[derive(Debug, Clone, Copy)]
pub struct LeaveFacts {
    pub paid: bool,
    pub half_day: bool,
}

/// The day type for premium pay. A special working day is an ordinary day.
pub fn day_type(holidays: &[HolidayKind], scheduled: bool) -> DayType {
    let regular = holidays
        .iter()
        .filter(|h| **h == HolidayKind::Regular)
        .count();
    let special = holidays.contains(&HolidayKind::SpecialNonWorking);
    match (regular, special, scheduled) {
        (2.., _, true) => DayType::DoubleRegular,
        (2.., _, false) => DayType::DoubleRegularRestDay,
        (1, _, true) => DayType::Regular,
        (1, _, false) => DayType::RegularRestDay,
        (0, true, true) => DayType::Special,
        (0, true, false) => DayType::SpecialRestDay,
        (0, false, true) => DayType::Ordinary,
        (0, false, false) => DayType::RestDay,
    }
}

/// One day for the engine. A punch makes it a worked day; otherwise a holiday off or a
/// rest day is off, approved leave is leave, and any other scheduled day is an absence.
/// Work days outside employment count as absences (owner's decision, 2026-10-05).
pub fn day_record(f: &DayFacts) -> DayRecord {
    let day_type = day_type(&f.holidays, f.scheduled);
    let day_off = matches!(
        day_type,
        DayType::Special | DayType::Regular | DayType::DoubleRegular
    ) || day_type.is_rest_day();
    let mut r = DayRecord {
        date: f.date,
        day_type,
        scheduled: f.scheduled,
        status: DayStatus::Off,
        late_minutes: 0,
        undertime_minutes: 0,
        worked_minutes: 0,
        night_minutes: 0,
    };
    let punch = f.record.filter(|rec| rec.clocked_in);

    if !f.employed {
        if f.scheduled {
            r.status = DayStatus::Absent;
        }
        return r;
    }
    if let Some(p) = punch {
        r.status = DayStatus::Worked;
        r.worked_minutes = p.worked_minutes;
        r.night_minutes = p.night_minutes;
        match f.leave {
            Some(LeaveFacts {
                half_day: true,
                paid: true,
            }) => {}
            Some(LeaveFacts {
                half_day: true,
                paid: false,
            }) => r.undertime_minutes = HALF_DAY_MINUTES,
            _ => {
                r.late_minutes = p.late_minutes;
                r.undertime_minutes = p.undertime_minutes;
            }
        }
        return r;
    }
    if day_off {
        return r;
    }
    r.status = match f.leave {
        Some(LeaveFacts {
            paid: true,
            half_day,
        }) => {
            if half_day {
                r.undertime_minutes = HALF_DAY_MINUTES;
            }
            DayStatus::PaidLeave
        }
        Some(LeaveFacts {
            paid: false,
            half_day: false,
        }) => DayStatus::UnpaidLeave,
        _ => DayStatus::Absent,
    };
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use HolidayKind::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn work_day() -> DayFacts {
        DayFacts {
            date: d("2026-10-19"),
            scheduled: true,
            employed: true,
            ..Default::default()
        }
    }

    fn punched(late: i64, under: i64, worked: i64, night: i64) -> Option<RecordFacts> {
        Some(RecordFacts {
            clocked_in: true,
            late_minutes: late,
            undertime_minutes: under,
            worked_minutes: worked,
            night_minutes: night,
        })
    }

    /// (status, late, undertime, worked, night)
    fn shape(r: &DayRecord) -> (DayStatus, i64, i64, i64, i64) {
        (
            r.status,
            r.late_minutes,
            r.undertime_minutes,
            r.worked_minutes,
            r.night_minutes,
        )
    }

    #[test]
    fn day_types_follow_the_holidays_and_the_schedule() {
        assert_eq!(day_type(&[], true), DayType::Ordinary);
        assert_eq!(day_type(&[], false), DayType::RestDay);
        assert_eq!(day_type(&[SpecialWorking], true), DayType::Ordinary);
        assert_eq!(day_type(&[SpecialWorking], false), DayType::RestDay);
        assert_eq!(day_type(&[SpecialNonWorking], true), DayType::Special);
        assert_eq!(
            day_type(&[SpecialNonWorking], false),
            DayType::SpecialRestDay
        );
        assert_eq!(day_type(&[Regular], true), DayType::Regular);
        assert_eq!(day_type(&[Regular], false), DayType::RegularRestDay);
        assert_eq!(
            day_type(&[SpecialNonWorking, Regular], true),
            DayType::Regular,
            "a regular holiday outranks a special one"
        );
        assert_eq!(day_type(&[Regular, Regular], true), DayType::DoubleRegular);
        assert_eq!(
            day_type(&[Regular, Regular], false),
            DayType::DoubleRegularRestDay
        );
    }

    #[test]
    fn a_punch_is_a_worked_day_with_its_minutes() {
        let f = DayFacts {
            record: punched(10, 5, 455, 0),
            ..work_day()
        };
        let r = day_record(&f);
        assert_eq!(r.date, d("2026-10-19"));
        assert_eq!(r.day_type, DayType::Ordinary);
        assert!(r.scheduled);
        assert_eq!(shape(&r), (DayStatus::Worked, 10, 5, 455, 0));
    }

    #[test]
    fn work_on_a_rest_day_or_holiday_is_worked() {
        let rest = DayFacts {
            scheduled: false,
            record: punched(0, 0, 480, 60),
            ..work_day()
        };
        let r = day_record(&rest);
        assert_eq!(r.day_type, DayType::RestDay);
        assert_eq!(shape(&r), (DayStatus::Worked, 0, 0, 480, 60));

        let holiday = DayFacts {
            holidays: vec![Regular],
            record: punched(0, 0, 480, 0),
            ..work_day()
        };
        assert_eq!(day_record(&holiday).status, DayStatus::Worked);
        assert_eq!(day_record(&holiday).day_type, DayType::Regular);
    }

    #[test]
    fn a_scheduled_day_with_no_punch_is_an_absence() {
        assert_eq!(day_record(&work_day()).status, DayStatus::Absent);
        let manual_absent = DayFacts {
            record: Some(RecordFacts::default()),
            ..work_day()
        };
        assert_eq!(day_record(&manual_absent).status, DayStatus::Absent);
    }

    #[test]
    fn rest_days_and_holidays_off_are_off() {
        let rest = DayFacts {
            scheduled: false,
            ..work_day()
        };
        assert_eq!(day_record(&rest).status, DayStatus::Off);
        for h in [Regular, SpecialNonWorking] {
            let holiday = DayFacts {
                holidays: vec![h],
                ..work_day()
            };
            assert_eq!(day_record(&holiday).status, DayStatus::Off, "{h:?}");
        }
        let special_working = DayFacts {
            holidays: vec![SpecialWorking],
            ..work_day()
        };
        assert_eq!(
            day_record(&special_working).status,
            DayStatus::Absent,
            "a special working day is an ordinary work day"
        );
    }

    #[test]
    fn whole_day_leave_is_paid_or_unpaid_leave() {
        for (paid, status) in [
            (true, DayStatus::PaidLeave),
            (false, DayStatus::UnpaidLeave),
        ] {
            let f = DayFacts {
                leave: Some(LeaveFacts {
                    paid,
                    half_day: false,
                }),
                ..work_day()
            };
            assert_eq!(shape(&day_record(&f)), (status, 0, 0, 0, 0));
        }
    }

    #[test]
    fn a_holiday_off_wins_over_leave() {
        let f = DayFacts {
            holidays: vec![Regular],
            leave: Some(LeaveFacts {
                paid: true,
                half_day: false,
            }),
            ..work_day()
        };
        assert_eq!(day_record(&f).status, DayStatus::Off);
    }

    #[test]
    fn a_half_day_of_paid_leave_excuses_the_other_half() {
        let leave = Some(LeaveFacts {
            paid: true,
            half_day: true,
        });
        // Worked the afternoon: the morning shows as late.
        let worked = DayFacts {
            record: punched(300, 0, 240, 0),
            leave,
            ..work_day()
        };
        assert_eq!(
            shape(&day_record(&worked)),
            (DayStatus::Worked, 0, 0, 240, 0)
        );
        // Never came in: half paid, half missing.
        let missing = DayFacts {
            leave,
            ..work_day()
        };
        assert_eq!(
            shape(&day_record(&missing)),
            (DayStatus::PaidLeave, 0, HALF_DAY_MINUTES, 0, 0)
        );
    }

    #[test]
    fn a_half_day_of_unpaid_leave_costs_half_a_day() {
        let leave = Some(LeaveFacts {
            paid: false,
            half_day: true,
        });
        let worked = DayFacts {
            record: punched(0, 300, 240, 0),
            leave,
            ..work_day()
        };
        assert_eq!(
            shape(&day_record(&worked)),
            (DayStatus::Worked, 0, HALF_DAY_MINUTES, 240, 0)
        );
        let missing = DayFacts {
            leave,
            ..work_day()
        };
        assert_eq!(day_record(&missing).status, DayStatus::Absent);
    }

    #[test]
    fn work_days_outside_employment_are_absences() {
        // Decision 2026-10-05: hired or separated mid-period, the missing work days are
        // deducted like absences.
        let before_hire = DayFacts {
            employed: false,
            record: punched(0, 0, 480, 0),
            ..work_day()
        };
        assert_eq!(
            shape(&day_record(&before_hire)),
            (DayStatus::Absent, 0, 0, 0, 0)
        );
        let holiday = DayFacts {
            employed: false,
            holidays: vec![Regular],
            ..work_day()
        };
        assert_eq!(day_record(&holiday).status, DayStatus::Absent);
        let rest = DayFacts {
            employed: false,
            scheduled: false,
            ..work_day()
        };
        assert_eq!(day_record(&rest).status, DayStatus::Off);
    }
}
