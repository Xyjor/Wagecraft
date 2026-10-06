//! Overtime requests (plan §6.3). Overtime is paid only once HR approves it. A request must
//! fall outside the employee's shift (or on a rest day or day-off holiday), last at most
//! 12 hours, and not overlap another live request. Nothing changes on a date that is in a
//! posted payroll period. Every step is audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::attendance::DateRange;
use crate::domain::attendance_calc::{HolidayKind, Interval, Shift};
use crate::domain::employee::employment_covers;
use crate::domain::overtime::{self as rules, OvertimeInput, OvertimeRequest, MAX_MINUTES};
use crate::error::AppError;
use crate::repositories::attendance as attendance_repo;
use crate::repositories::leave_requests as leave_repo;
use crate::repositories::overtime::{self as repo, NewRequest, OvertimeEmployee};
use crate::services::attendance::{check_range, strongest_holiday};
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

const DECIDED: &str = "This request has already been decided.";
const CHANGED: &str = "Someone else just changed this request. Refresh to see it.";
const ON_LEAVE: &str = "There's whole-day leave filed for this day. Settle the leave first.";
const HOLIDAY_OVERTIME: &str = "Overtime during work hours was filed on this date because it \
    was a holiday off. Cancel that overtime first, then change the holiday.";
const LOCKED: &str = "This day is in a posted payroll period, so its overtime can't change.";
/// How far ahead overtime can be filed, so a planned job can be approved before the day.
const MAX_DAYS_AHEAD: i64 = 31;
const STATUSES: [&str; 4] = ["PENDING", "APPROVED", "REJECTED", "CANCELLED"];

/// The signed-in person's own requests.
pub async fn mine(
    db: &SqlitePool,
    employee_id: i64,
    range: &DateRange,
) -> Result<Vec<OvertimeRequest>, AppError> {
    let (from, to) = check_range(range)?;
    Ok(repo::for_employee(db, employee_id, &from.to_string(), &to.to_string()).await?)
}

/// HR's list over a range, optionally one status.
pub async fn list(
    db: &SqlitePool,
    status: Option<&str>,
    range: &DateRange,
) -> Result<Vec<OvertimeRequest>, AppError> {
    let (from, to) = check_range(range)?;
    if status.is_some_and(|s| !STATUSES.contains(&s)) {
        return Err(AppError::Validation(vec![field("status", "Pick a status")]));
    }
    Ok(repo::list(db, status, &from.to_string(), &to.to_string()).await?)
}

/// Every request still waiting for HR, whatever its date.
pub async fn pending(db: &SqlitePool) -> Result<Vec<OvertimeRequest>, AppError> {
    Ok(repo::pending(db).await?)
}

fn shift_of(e: &OvertimeEmployee) -> Option<(Shift, &str)> {
    let t = |s: &Option<String>| {
        s.as_deref()
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
    };
    let shift = Shift {
        start: t(&e.start_time)?,
        end: t(&e.end_time)?,
        break_minutes: e.break_minutes?,
        grace_minutes: e.grace_minutes?,
    };
    Some((shift, e.work_days.as_deref()?))
}

