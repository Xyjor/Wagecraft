//! Leave requests (plan §6.4). Staff file leave for themselves; HR files for anyone and
//! decides. Approving paid leave takes the days from the employee's balance in the same
//! transaction, and cancelling approved leave gives them back. Nothing changes on a day
//! that is in a posted payroll period. Every step is audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::attendance::DateRange;
use crate::domain::employee::employment_covers;
use crate::domain::leave::{count_halfdays_by_history, LeaveRequest, LeaveRequestInput};
use crate::error::AppError;
use crate::repositories::leave as leave_repo;
use crate::repositories::leave_requests::{self as repo, NewRequest};
use crate::repositories::overtime as overtime_repo;
use crate::repositories::schedules;
use crate::services::attendance::check_range;
use crate::services::auth::field;
use crate::services::leave::grant_in;
use crate::time;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashSet;

const NO_SCHEDULE: &str =
    "This employee has no work schedule for these dates. HR needs to set one first.";
const LOCKED: &str =
    "Some of these days are in a posted payroll period, so this leave can't change.";
const DECIDED: &str = "This request has already been decided.";
const CHANGED: &str = "Someone else just changed this request. Refresh to see it.";
const WORKED: &str =
    "The employee clocked in on a day in this leave. Correct that day first, or reject the request.";
const NO_DAYS_LEFT: &str = "This would leave a leave request with no working days in it. \
    Cancel that leave first, then change the holiday.";
const LONGER_THAN_BALANCE: &str = "This would make approved leave take more days than the \
    employee has left. Adjust their balance or cancel that leave first.";
const OVERTIME: &str =
    "There's overtime filed on a day in this leave. Reject or cancel the overtime first.";
const RUNS_PAST_SEPARATION: &str = "This employee has leave that starts before and ends after \
    their last day. Cancel that leave first, then save the separation.";
const NOT_ENOUGH: &str = "There aren't enough days left in this leave balance.";
/// Longest single request, in calendar days. Longer leave is filed in parts.
const MAX_SPAN_DAYS: i64 = 60;
/// How far ahead leave can be filed.
const MAX_DAYS_AHEAD: i64 = 365;
const STATUSES: [&str; 4] = ["PENDING", "APPROVED", "REJECTED", "CANCELLED"];

/// The signed-in person's own requests in a year, newest first.
pub async fn mine(
    db: &SqlitePool,
    employee_id: i64,
    year: i32,
) -> Result<Vec<LeaveRequest>, AppError> {
    if !(2000..=2100).contains(&year) {
        return Err(AppError::Validation(vec![field("year", "Pick a year")]));
    }
    let (from, to) = (format!("{year}-01-01"), format!("{year}-12-31"));
    Ok(repo::for_employee(db, employee_id, &from, &to).await?)
}

/// HR's list over a range, optionally one status.
pub async fn list(
    db: &SqlitePool,
    status: Option<&str>,
    range: &DateRange,
) -> Result<Vec<LeaveRequest>, AppError> {
    let (from, to) = check_range(range)?;
    if status.is_some_and(|s| !STATUSES.contains(&s)) {
        return Err(AppError::Validation(vec![field("status", "Pick a status")]));
    }
    Ok(repo::list(db, status, &from.to_string(), &to.to_string()).await?)
}

/// Every request still waiting for HR, whatever its dates.
pub async fn pending(db: &SqlitePool) -> Result<Vec<LeaveRequest>, AppError> {
    Ok(repo::pending(db).await?)
}

