//! Attendance for HR and for each employee (plan §6.3): the day grid, the review queue,
//! corrections with a required reason, and one employee's records over a range.

use crate::audit::{self, Actor, Entry};
use crate::domain::attendance::{AttendanceInput, AttendanceRecord, DateRange, DayRow, ReviewItem};
use crate::domain::attendance_calc::{self, DayFacts, HolidayKind, Shift};
use crate::error::{AppError, FieldError};
use crate::repositories::attendance::{self as repo, Manual};
use crate::repositories::leave_requests as leave_repo;
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;
use std::collections::HashMap;

const LOCKED: &str = "This day is in a posted payroll period, so it can't be changed.";
const ON_LEAVE: &str = "This employee is on approved leave this day. Cancel the leave first if \
    they worked.";

/// Longest range one request may ask for: two months and change.
const MAX_RANGE_DAYS: i64 = 62;

/// One employee's records in the range, newest first.
pub async fn for_employee(
    db: &SqlitePool,
    employee_id: i64,
    range: &DateRange,
) -> Result<Vec<AttendanceRecord>, AppError> {
    let (from, to) = check_range(range)?;
    Ok(repo::for_employee(db, employee_id, &from.to_string(), &to.to_string()).await?)
}

pub(crate) fn check_range(range: &DateRange) -> Result<(NaiveDate, NaiveDate), AppError> {
    let from = NaiveDate::parse_from_str(range.from.trim(), "%Y-%m-%d");
    let to = NaiveDate::parse_from_str(range.to.trim(), "%Y-%m-%d");
    let (Ok(from), Ok(to)) = (from, to) else {
        return Err(AppError::Validation(vec![field(
            "from",
            "Pick a valid date range",
        )]));
    };
    if to < from {
        return Err(AppError::Validation(vec![field(
            "to",
            "The end date is before the start date",
        )]));
    }
    if (to - from).num_days() > MAX_RANGE_DAYS {
        return Err(AppError::Validation(vec![field(
            "to",
            "Pick at most two months at a time",
        )]));
    }
    Ok((from, to))
}

/// Every current employee's day on `date`: their record, or what the day works out to
/// without one (absent, rest day or holiday).
pub async fn day(db: &SqlitePool, date: &str) -> Result<Vec<DayRow>, AppError> {
    let date = parse_date(date, "date")?;
    let iso = date.to_string();
    let employees = repo::grid_employees(db, &iso).await?;
    let mut records: HashMap<i64, AttendanceRecord> = repo::on_date_all(db, &iso)
        .await?
        .into_iter()
        .map(|r| (r.employee_id, r))
        .collect();
    let mut leave: HashMap<i64, (String, bool)> = leave_repo::approved_on(db, &iso)
        .await?
        .into_iter()
        .map(|l| (l.employee_id, (l.leave_type_name, l.half_day)))
        .collect();
    let holidays = repo::holidays_on(db, &iso).await?;
    let holiday = strongest_holiday(&holidays);
    let holiday_name = (!holidays.is_empty()).then(|| {
        holidays
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" and ")
    });

    Ok(employees
        .into_iter()
        .map(|e| {
            let record = records.remove(&e.id);
            let on_leave = leave.remove(&e.id);
            let status = match &record {
                Some(r) => Some(r.status.clone()),
                None => e.work_days.as_deref().map(|days| {
                    let facts = DayFacts {
                        scheduled: attendance_calc::is_work_day(days, date),
                        holiday,
                        on_approved_leave: on_leave.is_some(),
                        clocked_in: false,
                    };
                    attendance_calc::day_status(facts).as_db().to_string()
                }),
            };
            DayRow {
                employee_id: e.id,
                employee_no: e.employee_no,
                employee_name: e.name,
                department_name: e.department_name,
                status,
                holiday: holiday_name.clone(),
                full_day_leave: on_leave.as_ref().is_some_and(|(_, half)| !half),
                leave: on_leave.map(|(name, half)| {
                    if half {
                        format!("{name} (half day)")
                    } else {
                        name
                    }
                }),
                record,
            }
        })
        .collect())
}