/// Files a request. With `input.employee_no` it is HR filing for that employee (the
/// command checks HR's permission); without it, it is for `own_employee`.
pub async fn file(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    input: OvertimeInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<OvertimeRequest, AppError> {
    let mut errors = Vec::new();
    let work_date = NaiveDate::parse_from_str(input.work_date.trim(), "%Y-%m-%d").ok();
    match work_date {
        None => errors.push(field("workDate", "Pick a date")),
        Some(d) if d > today + Duration::days(MAX_DAYS_AHEAD) => {
            errors.push(field("workDate", "File overtime at most 31 days ahead"))
        }
        Some(_) => {}
    }
    let time = |s: &str, name: &str, errors: &mut Vec<_>| {
        let t = NaiveTime::parse_from_str(s.trim(), "%H:%M").ok();
        if t.is_none() {
            errors.push(field(name, "Enter a time like 17:00"));
        }
        t
    };
    let start = time(&input.start_time, "startTime", &mut errors);
    let end = time(&input.end_time, "endTime", &mut errors);
    let reason = input.reason.trim();
    if !(3..=200).contains(&reason.chars().count()) {
        errors.push(field(
            "reason",
            "Say what the overtime is for, in 3 to 200 characters",
        ));
    }
    let (Some(work_date), Some(start), Some(end)) = (work_date, start, end) else {
        return Err(AppError::Validation(errors));
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let mut tx = db.begin().await?;
    let on = work_date.to_string();
    let employee = match input.employee_no.as_deref().map(str::trim) {
        Some(no) => repo::employee_by_no(&mut tx, no, &on)
            .await?
            .ok_or_else(|| {
                AppError::Validation(vec![field("employeeNo", "No employee has this number")])
            })?,
        None => {
            let id = own_employee.ok_or(AppError::Conflict(
                "Your account isn't linked to an employee record, so it can't file overtime.",
            ))?;
            repo::employee_by_id(&mut tx, id, &on)
                .await?
                .ok_or(AppError::NotFound("Employee"))?
        }
    };
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "This employee is archived, so overtime can't be filed for them.",
        ));
    }
    let date = work_date.to_string();
    if let Err(message) = employment_covers(
        &employee.hire_date,
        employee.separation_date.as_deref(),
        &date,
    ) {
        return Err(AppError::Validation(vec![field("workDate", &message)]));
    }
    let Some((shift, work_days)) = shift_of(&employee) else {
        return Err(AppError::Conflict(
            "This employee has no work schedule on this date. HR needs to set one first.",
        ));
    };
    if repo::day_locked(&mut tx, employee.id, &date).await? {
        return Err(AppError::Conflict(LOCKED));
    }
    if leave_repo::full_day_leave_on(&mut tx, employee.id, &date, true).await? {
        return Err(AppError::Validation(vec![field("workDate", ON_LEAVE)]));
    }

    let ot = rules::place(work_date, start, end, Some(&shift));
    if ot.minutes() > MAX_MINUTES {
        return Err(AppError::Validation(vec![field(
            "endTime",
            "Overtime can be at most 12 hours",
        )]));
    }
    let holidays = attendance_repo::holidays_on(&mut *tx, &date).await?;
    let day_off = matches!(
        strongest_holiday(&holidays),
        Some(HolidayKind::Regular | HolidayKind::SpecialNonWorking)
    );
    if rules::clashes_with_shift(&ot, work_date, &shift, work_days, day_off) {
        return Err(AppError::Validation(vec![field(
            "startTime",
            "Overtime must be outside the work schedule",
        )]));
    }
    let start_at = time::to_local_db(ot.start);
    let end_at = time::to_local_db(ot.end);
    if repo::overlaps_active(&mut tx, employee.id, &start_at, &end_at).await? {
        return Err(AppError::Validation(vec![field(
            "startTime",
            "This overlaps another overtime request",
        )]));
    }

    let id = repo::insert(
        &mut tx,
        &NewRequest {
            employee_id: employee.id,
            work_date: &date,
            start_at: &start_at,
            end_at: &end_at,
            minutes: ot.minutes(),
            reason,
            filed_by: actor.user_id,
        },
    )
    .await?;
    let created = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Overtime request"))?;
    let entry = Entry {
        action: "overtime.file",
        entity: Some(("overtime_request", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

/// Approves or rejects a pending request. A rejection needs a note. HR can't decide their
/// own overtime.
pub async fn decide(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    id: i64,
    approve: bool,
    note: Option<String>,
    now: DateTime<Utc>,
) -> Result<OvertimeRequest, AppError> {
    let note = note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned);
    match &note {
        None if !approve => {
            return Err(AppError::Validation(vec![field(
                "note",
                "Say why it's rejected, in 3 to 200 characters",
            )]))
        }
        Some(n) if !(3..=200).contains(&n.chars().count()) => {
            return Err(AppError::Validation(vec![field(
                "note",
                "Keep the note to 3 to 200 characters",
            )]))
        }
        _ => {}
    }

    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Overtime request"))?;
    if before.status != "PENDING" {
        return Err(AppError::Conflict(DECIDED));
    }
    if own_employee == Some(before.employee_id) {
        return Err(AppError::Conflict(
            "Someone else needs to decide your own overtime request.",
        ));
    }
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    if approve
        && leave_repo::full_day_leave_on(&mut tx, before.employee_id, &before.work_date, true)
            .await?
    {
        return Err(AppError::Conflict(ON_LEAVE));
    }
    let status = if approve { "APPROVED" } else { "REJECTED" };
    if !repo::set_status(
        &mut tx,
        id,
        "PENDING",
        status,
        actor.user_id,
        note.as_deref(),
        &time::to_db(now),
    )
    .await?
    {
        return Err(AppError::Conflict(DECIDED));
    }
    let after = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Overtime request"))?;
    let entry = Entry {
        action: if approve {
            "overtime.approve"
        } else {
            "overtime.reject"
        },
        entity: Some(("overtime_request", id)),
        before: Some(json!({ "status": before.status })),
        after: Some(json!({ "status": status, "note": note, "employeeNo": after.employee_no })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// The employee can cancel their own pending request. HR can also cancel an approved one,
/// as long as payroll for that day isn't posted.
pub async fn cancel(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    can_decide: bool,
    id: i64,
    now: DateTime<Utc>,
) -> Result<OvertimeRequest, AppError> {
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Overtime request"))?;
    let owner = own_employee == Some(before.employee_id);
    if !owner && !can_decide {
        // Someone else's request: don't even confirm it exists.
        return Err(AppError::NotFound("Overtime request"));
    }
    let allowed = match before.status.as_str() {
        "PENDING" => true,
        "APPROVED" => can_decide,
        _ => false,
    };
    if !allowed {
        return Err(AppError::Conflict(if before.status == "APPROVED" {
            "This request is already approved. Ask HR to cancel it."
        } else {
            "This request is already closed."
        }));
    }
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    if !repo::set_status(
        &mut tx,
        id,
        &before.status,
        "CANCELLED",
        actor.user_id,
        None,
        &time::to_db(now),
    )
    .await?
    {
        return Err(AppError::Conflict(CHANGED));
    }
    let after = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Overtime request"))?;
    let entry = Entry {
        action: "overtime.cancel",
        entity: Some(("overtime_request", id)),
        before: Some(json!({ "status": before.status })),
        after: Some(json!({ "status": "CANCELLED", "employeeNo": after.employee_no })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// After a holiday on `date` changed, checks that the overtime on that date still falls
/// outside the schedule. Overtime during shift hours is allowed only on a holiday off, so
/// removing that holiday would count the hours twice; the holiday change is refused.
pub(crate) async fn recheck_on(conn: &mut SqliteConnection, date: &str) -> Result<(), AppError> {
    let requests = repo::live_on(conn, date).await?;
    if requests.is_empty() {
        return Ok(());
    }
    let Ok(work_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return Ok(());
    };
    let holidays = attendance_repo::holidays_on(&mut *conn, date).await?;
    let day_off = matches!(
        strongest_holiday(&holidays),
        Some(HolidayKind::Regular | HolidayKind::SpecialNonWorking)
    );
    for r in requests {
        let Some(employee) = repo::employee_by_id(conn, r.employee_id, date).await? else {
            continue;
        };
        let (Some((shift, work_days)), Some(start), Some(end)) = (
            shift_of(&employee),
            time::from_local_db(&r.start_at),
            time::from_local_db(&r.end_at),
        ) else {
            continue;
        };
        let ot = Interval::new(start, end);
        if rules::clashes_with_shift(&ot, work_date, &shift, work_days, day_off) {
            return Err(AppError::Conflict(HOLIDAY_OVERTIME));
        }
    }
    Ok(())
}

/// The first day on or after `from` where the employee's live overtime falls inside the
/// schedule they now have for that day. Used before a schedule move is saved.
pub(crate) async fn first_clash_from(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: NaiveDate,
) -> Result<Option<NaiveDate>, AppError> {
    let day_before = (from - Duration::days(1)).to_string();
    for r in repo::live_after(conn, employee_id, &day_before).await? {
        let Ok(work_date) = NaiveDate::parse_from_str(&r.work_date, "%Y-%m-%d") else {
            continue;
        };
        let holidays = attendance_repo::holidays_on(&mut *conn, &r.work_date).await?;
        let day_off = matches!(
            strongest_holiday(&holidays),
            Some(HolidayKind::Regular | HolidayKind::SpecialNonWorking)
        );
        let Some(employee) = repo::employee_by_id(conn, employee_id, &r.work_date).await? else {
            continue;
        };
        let (Some((shift, work_days)), Some(start), Some(end)) = (
            shift_of(&employee),
            time::from_local_db(&r.start_at),
            time::from_local_db(&r.end_at),
        ) else {
            continue;
        };
        let ot = Interval::new(start, end);
        if rules::clashes_with_shift(&ot, work_date, &shift, work_days, day_off) {
            return Ok(Some(work_date));
        }
    }
    Ok(None)
}

/// Cancels an employee's pending and approved overtime after their last day of work.
pub(crate) async fn cancel_after_separation(
    conn: &mut SqliteConnection,
    actor: Actor<'_>,
    employee_id: i64,
    last_day: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let stamp = time::to_db(now);
    for r in repo::live_after(conn, employee_id, last_day).await? {
        if r.locked {
            return Err(AppError::Conflict(LOCKED));
        }
        if !repo::set_status(
            conn,
            r.id,
            &r.status,
            "CANCELLED",
            actor.user_id,
            None,
            &stamp,
        )
        .await?
        {
            return Err(AppError::Conflict(CHANGED));
        }
        let entry = Entry {
            action: "overtime.cancel",
            entity: Some(("overtime_request", r.id)),
            before: Some(json!({ "status": r.status })),
            after: Some(json!({
                "status": "CANCELLED",
                "employeeNo": r.employee_no,
                "reason": "After the employee's last day",
            })),
        };
        audit::record(conn, now, actor, entry).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::services::payroll_fixtures::posted;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap()
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    fn juan() -> Actor<'static> {
        Actor {
            user_id: Some(2),
            username: "juan",
        }
    }

    /// Juan (1) works the office schedule; Ana (2) the night shift; Leo (3) has none.
    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("hr");
        crate::repositories::users::insert(&mut conn, "juan", "x", "STAFF", false)
            .await
            .expect("juan");
        drop(conn);
        sqlx::query(
            "INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days) VALUES \
             (1, 'Office', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI'), \
             (2, 'Night', '22:00', '06:00', 60, 0, 'MON,TUE,WED,THU,FRI'); \
             INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, schedule_id, archived_at) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2025-01-06', 'REGULAR', 1, NULL), \
             (2, 'EMP-2', 'Ana', 'Reyes', '2025-01-06', 'REGULAR', 2, NULL), \
             (3, 'EMP-3', 'Leo', 'Santos', '2025-01-06', 'REGULAR', NULL, NULL), \
             (4, 'EMP-4', 'Old', 'Hand', '2020-01-06', 'RESIGNED', 1, '2026-01-01T00:00:00Z');",
        )
        .execute(&db)
        .await
        .expect("fixtures");
        (dir, db)
    }

    fn ot(date: &str, start: &str, end: &str) -> OvertimeInput {
        OvertimeInput {
            employee_no: None,
            work_date: date.into(),
            start_time: start.into(),
            end_time: end.into(),
            reason: "Month-end stock count".into(),
        }
    }

    fn for_no(no: &str, date: &str, start: &str, end: &str) -> OvertimeInput {
        OvertimeInput {
            employee_no: Some(no.into()),
            ..ot(date, start, end)
        }
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    async fn lock(db: &SqlitePool, employee_id: i64, date: &str) {
        sqlx::query(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source, \
             locked_by_period_id) VALUES (?, ?, ?, 'PRESENT', 'CLOCK', 1) \
             ON CONFLICT (employee_id, work_date) DO UPDATE SET locked_by_period_id = 1",
        )
        .bind(employee_id)
        .bind(date)
        .bind(format!("{date}T08:00:00"))
        .execute(db)
        .await
        .expect("lock");
    }

    #[tokio::test]
    async fn an_employee_files_overtime_after_their_shift() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        assert_eq!(r.status, "PENDING");
        assert_eq!(r.employee_no, "EMP-1");
        assert_eq!(r.start_at, "2026-10-07T17:00:00");
        assert_eq!(r.end_at, "2026-10-07T20:00:00");
        assert_eq!(r.minutes, 180);
        assert_eq!(r.filed_by_username.as_deref(), Some("juan"));
    }

    #[tokio::test]
    async fn overtime_is_checked_against_the_schedule_that_day_had() {
        let (_d, db) = db().await;
        // From Thu Oct 8 Juan works 09:00–18:00.
        sqlx::query(
            "INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days) VALUES \
             (3, 'Late', '09:00', '18:00', 60, 0, 'MON,TUE,WED,THU,FRI'); \
             INSERT INTO schedule_assignments (employee_id, schedule_id, effective_from) \
             VALUES (1, 3, '2026-10-08')",
        )
        .execute(&db)
        .await
        .expect("move");
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "19:00"),
            today(),
            t0(),
        )
        .await
        .expect("after the old 17:00 end");
        let err = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-08", "17:00", "19:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&err, AppError::Validation(f) if f.iter().any(|e| e.message.contains("outside the work schedule"))),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn night_shift_overtime_lands_on_the_next_morning() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            hr(),
            None,
            for_no("emp-2", "2026-10-07", "06:00", "08:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        assert_eq!(r.start_at, "2026-10-08T06:00:00");
        assert_eq!(r.filed_by_username.as_deref(), Some("hr"));
    }

    #[tokio::test]
    async fn the_request_is_checked() {
        let (_d, db) = db().await;
        let bad = OvertimeInput {
            reason: " ".into(),
            ..ot("2026-13-01", "5pm", "20:00")
        };
        let err = file(&db, juan(), Some(1), bad, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["workDate", "startTime", "reason"]);

        let far = ot("2026-11-30", "17:00", "20:00");
        let err = file(&db, juan(), Some(1), far, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["workDate"]);

        let long = ot("2026-10-10", "06:00", "18:01");
        let err = file(&db, juan(), Some(1), long, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["endTime"]);

        let unknown = for_no("EMP-99", "2026-10-07", "17:00", "20:00");
        let err = file(&db, hr(), None, unknown, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["employeeNo"]);
    }

    #[tokio::test]
    async fn overtime_must_be_outside_the_schedule_unless_the_day_is_off() {
        let (_d, db) = db().await;
        let during = ot("2026-10-07", "16:00", "19:00");
        let err = file(&db, juan(), Some(1), during, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["startTime"]);

        // Saturday is a rest day.
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-10", "08:00", "17:00"),
            today(),
            t0(),
        )
        .await
        .expect("rest day");

        // A regular holiday is a day off, a special working day is not.
        sqlx::query(
            "INSERT INTO holidays (date, name, type) VALUES \
             ('2026-10-12', 'Test Holiday', 'REGULAR'), \
             ('2026-10-13', 'Working Day', 'SPECIAL_WORKING')",
        )
        .execute(&db)
        .await
        .expect("holidays");
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-12", "08:00", "17:00"),
            today(),
            t0(),
        )
        .await
        .expect("holiday");
        let err = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-13", "08:00", "12:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["startTime"]);
    }

    #[tokio::test]
    async fn live_requests_may_not_overlap() {
        let (_d, db) = db().await;
        let first = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("first");
        let err = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "19:00", "21:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["startTime"]);
        // Back to back is fine, and a cancelled request frees its time.
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "20:00", "21:00"),
            today(),
            t0(),
        )
        .await
        .expect("back to back");
        cancel(&db, juan(), Some(1), false, first.id, t0())
            .await
            .expect("cancel");
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "18:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("after cancel");
    }

    #[tokio::test]
    async fn some_employees_cant_have_overtime_filed() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            hr(),
            None,
            for_no("EMP-3", "2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await;
        assert!(
            matches!(r, Err(AppError::Conflict(_))),
            "no schedule: {r:?}"
        );
        let r = file(
            &db,
            hr(),
            None,
            for_no("EMP-4", "2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await;
        assert!(matches!(r, Err(AppError::Conflict(_))), "archived: {r:?}");
        let r = file(
            &db,
            hr(),
            None,
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await;
        assert!(
            matches!(r, Err(AppError::Conflict(_))),
            "unlinked account: {r:?}"
        );
    }

    #[tokio::test]
    async fn hr_approves_or_rejects_with_a_reason() {
        let (_d, db) = db().await;
        let a = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("a");
        let b = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-08", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("b");

        let approved = decide(&db, hr(), None, a.id, true, None, t0())
            .await
            .expect("approve");
        assert_eq!(approved.status, "APPROVED");
        assert_eq!(approved.decided_by_username.as_deref(), Some("hr"));
        assert_eq!(approved.decided_at.as_deref(), Some("2026-10-07T09:00:00Z"));

        let err = decide(&db, hr(), None, b.id, false, Some("  ".into()), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["note"]);
        let rejected = decide(
            &db,
            hr(),
            None,
            b.id,
            false,
            Some("Not needed".into()),
            t0(),
        )
        .await
        .expect("reject");
        assert_eq!(rejected.status, "REJECTED");
        assert_eq!(rejected.decision_note.as_deref(), Some("Not needed"));

        let again = decide(
            &db,
            hr(),
            None,
            a.id,
            false,
            Some("Changed mind".into()),
            t0(),
        )
        .await;
        assert!(matches!(again, Err(AppError::Conflict(_))));
        let pending = pending(&db).await.expect("pending");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn nobody_decides_their_own_overtime() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        // Juan's account also has HR rights in this test.
        let own = decide(&db, juan(), Some(1), r.id, true, None, t0()).await;
        assert!(matches!(own, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn who_can_cancel_what() {
        let (_d, db) = db().await;
        let mine = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        let anas = file(
            &db,
            hr(),
            None,
            for_no("EMP-2", "2026-10-07", "06:00", "08:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        // Staff can't see, let alone cancel, someone else's request.
        let r = cancel(&db, juan(), Some(1), false, anas.id, t0()).await;
        assert!(matches!(r, Err(AppError::NotFound(_))));

        decide(&db, hr(), None, mine.id, true, None, t0())
            .await
            .expect("approve");
        let r = cancel(&db, juan(), Some(1), false, mine.id, t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(_))), "approved: {r:?}");
        let c = cancel(&db, hr(), None, true, mine.id, t0())
            .await
            .expect("hr cancels");
        assert_eq!(c.status, "CANCELLED");
        let r = cancel(&db, hr(), None, true, mine.id, t0()).await;
        assert!(
            matches!(r, Err(AppError::Conflict(_))),
            "already closed: {r:?}"
        );
    }

    #[tokio::test]
    async fn posted_days_are_locked() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        lock(&db, 1, "2026-10-07").await;
        lock(&db, 1, "2026-10-06").await;
        let new = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-06", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await;
        assert!(matches!(new, Err(AppError::Conflict(LOCKED))));
        let d = decide(&db, hr(), None, r.id, true, None, t0()).await;
        assert!(matches!(d, Err(AppError::Conflict(LOCKED))));
        let c = cancel(&db, juan(), Some(1), false, r.id, t0()).await;
        assert!(matches!(c, Err(AppError::Conflict(LOCKED))));
    }

    #[tokio::test]
    async fn a_posted_period_locks_its_days_even_without_attendance() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        posted(&db, "2026-10-01", "2026-10-15").await;

        let read = repo::by_id(&db, r.id).await.expect("read");
        assert!(read.expect("row").locked);
        let new = ot("2026-10-15", "17:00", "20:00");
        let f = file(&db, juan(), Some(1), new, today(), t0()).await;
        assert!(matches!(f, Err(AppError::Conflict(LOCKED))));
        let d = decide(&db, hr(), None, r.id, true, None, t0()).await;
        assert!(matches!(d, Err(AppError::Conflict(LOCKED))));
        let c = cancel(&db, juan(), Some(1), false, r.id, t0()).await;
        assert!(matches!(c, Err(AppError::Conflict(LOCKED))));
        // The day after the period is still open.
        let after = ot("2026-10-16", "17:00", "20:00");
        file(&db, juan(), Some(1), after, today(), t0())
            .await
            .expect("after the period");
    }

    #[tokio::test]
    async fn lists_and_audits() {
        let (_d, db) = db().await;
        let a = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("a");
        file(
            &db,
            hr(),
            None,
            for_no("EMP-2", "2026-10-08", "06:00", "08:00"),
            today(),
            t0(),
        )
        .await
        .expect("b");
        decide(&db, hr(), None, a.id, true, None, t0())
            .await
            .expect("approve");

        let oct = DateRange {
            from: "2026-10-01".into(),
            to: "2026-10-31".into(),
        };
        assert_eq!(list(&db, None, &oct).await.expect("all").len(), 2);
        let approved = list(&db, Some("APPROVED"), &oct).await.expect("approved");
        assert_eq!(approved.len(), 1);
        assert_eq!(mine(&db, 2, &oct).await.expect("ana's").len(), 1);
        assert_eq!(pending(&db).await.expect("pending").len(), 1);
        let err = list(&db, Some("MAYBE"), &oct).await.unwrap_err();
        assert_eq!(fields(err), ["status"]);

        assert_eq!(
            audit_rows::actions(&db).await,
            ["overtime.file", "overtime.file", "overtime.approve"]
        );
    }

    #[tokio::test]
    async fn an_approval_and_a_rejection_at_the_same_moment_decide_once() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-07", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        let (a, b) = tokio::join!(
            decide(&db, hr(), None, r.id, true, None, t0()),
            decide(
                &db,
                hr(),
                None,
                r.id,
                false,
                Some("Not needed".into()),
                t0()
            ),
        );
        let refused = [a, b]
            .into_iter()
            .filter_map(Result::err)
            .collect::<Vec<_>>();
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(
            matches!(refused[0], AppError::Conflict(DECIDED)),
            "{refused:?}"
        );
        let decisions = audit_rows::actions(&db).await;
        assert_eq!(decisions, ["overtime.file", "overtime.approve"]);
    }

    async fn leave_on(db: &SqlitePool, employee_id: i64, date: &str, status: &str, half: bool) {
        sqlx::query(
            "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             half_day, halfdays, reason, status) SELECT ?, id, ?, ?, ?, ?, 'Trip', ? \
             FROM leave_types WHERE code = 'VL'",
        )
        .bind(employee_id)
        .bind(date)
        .bind(date)
        .bind(half)
        .bind(if half { 1 } else { 2 })
        .bind(status)
        .execute(db)
        .await
        .expect("leave");
    }

    #[tokio::test]
    async fn overtime_cant_be_on_a_day_of_leave() {
        let (_d, db) = db().await;
        leave_on(&db, 1, "2026-10-13", "PENDING", false).await;
        let err = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-13", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["workDate"]);
        // A half day off leaves the rest of the day to work, so overtime is fine.
        leave_on(&db, 1, "2026-10-14", "APPROVED", true).await;
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-14", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("after a half day");

        // Leave filed after the overtime stops the approval.
        let r = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-15", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("file");
        leave_on(&db, 1, "2026-10-15", "APPROVED", false).await;
        let err = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(ON_LEAVE)), "{err:?}");
    }

    #[tokio::test]
    async fn a_holiday_cant_go_while_overtime_depends_on_it() {
        use crate::domain::org::HolidayInput;
        use crate::services::holidays;
        let (_d, db) = db().await;
        let holiday = |date: &str, kind: &str| HolidayInput {
            date: date.into(),
            name: "Test Holiday".into(),
            kind: kind.into(),
        };
        // Monday Oct 12 is a holiday off, so a full shift of overtime is allowed.
        let h = holidays::create(&db, hr(), holiday("2026-10-12", "REGULAR"), t0())
            .await
            .expect("holiday");
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-12", "08:00", "17:00"),
            today(),
            t0(),
        )
        .await
        .expect("holiday overtime");
        let err = holidays::delete(&db, hr(), h.id, t0()).await.unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(HOLIDAY_OVERTIME)),
            "{err:?}"
        );
        let err = holidays::update(
            &db,
            hr(),
            h.id,
            holiday("2026-10-12", "SPECIAL_WORKING"),
            t0(),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(HOLIDAY_OVERTIME)),
            "{err:?}"
        );
        assert_eq!(holidays::list(&db, 2026).await.expect("list").len(), 1);

        // Overtime after hours never depended on the holiday, so that one can go.
        let h2 = holidays::create(&db, hr(), holiday("2026-10-19", "REGULAR"), t0())
            .await
            .expect("holiday");
        file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-19", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("evening overtime");
        holidays::delete(&db, hr(), h2.id, t0())
            .await
            .expect("delete");
    }

    #[tokio::test]
    async fn a_separation_cancels_overtime_after_the_last_day() {
        let (_d, db) = db().await;
        let before = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-20", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("before");
        let after = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-27", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .expect("after");
        decide(&db, hr(), None, after.id, true, None, t0())
            .await
            .expect("approve");
        let mut conn = db.acquire().await.expect("conn");
        cancel_after_separation(&mut conn, hr(), 1, "2026-10-21", t0())
            .await
            .expect("cancel");
        drop(conn);
        assert_eq!(
            repo::by_id(&db, after.id)
                .await
                .expect("read")
                .expect("row")
                .status,
            "CANCELLED"
        );
        assert_eq!(
            repo::by_id(&db, before.id)
                .await
                .expect("read")
                .expect("row")
                .status,
            "PENDING"
        );
        assert_eq!(
            audit_rows::actions(&db).await.last().map(String::as_str),
            Some("overtime.cancel")
        );
    }

    #[tokio::test]
    async fn overtime_stays_within_the_employment_dates() {
        let (_d, db) = db().await;
        let early = file(
            &db,
            juan(),
            Some(1),
            ot("2025-01-02", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(early), ["workDate"]);
        sqlx::query("UPDATE employees SET separation_date = '2026-10-09' WHERE id = 1")
            .execute(&db)
            .await
            .expect("resign");
        let late = file(
            &db,
            juan(),
            Some(1),
            ot("2026-10-12", "17:00", "20:00"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(late), ["workDate"]);
    }
}