/// "5 days", "1½ days", "½ day".
fn days(halfdays: i64) -> String {
    let whole = halfdays / 2;
    let half = if halfdays % 2 == 1 { "½" } else { "" };
    match (whole, half) {
        (0, "") => "0 days".into(),
        (0, _) => "½ day".into(),
        (1, "") => "1 day".into(),
        _ => format!("{whole}{half} days"),
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

/// Files a request. With `input.employee_no` it is HR filing for that employee (the
/// command checks HR's permission); without it, it is for `own_employee`.
pub async fn file(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    input: LeaveRequestInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<LeaveRequest, AppError> {
    let mut errors = Vec::new();
    let start = parse_date(&input.start_date);
    let end = parse_date(&input.end_date);
    if start.is_none() {
        errors.push(field("startDate", "Pick a start date"));
    }
    if end.is_none() {
        errors.push(field("endDate", "Pick an end date"));
    }
    if let (Some(s), Some(e)) = (start, end) {
        if e < s {
            errors.push(field("endDate", "The end date is before the start date"));
        } else if s.year() != e.year() {
            errors.push(field(
                "endDate",
                "File leave across New Year as two requests, one for each year",
            ));
        } else if (e - s).num_days() >= MAX_SPAN_DAYS {
            errors.push(field("endDate", "File at most 60 days at a time"));
        } else if input.half_day && s != e {
            errors.push(field(
                "halfDay",
                "A half day starts and ends on the same date",
            ));
        }
        if s > today + Duration::days(MAX_DAYS_AHEAD) {
            errors.push(field("startDate", "File leave at most a year ahead"));
        }
    }
    let reason = input.reason.trim();
    if !(3..=200).contains(&reason.chars().count()) {
        errors.push(field(
            "reason",
            "Say what the leave is for, in 3 to 200 characters",
        ));
    }
    let (Some(start), Some(end)) = (start, end) else {
        return Err(AppError::Validation(errors));
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let mut tx = db.begin().await?;
    let employee = match input.employee_no.as_deref().map(str::trim) {
        Some(no) => repo::employee_by_no(&mut tx, no).await?.ok_or_else(|| {
            AppError::Validation(vec![field("employeeNo", "No employee has this number")])
        })?,
        None => {
            let id = own_employee.ok_or(AppError::Conflict(
                "Your account isn't linked to an employee record, so it can't file leave.",
            ))?;
            repo::employee_by_id(&mut tx, id)
                .await?
                .ok_or(AppError::NotFound("Employee"))?
        }
    };
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "This employee is archived, so leave can't be filed for them.",
        ));
    }
    let leave_type = leave_repo::type_by_id(&mut *tx, input.leave_type_id)
        .await?
        .filter(|t| t.is_active)
        .ok_or_else(|| AppError::Validation(vec![field("leaveTypeId", "Pick a leave type")]))?;

    let (from, to) = (start.to_string(), end.to_string());
    for (name, date) in [("startDate", &from), ("endDate", &to)] {
        if let Err(message) = employment_covers(
            &employee.hire_date,
            employee.separation_date.as_deref(),
            date,
        ) {
            return Err(AppError::Validation(vec![field(name, &message)]));
        }
    }
    let days_off: HashSet<NaiveDate> = repo::days_off(&mut tx, &from, &to)
        .await?
        .iter()
        .filter_map(|d| parse_date(d))
        .collect();
    let history = schedules::work_days_history(&mut tx, employee.id).await?;
    let halfdays = count_halfdays_by_history(start, end, input.half_day, &history, &days_off)
        .ok_or(AppError::Conflict(NO_SCHEDULE))?;
    if halfdays == 0 {
        return Err(AppError::Validation(vec![field(
            "startDate",
            "There are no working days in these dates",
        )]));
    }
    if repo::range_locked(&mut tx, employee.id, &from, &to).await? {
        return Err(AppError::Conflict(LOCKED));
    }
    if !input.half_day {
        if let Some(day) = repo::first_worked_day(&mut tx, employee.id, &from, &to).await? {
            return Err(AppError::Validation(vec![field(
                "startDate",
                &format!("There's a time in on {day}, so that day can't be leave. HR can correct it first."),
            )]));
        }
    }
    if !input.half_day {
        if let Some(day) = overtime_repo::first_live_in(&mut tx, employee.id, &from, &to).await? {
            return Err(AppError::Validation(vec![field(
                "startDate",
                &format!("There's overtime filed for {day}. Cancel it first, or file a half day."),
            )]));
        }
    }
    if repo::overlaps_active(&mut tx, employee.id, &from, &to).await? {
        return Err(AppError::Validation(vec![field(
            "startDate",
            "This overlaps another leave request",
        )]));
    }

    let year = start.year();
    if leave_type.is_paid {
        // Approval checks the balance for real; this only stops a request that can't be
        // approved from waiting in HR's queue.
        if year == today.year() {
            grant_in(&mut tx, year, today, Some(employee.id)).await?;
        }
        let Some((entitled, used)) =
            repo::balance(&mut tx, employee.id, leave_type.id, year).await?
        else {
            return Err(AppError::Validation(vec![field(
                "leaveTypeId",
                &format!("There's no {} balance for {year}", leave_type.name),
            )]));
        };
        let waiting = repo::pending_halfdays(&mut tx, employee.id, leave_type.id, year).await?;
        let left = entitled - used - waiting;
        if halfdays > left {
            return Err(AppError::Validation(vec![field(
                "leaveTypeId",
                &format!(
                    "{} has {} left, and this takes {}",
                    leave_type.name,
                    days(left.max(0)),
                    days(halfdays)
                ),
            )]));
        }
    }

    let id = repo::insert(
        &mut tx,
        &NewRequest {
            employee_id: employee.id,
            leave_type_id: leave_type.id,
            start_date: &from,
            end_date: &to,
            half_day: input.half_day,
            halfdays,
            reason,
            filed_by: actor.user_id,
        },
    )
    .await?;
    let created = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave request"))?;
    let entry = Entry {
        action: "leave.file",
        entity: Some(("leave_request", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

fn year_of(r: &LeaveRequest) -> Result<i32, AppError> {
    parse_date(&r.start_date)
        .map(|d| d.year())
        .ok_or(AppError::NotFound("Leave request"))
}

/// Approves or rejects a pending request. A rejection needs a note. HR can't decide their
/// own leave. Approving paid leave takes the days from the balance.
pub async fn decide(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    id: i64,
    approve: bool,
    note: Option<String>,
    now: DateTime<Utc>,
) -> Result<LeaveRequest, AppError> {
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
        .ok_or(AppError::NotFound("Leave request"))?;
    if before.status != "PENDING" {
        return Err(AppError::Conflict(DECIDED));
    }
    if own_employee == Some(before.employee_id) {
        return Err(AppError::Conflict(
            "Someone else needs to decide your own leave request.",
        ));
    }
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    if approve
        && !before.half_day
        && repo::first_worked_day(
            &mut tx,
            before.employee_id,
            &before.start_date,
            &before.end_date,
        )
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(WORKED));
    }
    if approve
        && !before.half_day
        && overtime_repo::first_live_in(
            &mut tx,
            before.employee_id,
            &before.start_date,
            &before.end_date,
        )
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(OVERTIME));
    }
    let stamp = time::to_db(now);
    let status = if approve { "APPROVED" } else { "REJECTED" };
    if !repo::set_status(
        &mut tx,
        id,
        "PENDING",
        status,
        actor.user_id,
        note.as_deref(),
        &stamp,
    )
    .await?
    {
        return Err(AppError::Conflict(DECIDED));
    }
    if approve
        && before.is_paid
        && !repo::use_balance(
            &mut tx,
            before.employee_id,
            before.leave_type_id,
            year_of(&before)?,
            before.halfdays,
            &stamp,
        )
        .await?
    {
        return Err(AppError::Conflict(NOT_ENOUGH));
    }
    let after = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave request"))?;
    let entry = Entry {
        action: if approve {
            "leave.approve"
        } else {
            "leave.reject"
        },
        entity: Some(("leave_request", id)),
        before: Some(json!({ "status": before.status })),
        after: Some(json!({
            "status": status,
            "note": note,
            "employeeNo": after.employee_no,
            "type": after.leave_type_code,
            "halfdays": after.halfdays,
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// The employee can cancel their own pending request. HR can also cancel an approved one,
/// which gives the days back, as long as payroll for those days isn't posted.
pub async fn cancel(
    db: &SqlitePool,
    actor: Actor<'_>,
    own_employee: Option<i64>,
    can_decide: bool,
    id: i64,
    now: DateTime<Utc>,
) -> Result<LeaveRequest, AppError> {
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave request"))?;
    let owner = own_employee == Some(before.employee_id);
    if !owner && !can_decide {
        // Someone else's request: don't even confirm it exists.
        return Err(AppError::NotFound("Leave request"));
    }
    let allowed = match before.status.as_str() {
        "PENDING" => true,
        "APPROVED" => can_decide,
        _ => false,
    };
    if !allowed {
        return Err(AppError::Conflict(if before.status == "APPROVED" {
            "This leave is already approved. Ask HR to cancel it."
        } else {
            "This request is already closed."
        }));
    }
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    let stamp = time::to_db(now);
    if before.status == "APPROVED" && before.is_paid {
        repo::restore_balance(
            &mut tx,
            before.employee_id,
            before.leave_type_id,
            year_of(&before)?,
            before.halfdays,
            &stamp,
        )
        .await?;
    }
    if !repo::set_status(
        &mut tx,
        id,
        &before.status,
        "CANCELLED",
        actor.user_id,
        None,
        &stamp,
    )
    .await?
    {
        return Err(AppError::Conflict(CHANGED));
    }
    let after = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave request"))?;
    let entry = Entry {
        action: "leave.cancel",
        entity: Some(("leave_request", id)),
        before: Some(json!({ "status": before.status })),
        after: Some(json!({
            "status": "CANCELLED",
            "employeeNo": after.employee_no,
            "type": after.leave_type_code,
            "halfdaysReturned": if before.status == "APPROVED" && before.is_paid {
                before.halfdays
            } else {
                0
            },
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// Works out again how long the pending and approved leave covering `date` is, after a
/// holiday on that date was added, moved or removed. Approved paid leave gives back or
/// takes the difference from the balance. Refuses (so the holiday change rolls back) when
/// a request would have no working days left, or approved leave would outgrow its balance.
pub(crate) async fn recount_on(
    conn: &mut SqliteConnection,
    actor: Actor<'_>,
    date: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let requests = repo::live_on(conn, date).await?;
    recount(conn, actor, requests, ("holidayDate", date), now).await
}

/// Counts each request's days again and fixes the request and its balance where the
/// count changed. `cause` names what changed in the audit log, like `("holidayDate", date)`.
pub(crate) async fn recount(
    conn: &mut SqliteConnection,
    actor: Actor<'_>,
    requests: Vec<repo::Recount>,
    cause: (&str, &str),
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let stamp = time::to_db(now);
    for r in requests {
        let (Some(start), Some(end)) = (parse_date(&r.start_date), parse_date(&r.end_date)) else {
            continue;
        };
        let days_off: HashSet<NaiveDate> = repo::days_off(conn, &r.start_date, &r.end_date)
            .await?
            .iter()
            .filter_map(|d| parse_date(d))
            .collect();
        let history = schedules::work_days_history(conn, r.employee_id).await?;
        let Some(halfdays) = count_halfdays_by_history(start, end, r.half_day, &history, &days_off)
        else {
            return Err(AppError::Conflict(NO_SCHEDULE));
        };
        if halfdays == r.halfdays {
            continue;
        }
        if halfdays == 0 {
            return Err(AppError::Conflict(NO_DAYS_LEFT));
        }
        if r.status == "APPROVED" && r.is_paid {
            let year = start.year();
            let more = halfdays - r.halfdays;
            if more > 0 {
                if !repo::use_balance(conn, r.employee_id, r.leave_type_id, year, more, &stamp)
                    .await?
                {
                    return Err(AppError::Conflict(LONGER_THAN_BALANCE));
                }
            } else {
                repo::restore_balance(conn, r.employee_id, r.leave_type_id, year, -more, &stamp)
                    .await?;
            }
        }
        repo::set_halfdays(conn, r.id, halfdays, &stamp).await?;
        let entry = Entry {
            action: "leave.recount",
            entity: Some(("leave_request", r.id)),
            before: Some(json!({ "halfdays": r.halfdays })),
            after: Some(json!({
                "halfdays": halfdays,
                "employeeNo": r.employee_no,
                cause.0: cause.1,
            })),
        };
        audit::record(conn, now, actor, entry).await?;
    }
    Ok(())
}

/// Cancels an employee's pending and approved leave after their last day of work, giving
/// approved paid days back. Leave that straddles the last day is refused instead, since
/// cutting it short is HR's call.
pub(crate) async fn cancel_after_separation(
    conn: &mut SqliteConnection,
    actor: Actor<'_>,
    employee_id: i64,
    last_day: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let stamp = time::to_db(now);
    for r in repo::live_after(conn, employee_id, last_day).await? {
        if r.start_date.as_str() <= last_day {
            return Err(AppError::Conflict(RUNS_PAST_SEPARATION));
        }
        if r.locked {
            return Err(AppError::Conflict(LOCKED));
        }
        if r.status == "APPROVED" && r.is_paid {
            repo::restore_balance(
                conn,
                r.employee_id,
                r.leave_type_id,
                year_of(&r)?,
                r.halfdays,
                &stamp,
            )
            .await?;
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
            action: "leave.cancel",
            entity: Some(("leave_request", r.id)),
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

    /// A Wednesday.
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

    /// Juan (1) hired 2020 with SIL; Leo (2) has no schedule; Old (3) is archived; Ana (4)
    /// hired this year, so no SIL yet. Friday Oct 16 is a regular holiday.
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
             (1, 'Office', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI'); \
             INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, schedule_id, archived_at) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2020-03-02', 'REGULAR', 1, NULL), \
             (2, 'EMP-2', 'Leo', 'Santos', '2020-03-02', 'REGULAR', NULL, NULL), \
             (3, 'EMP-3', 'Old', 'Hand', '2018-01-08', 'REGULAR', 1, '2026-01-01T00:00:00Z'), \
             (4, 'EMP-4', 'Ana', 'Reyes', '2026-02-02', 'PROBATIONARY', 1, NULL); \
             INSERT INTO holidays (date, name, type) VALUES \
             ('2026-10-16', 'Town Fiesta', 'REGULAR');",
        )
        .execute(&db)
        .await
        .expect("fixtures");
        (dir, db)
    }

    async fn type_id(db: &SqlitePool, code: &str) -> i64 {
        sqlx::query_scalar("SELECT id FROM leave_types WHERE code = ?")
            .bind(code)
            .fetch_one(db)
            .await
            .expect("type")
    }

    async fn leave(db: &SqlitePool, code: &str, start: &str, end: &str) -> LeaveRequestInput {
        LeaveRequestInput {
            employee_no: None,
            leave_type_id: type_id(db, code).await,
            start_date: start.into(),
            end_date: end.into(),
            half_day: false,
            reason: "Family trip".into(),
        }
    }

    async fn used(db: &SqlitePool, employee_id: i64, code: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT b.used_halfdays FROM leave_balances b JOIN leave_types t \
             ON t.id = b.leave_type_id WHERE b.employee_id = ? AND t.code = ? AND b.year = 2026",
        )
        .bind(employee_id)
        .bind(code)
        .fetch_one(db)
        .await
        .expect("balance")
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn message(e: AppError) -> String {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.message).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    async fn lock(db: &SqlitePool, employee_id: i64, date: &str) {
        sqlx::query(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source, \
             locked_by_period_id) VALUES (?, ?, ?, 'PRESENT', 'CLOCK', 1)",
        )
        .bind(employee_id)
        .bind(date)
        .bind(format!("{date}T08:00:00"))
        .execute(db)
        .await
        .expect("lock");
    }

    async fn clock_in(db: &SqlitePool, employee_id: i64, date: &str) {
        sqlx::query(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source) \
             VALUES (?, ?, ?, 'PRESENT', 'CLOCK')",
        )
        .bind(employee_id)
        .bind(date)
        .bind(format!("{date}T08:00:00"))
        .execute(db)
        .await
        .expect("clock in");
    }

    fn holiday(date: &str) -> crate::domain::org::HolidayInput {
        crate::domain::org::HolidayInput {
            date: date.into(),
            name: "Typhoon day".into(),
            kind: "SPECIAL_NON_WORKING".into(),
        }
    }

    async fn halfdays_of(db: &SqlitePool, id: i64) -> i64 {
        repo::by_id(db, id)
            .await
            .expect("read")
            .expect("row")
            .halfdays
    }

    #[tokio::test]
    async fn filing_counts_only_working_days() {
        let (_d, db) = db().await;
        // Mon Oct 12 to Sun Oct 18: Friday is a holiday, the weekend is off.
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-18").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        assert_eq!((r.halfdays, r.status.as_str()), (8, "PENDING"));
        assert_eq!(r.filed_by_username.as_deref(), Some("juan"));
        assert_eq!(audit_rows::actions(&db).await, ["leave.file"]);

        let mut half = leave(&db, "SL", "2026-10-20", "2026-10-20").await;
        half.half_day = true;
        let r = file(&db, juan(), Some(1), half, today(), t0())
            .await
            .expect("half day");
        assert_eq!(r.halfdays, 1);
    }

    #[tokio::test]
    async fn filing_checks_dates_and_reason() {
        let (_d, db) = db().await;
        let mut bad = leave(&db, "VL", "2026-10-14", "2026-10-12").await;
        bad.reason = "x".into();
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), bad, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["endDate", "reason"]
        );
        let across = leave(&db, "VL", "2026-12-28", "2027-01-04").await;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), across, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["endDate"]
        );
        let mut half = leave(&db, "VL", "2026-10-12", "2026-10-13").await;
        half.half_day = true;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), half, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["halfDay"]
        );
        let weekend = leave(&db, "VL", "2026-10-10", "2026-10-11").await;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), weekend, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["startDate"]
        );
        let far = leave(&db, "VL", "2027-11-02", "2027-11-02").await;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), far, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["startDate"]
        );
        let mut inactive = leave(&db, "VL", "2026-10-12", "2026-10-12").await;
        inactive.leave_type_id = 999;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), inactive, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["leaveTypeId"]
        );
    }

    #[tokio::test]
    async fn a_request_spans_at_most_60_days() {
        let (_d, db) = db().await;
        // Oct 1 to Nov 29 is 60 days; one more is refused.
        let longest = leave(&db, "LWOP", "2026-10-01", "2026-11-29").await;
        file(&db, juan(), Some(1), longest, today(), t0())
            .await
            .expect("60 days");
        let too_long = LeaveRequestInput {
            employee_no: Some("EMP-4".into()),
            ..leave(&db, "LWOP", "2026-10-01", "2026-11-30").await
        };
        let err = file(&db, hr(), None, too_long, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(message(err), "File at most 60 days at a time");
    }

    #[tokio::test]
    async fn leave_across_a_schedule_change_counts_each_day_by_its_schedule() {
        let (_d, db) = db().await;
        // From Mon Oct 12 Juan also works Saturdays. Fri Oct 16 is a holiday.
        sqlx::query(
            "INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days) VALUES \
             (2, 'Six days', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI,SAT'); \
             INSERT INTO schedule_assignments (employee_id, schedule_id, effective_from) \
             VALUES (1, 2, '2026-10-12')",
        )
        .execute(&db)
        .await
        .expect("move");
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-10", "2026-10-17").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        // Sat 10 was a rest day; Mon–Thu 12–15 and Sat 17 are work days.
        assert_eq!(r.halfdays, 10);
    }

    #[tokio::test]
    async fn a_switched_off_leave_type_cant_be_filed() {
        let (_d, db) = db().await;
        let vl = type_id(&db, "VL").await;
        crate::services::leave::set_type_active(&db, hr(), vl, false, t0())
            .await
            .expect("switch off");
        let input = leave(&db, "VL", "2026-10-12", "2026-10-12").await;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), input, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["leaveTypeId"]
        );
    }

    #[tokio::test]
    async fn filing_needs_a_current_employee_with_a_schedule() {
        let (_d, db) = db().await;
        let input = leave(&db, "LWOP", "2026-10-12", "2026-10-12").await;
        for (own, no) in [(None, None), (Some(2), None), (None, Some("EMP-3"))] {
            let i = LeaveRequestInput {
                employee_no: no.map(String::from),
                ..input.clone()
            };
            let err = file(&db, hr(), own, i, today(), t0()).await.unwrap_err();
            assert!(
                matches!(err, AppError::Conflict(_)),
                "{own:?} {no:?}: {err:?}"
            );
        }
        let unknown = LeaveRequestInput {
            employee_no: Some("EMP-9".into()),
            ..input.clone()
        };
        assert_eq!(
            fields(
                file(&db, hr(), None, unknown, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["employeeNo"]
        );
        // HR filing for Juan.
        let for_juan = LeaveRequestInput {
            employee_no: Some("EMP-1".into()),
            ..input
        };
        let r = file(&db, hr(), None, for_juan, today(), t0())
            .await
            .expect("for juan");
        assert_eq!(
            (r.employee_id, r.filed_by_username.as_deref()),
            (1, Some("hr"))
        );
    }

    #[tokio::test]
    async fn filing_refuses_overlaps_and_more_than_is_left() {
        let (_d, db) = db().await;
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-19", "2026-10-23").await,
            today(),
            t0(),
        )
        .await
        .expect("5 days of VL");
        let overlap = leave(&db, "SL", "2026-10-23", "2026-10-26").await;
        assert_eq!(
            fields(
                file(&db, juan(), Some(1), overlap, today(), t0())
                    .await
                    .unwrap_err()
            ),
            ["startDate"]
        );
        // The 5 pending days use up VL, so one more is refused before HR sees it.
        let more = leave(&db, "VL", "2026-10-26", "2026-10-26").await;
        let err = file(&db, juan(), Some(1), more, today(), t0())
            .await
            .unwrap_err();
        assert_eq!(
            message(err),
            "Vacation Leave has 0 days left, and this takes 1 day"
        );
        // Ana has served under a year, so she has no SIL.
        let sil = LeaveRequestInput {
            employee_no: Some("EMP-4".into()),
            ..leave(&db, "SIL", "2026-10-12", "2026-10-12").await
        };
        assert_eq!(
            message(file(&db, hr(), None, sil, today(), t0()).await.unwrap_err()),
            "There's no Service Incentive Leave balance for 2026"
        );
    }

    #[tokio::test]
    async fn approving_takes_the_days_and_cancelling_gives_them_back() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-13").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let ok = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .expect("approve");
        assert_eq!(
            (ok.status.as_str(), ok.decided_by_username.as_deref()),
            ("APPROVED", Some("hr"))
        );
        assert_eq!(used(&db, 1, "VL").await, 4);
        let again = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .unwrap_err();
        assert!(matches!(again, AppError::Conflict(_)));

        // Juan can't take back approved leave; HR can, and the days return.
        assert!(matches!(
            cancel(&db, juan(), Some(1), false, r.id, t0())
                .await
                .unwrap_err(),
            AppError::Conflict(_)
        ));
        let c = cancel(&db, hr(), None, true, r.id, t0())
            .await
            .expect("cancel");
        assert_eq!(c.status, "CANCELLED");
        assert_eq!(used(&db, 1, "VL").await, 0);
        let actions = audit_rows::actions(&db).await;
        assert!(
            actions.ends_with(&[
                "leave.file".into(),
                "leave.approve".into(),
                "leave.cancel".into()
            ]),
            "{actions:?}"
        );
    }

    #[tokio::test]
    async fn approval_checks_the_balance_again() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-12", "2026-10-16").await,
            today(),
            t0(),
        )
        .await
        .expect("4 days of SL");
        // HR lowers SL to 2 days after it was filed.
        sqlx::query("UPDATE leave_balances SET entitled_halfdays = 4 WHERE employee_id = 1 AND leave_type_id = ?")
            .bind(type_id(&db, "SL").await)
            .execute(&db)
            .await
            .expect("lower");
        let err = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(NOT_ENOUGH)));
        let still = repo::by_id(&db, r.id).await.expect("read").expect("row");
        assert_eq!(still.status, "PENDING");
        assert_eq!(used(&db, 1, "SL").await, 0);
    }

    #[tokio::test]
    async fn unpaid_leave_uses_no_balance() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "LWOP", "2026-10-12", "2026-11-30").await,
            today(),
            t0(),
        )
        .await
        .expect("long unpaid leave");
        decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .expect("approve");
        let total: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(used_halfdays), 0) FROM leave_balances")
                .fetch_one(&db)
                .await
                .expect("sum");
        assert_eq!(total, 0);
    }

    #[tokio::test]
    async fn deciding_needs_someone_else_and_a_note_to_reject() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-12").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        assert!(matches!(
            decide(&db, hr(), Some(1), r.id, true, None, t0())
                .await
                .unwrap_err(),
            AppError::Conflict(_)
        ));
        assert_eq!(
            fields(
                decide(&db, hr(), None, r.id, false, Some(" ".into()), t0())
                    .await
                    .unwrap_err()
            ),
            ["note"]
        );
        let no = decide(&db, hr(), None, r.id, false, Some("Busy week".into()), t0())
            .await
            .expect("reject");
        assert_eq!(
            (no.status.as_str(), no.decision_note.as_deref()),
            ("REJECTED", Some("Busy week"))
        );
        assert_eq!(used(&db, 1, "VL").await, 0);
        assert_eq!(
            audit_rows::actions(&db).await.last().map(String::as_str),
            Some("leave.reject")
        );
    }

    #[tokio::test]
    async fn staff_cancel_only_their_own_pending_leave() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-12").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let other = cancel(&db, juan(), Some(4), false, r.id, t0())
            .await
            .unwrap_err();
        assert!(matches!(other, AppError::NotFound(_)));
        let c = cancel(&db, juan(), Some(1), false, r.id, t0())
            .await
            .expect("cancel");
        assert_eq!(c.status, "CANCELLED");
        assert!(matches!(
            cancel(&db, juan(), Some(1), false, r.id, t0())
                .await
                .unwrap_err(),
            AppError::Conflict(_)
        ));
    }

    #[tokio::test]
    async fn posted_payroll_days_cant_change() {
        let (_d, db) = db().await;
        let pending = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-13").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let approved = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-14", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        decide(&db, hr(), None, approved.id, true, None, t0())
            .await
            .expect("approve");
        lock(&db, 1, "2026-10-13").await;
        lock(&db, 1, "2026-10-14").await;
        lock(&db, 1, "2026-10-05").await;

        let locked = repo::by_id(&db, pending.id)
            .await
            .expect("read")
            .expect("row");
        assert!(locked.locked);
        assert!(matches!(
            decide(&db, hr(), None, pending.id, true, None, t0())
                .await
                .unwrap_err(),
            AppError::Conflict(LOCKED)
        ));
        assert!(matches!(
            cancel(&db, hr(), None, true, approved.id, t0())
                .await
                .unwrap_err(),
            AppError::Conflict(LOCKED)
        ));
        let new = leave(&db, "VL", "2026-10-05", "2026-10-06").await;
        assert!(matches!(
            file(&db, juan(), Some(1), new, today(), t0())
                .await
                .unwrap_err(),
            AppError::Conflict(LOCKED)
        ));
    }

    #[tokio::test]
    async fn a_posted_period_locks_its_days_even_without_attendance() {
        let (_d, db) = db().await;
        let pending = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-13").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let approved = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-14", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        decide(&db, hr(), None, approved.id, true, None, t0())
            .await
            .expect("approve");
        posted(&db, "2026-10-01", "2026-10-15").await;

        let read = repo::by_id(&db, pending.id).await.expect("read");
        assert!(read.expect("row").locked);
        let r = decide(&db, hr(), None, pending.id, true, None, t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
        let r = cancel(&db, hr(), None, true, approved.id, t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
        let new = leave(&db, "VL", "2026-10-15", "2026-10-16").await;
        let r = file(&db, juan(), Some(1), new, today(), t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
        // Days after the period are still open.
        let after = leave(&db, "VL", "2026-10-19", "2026-10-19").await;
        file(&db, juan(), Some(1), after, today(), t0())
            .await
            .expect("after the period");
    }

    #[tokio::test]
    async fn lists_find_requests_that_touch_the_range() {
        let (_d, db) = db().await;
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-29", "2026-11-03").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let oct = DateRange {
            from: "2026-10-01".into(),
            to: "2026-10-31".into(),
        };
        let nov = DateRange {
            from: "2026-11-01".into(),
            to: "2026-11-30".into(),
        };
        assert_eq!(mine(&db, 1, 2026).await.expect("mine").len(), 1);
        assert!(mine(&db, 1, 2027).await.expect("next year").is_empty());
        assert_eq!(
            list(&db, Some("PENDING"), &nov).await.expect("list").len(),
            1
        );
        assert!(list(&db, Some("APPROVED"), &nov)
            .await
            .expect("list")
            .is_empty());
        assert_eq!(
            fields(list(&db, Some("NOPE"), &nov).await.unwrap_err()),
            ["status"]
        );
        assert_eq!(pending(&db).await.expect("pending").len(), 1);
        assert!(mine(&db, 4, 2026).await.expect("ana").is_empty());
        assert_eq!(list(&db, None, &oct).await.expect("october").len(), 1);
    }

    #[tokio::test]
    async fn two_approvals_at_the_same_moment_decide_once() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-13").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let (a, b) = tokio::join!(
            decide(&db, hr(), None, r.id, true, None, t0()),
            decide(&db, hr(), None, r.id, true, None, t0()),
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
        assert_eq!(used(&db, 1, "VL").await, 4);
        let approvals = audit_rows::actions(&db).await;
        assert_eq!(
            approvals.iter().filter(|a| *a == "leave.approve").count(),
            1
        );
    }

    #[tokio::test]
    async fn two_approvals_at_the_same_moment_cant_overspend_a_balance() {
        let (_d, db) = db().await;
        let a = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .expect("3 days");
        let b = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-19", "2026-10-20").await,
            today(),
            t0(),
        )
        .await
        .expect("2 days");
        // HR lowers VL to 3 days, so only the first approval fits.
        sqlx::query("UPDATE leave_balances SET entitled_halfdays = 6 WHERE employee_id = 1 AND leave_type_id = ?")
            .bind(type_id(&db, "VL").await)
            .execute(&db)
            .await
            .expect("lower");
        let (ra, rb) = tokio::join!(
            decide(&db, hr(), None, a.id, true, None, t0()),
            decide(&db, hr(), None, b.id, true, None, t0()),
        );
        assert!(ra.is_ok(), "{ra:?}");
        assert!(matches!(rb, Err(AppError::Conflict(NOT_ENOUGH))), "{rb:?}");
        assert_eq!(used(&db, 1, "VL").await, 6);
        let still = repo::by_id(&db, b.id).await.expect("read").expect("row");
        assert_eq!(still.status, "PENDING");
    }

    #[tokio::test]
    async fn a_status_change_only_lands_on_the_status_it_expects() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-12").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let mut conn = db.acquire().await.expect("conn");
        let stale = repo::set_status(
            &mut conn,
            r.id,
            "APPROVED",
            "CANCELLED",
            Some(1),
            None,
            "now",
        )
        .await
        .expect("update");
        assert!(
            !stale,
            "a request that isn't approved can't be cancelled as approved"
        );
        drop(conn);
        let row = repo::by_id(&db, r.id).await.expect("read").expect("row");
        assert_eq!(row.status, "PENDING");
    }

    #[tokio::test]
    async fn leave_cant_cover_a_day_the_person_worked() {
        let (_d, db) = db().await;
        clock_in(&db, 1, "2026-10-13").await;
        let err = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            message(err),
            "There's a time in on 2026-10-13, so that day can't be leave. HR can correct it first."
        );
        // A half day off is fine on a day they came in.
        let mut half = leave(&db, "VL", "2026-10-13", "2026-10-13").await;
        half.half_day = true;
        file(&db, juan(), Some(1), half, today(), t0())
            .await
            .expect("half day");
    }

    #[tokio::test]
    async fn approval_is_refused_if_they_clocked_in_after_filing() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        clock_in(&db, 1, "2026-10-14").await;
        let err = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(WORKED)), "{err:?}");
        assert_eq!(used(&db, 1, "VL").await, 0);
        // Rejecting it still works.
        decide(
            &db,
            hr(),
            None,
            r.id,
            false,
            Some("You came in".into()),
            t0(),
        )
        .await
        .expect("reject");
    }

    #[tokio::test]
    async fn a_new_holiday_inside_approved_leave_gives_the_day_back() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-19", "2026-10-23").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .expect("approve");
        assert_eq!(used(&db, 1, "VL").await, 10);

        let h = crate::services::holidays::create(&db, hr(), holiday("2026-10-21"), t0())
            .await
            .expect("add holiday");
        assert_eq!(
            (halfdays_of(&db, r.id).await, used(&db, 1, "VL").await),
            (8, 8)
        );
        assert_eq!(
            audit_rows::actions(&db).await.last().map(String::as_str),
            Some("holiday.create")
        );
        assert!(audit_rows::actions(&db)
            .await
            .contains(&"leave.recount".to_string()));

        // Moving the holiday out of the leave takes the day again.
        let moved = crate::domain::org::HolidayInput {
            date: "2026-10-26".into(),
            ..holiday("")
        };
        crate::services::holidays::update(&db, hr(), h.id, moved, t0())
            .await
            .expect("move");
        assert_eq!(
            (halfdays_of(&db, r.id).await, used(&db, 1, "VL").await),
            (10, 10)
        );
    }

    #[tokio::test]
    async fn pending_leave_is_recounted_without_touching_the_balance() {
        let (_d, db) = db().await;
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-19", "2026-10-23").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let h = crate::services::holidays::create(&db, hr(), holiday("2026-10-22"), t0())
            .await
            .expect("add holiday");
        assert_eq!(halfdays_of(&db, r.id).await, 8);
        crate::services::holidays::delete(&db, hr(), h.id, t0())
            .await
            .expect("delete");
        assert_eq!(halfdays_of(&db, r.id).await, 10);
        assert_eq!(used(&db, 1, "SL").await, 0);
    }

    #[tokio::test]
    async fn a_holiday_change_that_breaks_leave_is_refused() {
        let (_d, db) = db().await;
        // One-day leave on Thursday: a holiday that day would leave nothing to take.
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-22", "2026-10-22").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let err = crate::services::holidays::create(&db, hr(), holiday("2026-10-22"), t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(NO_DAYS_LEFT)), "{err:?}");
        assert!(crate::services::holidays::list(&db, 2026)
            .await
            .expect("list")
            .iter()
            .all(|h| h.date != "2026-10-22"));

        // Approved SL with the balance used up: removing a holiday inside it can't add a day.
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-26", "2026-10-30").await,
            today(),
            t0(),
        )
        .await
        .expect("file");
        let h = crate::services::holidays::create(&db, hr(), holiday("2026-10-28"), t0())
            .await
            .expect("add holiday");
        decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .expect("approve 4 days");
        sqlx::query("UPDATE leave_balances SET entitled_halfdays = 8 WHERE employee_id = 1 AND leave_type_id = ?")
            .bind(type_id(&db, "SL").await)
            .execute(&db)
            .await
            .expect("lower");
        let err = crate::services::holidays::delete(&db, hr(), h.id, t0())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(LONGER_THAN_BALANCE)),
            "{err:?}"
        );
        assert_eq!(
            (halfdays_of(&db, r.id).await, used(&db, 1, "SL").await),
            (8, 8)
        );
    }

    fn overtime(date: &str) -> crate::domain::overtime::OvertimeInput {
        crate::domain::overtime::OvertimeInput {
            employee_no: None,
            work_date: date.into(),
            start_time: "17:00".into(),
            end_time: "20:00".into(),
            reason: "Stock count".into(),
        }
    }

    #[tokio::test]
    async fn leave_and_overtime_dont_mix() {
        let (_d, db) = db().await;
        crate::services::overtime::file(
            &db,
            juan(),
            Some(1),
            overtime("2026-10-13"),
            today(),
            t0(),
        )
        .await
        .expect("overtime");
        let err = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-12", "2026-10-14").await,
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            message(err),
            "There's overtime filed for 2026-10-13. Cancel it first, or file a half day."
        );
        let mut half = leave(&db, "VL", "2026-10-13", "2026-10-13").await;
        half.half_day = true;
        file(&db, juan(), Some(1), half, today(), t0())
            .await
            .expect("half day beside overtime");

        // The other way round: pending leave blocks new overtime.
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-19", "2026-10-20").await,
            today(),
            t0(),
        )
        .await
        .expect("leave");
        let err = crate::services::overtime::file(
            &db,
            juan(),
            Some(1),
            overtime("2026-10-19"),
            today(),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["workDate"]);

        // Overtime that slipped in anyway stops the approval.
        let r = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-26", "2026-10-27").await,
            today(),
            t0(),
        )
        .await
        .expect("leave");
        sqlx::query(
            "INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, minutes, reason) \
             VALUES (1, '2026-10-26', '2026-10-26T17:00:00', '2026-10-26T19:00:00', 120, 'Rush')",
        )
        .execute(&db)
        .await
        .expect("overtime");
        let err = decide(&db, hr(), None, r.id, true, None, t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(OVERTIME)), "{err:?}");
    }

    #[tokio::test]
    async fn leave_stays_within_the_employment_dates() {
        let (_d, db) = db().await;
        // Ana was hired on 2026-02-02.
        let early = LeaveRequestInput {
            employee_no: Some("EMP-4".into()),
            ..leave(&db, "LWOP", "2026-01-30", "2026-01-30").await
        };
        assert_eq!(
            message(
                file(&db, hr(), None, early, today(), t0())
                    .await
                    .unwrap_err()
            ),
            "This is before the hire date, 2026-02-02"
        );
        sqlx::query("UPDATE employees SET separation_date = '2026-10-30' WHERE id = 1")
            .execute(&db)
            .await
            .expect("resign");
        let late = leave(&db, "VL", "2026-11-02", "2026-11-03").await;
        assert_eq!(
            message(
                file(&db, juan(), Some(1), late, today(), t0())
                    .await
                    .unwrap_err()
            ),
            "This is after the last day of work, 2026-10-30"
        );
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-10-29", "2026-10-30").await,
            today(),
            t0(),
        )
        .await
        .expect("up to the last day");
    }

    #[tokio::test]
    async fn a_separation_cancels_leave_after_the_last_day_and_returns_the_days() {
        let (_d, db) = db().await;
        let pending = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-11-02", "2026-11-03").await,
            today(),
            t0(),
        )
        .await
        .expect("pending");
        let approved = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-11-09", "2026-11-10").await,
            today(),
            t0(),
        )
        .await
        .expect("approved");
        decide(&db, hr(), None, approved.id, true, None, t0())
            .await
            .expect("approve");
        let before = file(
            &db,
            juan(),
            Some(1),
            leave(&db, "SL", "2026-10-19", "2026-10-20").await,
            today(),
            t0(),
        )
        .await
        .expect("before the last day");
        assert_eq!(used(&db, 1, "VL").await, 4);

        let mut conn = db.acquire().await.expect("conn");
        cancel_after_separation(&mut conn, hr(), 1, "2026-10-30", t0())
            .await
            .expect("cancel");
        drop(conn);
        for id in [pending.id, approved.id] {
            assert_eq!(
                repo::by_id(&db, id)
                    .await
                    .expect("read")
                    .expect("row")
                    .status,
                "CANCELLED"
            );
        }
        assert_eq!(
            repo::by_id(&db, before.id)
                .await
                .expect("read")
                .expect("row")
                .status,
            "PENDING"
        );
        assert_eq!(used(&db, 1, "VL").await, 0);

        // Leave across the last day is HR's call, so the separation is refused.
        file(
            &db,
            juan(),
            Some(1),
            leave(&db, "VL", "2026-11-26", "2026-12-01").await,
            today(),
            t0(),
        )
        .await
        .expect("straddling");
        let mut conn = db.acquire().await.expect("conn");
        let err = cancel_after_separation(&mut conn, hr(), 1, "2026-11-27", t0())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(RUNS_PAST_SEPARATION)),
            "{err:?}"
        );
    }

    #[test]
    fn days_reads_like_a_person_wrote_it() {
        assert_eq!(days(0), "0 days");
        assert_eq!(days(1), "½ day");
        assert_eq!(days(2), "1 day");
        assert_eq!(days(3), "1½ days");
        assert_eq!(days(10), "5 days");
    }
}