/// A regular holiday outranks a special one when two fall on the same day.
pub(crate) fn strongest_holiday(holidays: &[(String, String)]) -> Option<HolidayKind> {
    let kinds: Vec<_> = holidays
        .iter()
        .filter_map(|(_, t)| HolidayKind::from_db(t))
        .collect();
    [
        HolidayKind::Regular,
        HolidayKind::SpecialNonWorking,
        HolidayKind::SpecialWorking,
    ]
    .into_iter()
    .find(|k| kinds.contains(k))
}

/// A time in with no time out after this long is a forgotten clock-out (the kiosk won't
/// close it either).
const FORGOTTEN_OUT_HOURS: i64 = 24;

/// Records flagged for HR, oldest first, including forgotten clock-outs as of `now` (the
/// office clock). Those have `needs_review` false: only a correction settles them.
pub async fn review_queue(
    db: &SqlitePool,
    now: NaiveDateTime,
) -> Result<Vec<ReviewItem>, AppError> {
    let stale_before = time::to_local_db(now - Duration::hours(FORGOTTEN_OUT_HOURS));
    Ok(repo::flagged(db, &stale_before)
        .await?
        .into_iter()
        .map(|(employee_no, employee_name, record)| ReviewItem {
            employee_no,
            employee_name,
            record,
        })
        .collect())
}

/// Adds or corrects an employee's day. The reason is required, and the change is audited
/// with its before and after values. The record becomes MANUAL and its review flag clears.
pub async fn save(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: AttendanceInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<AttendanceRecord, AppError> {
    let mut errors = Vec::new();
    let work_date = NaiveDate::parse_from_str(input.work_date.trim(), "%Y-%m-%d").ok();
    match work_date {
        None => errors.push(field("workDate", "Pick a date")),
        Some(d) if d > today => errors.push(field("workDate", "The date can't be in the future")),
        Some(_) => {}
    }
    let time_in = parse_time(&input.time_in, "timeIn", &mut errors);
    let time_out = match input.time_out.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(t) => parse_time(t, "timeOut", &mut errors),
    };
    let reason = input.reason.trim();
    if !(3..=200).contains(&reason.chars().count()) {
        errors.push(field("reason", "Say why, in 3 to 200 characters"));
    }
    let (Some(work_date), Some(t_in)) = (work_date, time_in) else {
        return Err(AppError::Validation(errors));
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let mut tx = db.begin().await?;
    let employee = repo::employee_shift(&mut tx, input.employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "This employee is archived. Unarchive them to change their attendance.",
        ));
    }
    let date = work_date.to_string();
    let before = repo::on_date(&mut tx, input.employee_id, &date).await?;
    if before.as_ref().is_some_and(|r| r.locked) {
        return Err(AppError::Conflict(LOCKED));
    }
    if leave_repo::full_day_leave_on(&mut tx, input.employee_id, &date, false).await? {
        return Err(AppError::Conflict(ON_LEAVE));
    }

    let shift = shift_of(&employee);
    let time_in = place_time_in(shift.as_ref(), work_date, t_in);
    let time_out = time_out.map(|t| {
        let out = time_in.date().and_time(t);
        if out <= time_in {
            out + Duration::days(1)
        } else {
            out
        }
    });
    let computed = shift
        .map(|s| attendance_calc::compute(&s, work_date, time_in, time_out))
        .unwrap_or_default();
    let in_text = time::to_local_db(time_in);
    let out_text = time_out.map(time::to_local_db);
    let id = repo::save_manual(
        &mut tx,
        &Manual {
            employee_id: input.employee_id,
            work_date: &date,
            time_in: &in_text,
            time_out: out_text.as_deref(),
            computed: &computed,
            reason,
            edited_by: actor.user_id,
        },
    )
    .await?;
    let after = repo::by_id(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound("Attendance record"))?;
    let entry = Entry {
        action: if before.is_some() {
            "attendance.correct"
        } else {
            "attendance.add"
        },
        entity: Some(("attendance_record", id)),
        before: before.as_ref().map(snapshot),
        after: Some({
            let mut v = snapshot(&after);
            v["employeeNo"] = json!(employee.employee_no);
            v["reason"] = json!(reason);
            v
        }),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// Clears a record's review flag after HR has looked and found nothing to change.
pub async fn mark_reviewed(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    now: DateTime<Utc>,
) -> Result<AttendanceRecord, AppError> {
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound("Attendance record"))?;
    if !before.needs_review {
        return Ok(before);
    }
    repo::clear_review(&mut tx, id).await?;
    let entry = Entry {
        action: "attendance.review",
        entity: Some(("attendance_record", id)),
        before: Some(json!({ "needsReview": true, "reviewNote": before.review_note })),
        after: Some(json!({ "needsReview": false })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let after = repo::by_id(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound("Attendance record"))?;
    tx.commit().await?;
    Ok(after)
}

/// A time typed for a night shift may fall after midnight: 00:15 on Monday's 22:00 shift
/// is Tuesday 00:15. Uses the same rule as the kiosk.
fn place_time_in(shift: Option<&Shift>, work_date: NaiveDate, t: NaiveTime) -> NaiveDateTime {
    let same_day = work_date.and_time(t);
    let next_day = (work_date + Duration::days(1)).and_time(t);
    if attendance_calc::work_date_for(shift, next_day) == work_date
        && attendance_calc::work_date_for(shift, same_day) != work_date
    {
        next_day
    } else {
        same_day
    }
}

fn shift_of(e: &repo::EmployeeShift) -> Option<Shift> {
    let t = |s: &Option<String>| {
        s.as_deref()
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
    };
    Some(Shift {
        start: t(&e.start_time)?,
        end: t(&e.end_time)?,
        break_minutes: e.break_minutes?,
        grace_minutes: e.grace_minutes?,
    })
}

fn snapshot(r: &AttendanceRecord) -> serde_json::Value {
    json!({
        "workDate": r.work_date,
        "timeIn": r.time_in,
        "timeOut": r.time_out,
        "status": r.status,
        "lateMinutes": r.late_minutes,
        "undertimeMinutes": r.undertime_minutes,
        "workedMinutes": r.worked_minutes,
        "nightMinutes": r.night_minutes,
        "source": r.source,
        "needsReview": r.needs_review,
    })
}

fn parse_time(s: &str, name: &str, errors: &mut Vec<FieldError>) -> Option<NaiveTime> {
    let t = NaiveTime::parse_from_str(s.trim(), "%H:%M").ok();
    if t.is_none() {
        errors.push(field(name, "Enter a time like 08:00"));
    }
    t
}

fn parse_date(s: &str, name: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation(vec![field(name, "Pick a date")]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use chrono::TimeZone;

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 9, 1, 0, 0).unwrap()
    }

    fn wall(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").expect("time")
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 9).unwrap()
    }

    /// EMP-1 Juan on 08:00–17:00 Mon–Fri (10 min grace), EMP-2 Ana on 22:00–06:00
    /// Mon–Fri, EMP-3 Leo with no schedule, EMP-4 hired next year, EMP-5 archived,
    /// EMP-6 who left in September.
    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        sqlx::query(
            "INSERT INTO departments (id, code, name) VALUES (1, 'OPS', 'Operations'); \
             INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days) VALUES \
             (1, 'Office', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI'), \
             (2, 'Night', '22:00', '06:00', 60, 0, 'MON,TUE,WED,THU,FRI'); \
             INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, department_id, schedule_id, archived_at, separation_date) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2025-01-06', 'REGULAR', 1, 1, NULL, NULL), \
             (2, 'EMP-2', 'Ana', 'Reyes', '2025-01-06', 'REGULAR', 1, 2, NULL, NULL), \
             (3, 'EMP-3', 'Leo', 'Santos', '2025-01-06', 'REGULAR', NULL, NULL, NULL, NULL), \
             (4, 'EMP-4', 'Mia', 'Cruz', '2027-01-04', 'PROBATIONARY', 1, 1, NULL, NULL), \
             (5, 'EMP-5', 'Ben', 'Lim', '2020-01-06', 'RESIGNED', 1, 1, '2026-01-01T00:00:00Z', '2025-12-01'), \
             (6, 'EMP-6', 'Kim', 'Tan', '2020-01-06', 'RESIGNED', 1, 1, NULL, '2026-09-30')",
        )
        .execute(&mut *conn)
        .await
        .expect("fixture");
        drop(conn);
        (dir, db)
    }

    fn input(
        employee_id: i64,
        date: &str,
        time_in: &str,
        time_out: Option<&str>,
    ) -> AttendanceInput {
        AttendanceInput {
            employee_id,
            work_date: date.into(),
            time_in: time_in.into(),
            time_out: time_out.map(Into::into),
            reason: "Forgot to clock in".into(),
        }
    }

    async fn flag(db: &SqlitePool, employee_id: i64, date: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source, \
             needs_review, review_note) VALUES (?, ?, ?, 'PRESENT', 'CLOCK', 1, 'PC clock') \
             RETURNING id",
        )
        .bind(employee_id)
        .bind(date)
        .bind(format!("{date}T08:00:00"))
        .fetch_one(db)
        .await
        .expect("flagged record")
    }

    fn statuses(rows: &[DayRow]) -> Vec<(&str, Option<&str>)> {
        rows.iter()
            .map(|r| (r.employee_no.as_str(), r.status.as_deref()))
            .collect()
    }

    #[tokio::test]
    async fn the_grid_shows_current_employees_and_works_out_missing_days() {
        let (_dir, db) = db().await;
        flag(&db, 1, "2026-10-05").await;
        let monday = day(&db, "2026-10-05").await.expect("grid");
        // Sorted by name; the future hire, the archived and the departed are left out.
        assert_eq!(
            statuses(&monday),
            [
                ("EMP-1", Some("PRESENT")),
                ("EMP-2", Some("ABSENT")),
                ("EMP-3", None),
            ]
        );
        assert_eq!(monday[0].department_name.as_deref(), Some("Operations"));
        assert!(monday[0].record.as_ref().is_some_and(|r| r.needs_review));

        let sunday = day(&db, "2026-10-04").await.expect("grid");
        assert_eq!(sunday[1].status.as_deref(), Some("REST_DAY"));
    }

    #[tokio::test]
    async fn a_holiday_names_the_day_and_excuses_the_absence() {
        let (_dir, db) = db().await;
        sqlx::query(
            "INSERT INTO holidays (date, name, type) VALUES \
             ('2026-11-30', 'Bonifacio Day', 'REGULAR')",
        )
        .execute(&db)
        .await
        .expect("holiday");
        let rows = day(&db, "2026-11-30").await.expect("grid");
        assert_eq!(rows[0].status.as_deref(), Some("HOLIDAY"));
        assert_eq!(rows[0].holiday.as_deref(), Some("Bonifacio Day"));
    }

    #[tokio::test]
    async fn hr_adds_a_missing_day_with_a_reason() {
        let (_dir, db) = db().await;
        let r = save(
            &db,
            hr(),
            input(1, "2026-10-05", "08:25", Some("17:00")),
            today(),
            now(),
        )
        .await
        .expect("saved");
        assert_eq!(r.time_in.as_deref(), Some("2026-10-05T08:25:00"));
        assert_eq!(
            (r.source.as_str(), r.status.as_str()),
            ("MANUAL", "PRESENT")
        );
        assert_eq!((r.late_minutes, r.worked_minutes), (15, 455));

        let rows = audit_rows::all(&db).await;
        let last = rows.last().expect("audit");
        assert_eq!(last.action, "attendance.add");
        assert!(last.before_json.is_none());
        assert!(last
            .after_json
            .as_deref()
            .unwrap_or("")
            .contains("Forgot to clock in"));
    }

    #[tokio::test]
    async fn a_correction_replaces_the_punch_clears_the_flag_and_keeps_the_before() {
        let (_dir, db) = db().await;
        let id = flag(&db, 1, "2026-10-05").await;
        let mut fix = input(1, "2026-10-05", "07:55", Some("17:05"));
        fix.reason = "PC clock was wrong; checked CCTV".into();
        let r = save(&db, hr(), fix, today(), now()).await.expect("saved");
        assert_eq!(r.id, id);
        assert!(!r.needs_review);
        assert_eq!(r.source, "MANUAL");
        assert_eq!((r.late_minutes, r.worked_minutes), (0, 480));

        let edited_by: Option<i64> =
            sqlx::query_scalar("SELECT edited_by FROM attendance_records WHERE id = ?")
                .bind(id)
                .fetch_one(&db)
                .await
                .expect("edited_by");
        assert_eq!(edited_by, Some(1));
        let rows = audit_rows::all(&db).await;
        let last = rows.last().expect("audit");
        assert_eq!(last.action, "attendance.correct");
        assert!(last
            .before_json
            .as_deref()
            .unwrap_or("")
            .contains("\"source\":\"CLOCK\""));
        assert!(review_queue(&db, wall("2026-10-05 09:00"))
            .await
            .expect("queue")
            .is_empty());
    }

    #[tokio::test]
    async fn night_shift_times_after_midnight_land_on_the_next_day() {
        let (_dir, db) = db().await;
        let r = save(
            &db,
            hr(),
            input(2, "2026-10-05", "00:15", Some("05:00")),
            today(),
            now(),
        )
        .await
        .expect("saved");
        assert_eq!(r.time_in.as_deref(), Some("2026-10-06T00:15:00"));
        assert_eq!(r.time_out.as_deref(), Some("2026-10-06T05:00:00"));
        assert_eq!(r.late_minutes, 135);

        let r = save(
            &db,
            hr(),
            input(2, "2026-10-06", "21:58", Some("06:02")),
            today(),
            now(),
        )
        .await
        .expect("saved");
        assert_eq!(r.time_in.as_deref(), Some("2026-10-06T21:58:00"));
        assert_eq!(r.time_out.as_deref(), Some("2026-10-07T06:02:00"));
        assert_eq!(r.worked_minutes, 420);
    }

    #[tokio::test]
    async fn a_correction_needs_a_reason_valid_times_and_a_past_date() {
        let (_dir, db) = db().await;
        let mut no_reason = input(1, "2026-10-05", "08:00", None);
        no_reason.reason = "  ".into();
        let cases = [
            (no_reason, "reason"),
            (input(1, "2026-10-05", "8am", None), "timeIn"),
            (input(1, "2026-10-05", "08:00", Some("25:00")), "timeOut"),
            (input(1, "2026-10-10", "08:00", None), "workDate"),
        ];
        for (bad, name) in cases {
            match save(&db, hr(), bad, today(), now()).await {
                Err(AppError::Validation(f)) => assert_eq!(f[0].field, name),
                other => panic!("{name}: expected a validation error, got {other:?}"),
            }
        }
        // No time out yet is fine: they may still be at work.
        save(
            &db,
            hr(),
            input(1, "2026-10-09", "08:00", Some("")),
            today(),
            now(),
        )
        .await
        .expect("open day");
    }

    #[tokio::test]
    async fn locked_and_archived_days_cannot_change() {
        let (_dir, db) = db().await;
        let id = flag(&db, 1, "2026-10-05").await;
        sqlx::query("UPDATE attendance_records SET locked_by_period_id = 1 WHERE id = ?")
            .bind(id)
            .execute(&db)
            .await
            .expect("lock");
        let r = save(
            &db,
            hr(),
            input(1, "2026-10-05", "08:00", None),
            today(),
            now(),
        )
        .await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));

        let r = save(
            &db,
            hr(),
            input(5, "2025-11-03", "08:00", None),
            today(),
            now(),
        )
        .await;
        assert!(matches!(r, Err(AppError::Conflict(_))));
        let r = save(
            &db,
            hr(),
            input(99, "2026-10-05", "08:00", None),
            today(),
            now(),
        )
        .await;
        assert!(matches!(r, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn marking_reviewed_clears_the_flag_once_and_is_audited() {
        let (_dir, db) = db().await;
        let id = flag(&db, 2, "2026-10-05").await;
        let queue = review_queue(&db, wall("2026-10-05 09:00"))
            .await
            .expect("queue");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].employee_name, "Reyes, Ana");
        assert_eq!(queue[0].record.review_note.as_deref(), Some("PC clock"));

        let r = mark_reviewed(&db, hr(), id, now()).await.expect("reviewed");
        assert!(!r.needs_review);
        mark_reviewed(&db, hr(), id, now()).await.expect("again");
        let reviews = audit_rows::actions(&db)
            .await
            .into_iter()
            .filter(|a| a == "attendance.review")
            .count();
        assert_eq!(reviews, 1);
        assert!(review_queue(&db, wall("2026-10-05 09:00"))
            .await
            .expect("queue")
            .is_empty());
    }

    #[tokio::test]
    async fn a_forgotten_time_out_joins_the_queue_after_a_day() {
        let (_dir, db) = db().await;
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source) \
             VALUES (1, '2026-10-05', '2026-10-05T08:00:00', 'PRESENT', 'CLOCK') RETURNING id",
        )
        .fetch_one(&db)
        .await
        .expect("open record");
        // Still at work the same evening: not in the queue.
        assert!(review_queue(&db, wall("2026-10-05 20:00"))
            .await
            .expect("q")
            .is_empty());

        let queue = review_queue(&db, wall("2026-10-06 09:00"))
            .await
            .expect("q");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].record.id, id);
        assert!(!queue[0].record.needs_review);
        assert_eq!(
            queue[0].record.review_note.as_deref(),
            Some("No time out was recorded.")
        );

        // Adding the time out settles it.
        save(
            &db,
            hr(),
            input(1, "2026-10-05", "08:00", Some("17:00")),
            today(),
            now(),
        )
        .await
        .expect("fixed");
        assert!(review_queue(&db, wall("2026-10-06 09:00"))
            .await
            .expect("q")
            .is_empty());
    }

    async fn approve_leave(db: &SqlitePool, employee_id: i64, from: &str, to: &str, half: bool) {
        sqlx::query(
            "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             half_day, halfdays, reason, status) SELECT ?, id, ?, ?, ?, ?, 'Trip', 'APPROVED' \
             FROM leave_types WHERE code = 'VL'",
        )
        .bind(employee_id)
        .bind(from)
        .bind(to)
        .bind(half)
        .bind(if half { 1 } else { 4 })
        .execute(db)
        .await
        .expect("leave");
    }

    #[tokio::test]
    async fn approved_leave_shows_on_the_grid_and_blocks_entering_time() {
        let (_d, db) = db().await;
        approve_leave(&db, 1, "2026-10-05", "2026-10-06", false).await;
        approve_leave(&db, 2, "2026-10-05", "2026-10-05", true).await;
        let rows = day(&db, "2026-10-05").await.expect("grid");
        let juan = rows
            .iter()
            .find(|r| r.employee_no == "EMP-1")
            .expect("juan");
        assert_eq!(
            (juan.status.as_deref(), juan.leave.as_deref()),
            (Some("ON_LEAVE"), Some("Vacation Leave"))
        );
        assert!(
            juan.full_day_leave,
            "a whole day off leaves nothing to record"
        );
        let ana = rows.iter().find(|r| r.employee_no == "EMP-2").expect("ana");
        assert_eq!(ana.leave.as_deref(), Some("Vacation Leave (half day)"));
        assert!(
            !ana.full_day_leave,
            "a half day still leaves the other half to record"
        );
        // The next week is back to normal.
        let later = day(&db, "2026-10-07").await.expect("grid");
        assert_eq!(
            later
                .iter()
                .find(|r| r.employee_no == "EMP-1")
                .and_then(|r| r.status.as_deref()),
            Some("ABSENT")
        );

        let err = save(
            &db,
            hr(),
            input(1, "2026-10-06", "08:00", Some("17:00")),
            today(),
            now(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Conflict(ON_LEAVE)), "{err:?}");
        // A half day off still leaves the other half to record.
        save(
            &db,
            hr(),
            input(2, "2026-10-05", "22:00", Some("02:00")),
            today(),
            now(),
        )
        .await
        .expect("half day worked");
    }
}
