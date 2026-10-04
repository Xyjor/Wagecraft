//! Moving an employee to another work schedule from a date. Days before that date keep the
//! schedule they had, so attendance, leave, overtime and payroll for them never change.

use crate::audit::{self, Actor, Entry};
use crate::domain::schedule::{ScheduleAssignment, ScheduleChangeInput};
use crate::error::AppError;
use crate::repositories::employees;
use crate::repositories::leave_requests as leave_repo;
use crate::repositories::schedules::{self as repo, NewAssignment};
use crate::services::auth::field;
use crate::services::{leave_requests, overtime};
use crate::time;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sqlx::SqlitePool;

const REASON_MAX: usize = 200;

/// Oldest first.
pub async fn history(
    db: &SqlitePool,
    employee_id: i64,
) -> Result<Vec<ScheduleAssignment>, AppError> {
    let mut conn = db.acquire().await?;
    if employees::get(&mut *conn, employee_id).await?.is_none() {
        return Err(AppError::NotFound("Employee"));
    }
    Ok(repo::history(&mut conn, employee_id).await?)
}

/// Moves the employee to `input.schedule_id` from `input.effective_from`. A move on a date
/// that already has one replaces it. Leave from that date on is counted again with the new
/// work days, and overtime that would land inside the new hours blocks the move.
pub async fn change(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    input: ScheduleChangeInput,
    now: DateTime<Utc>,
) -> Result<Vec<ScheduleAssignment>, AppError> {
    let mut tx = db.begin().await?;
    let employee = employees::get(&mut *tx, employee_id)
        .await?
        .filter(|e| e.archived_at.is_none())
        .ok_or(AppError::NotFound("Employee"))?;
    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    let mut errors = Vec::new();
    match employees::schedule_active(&mut tx, input.schedule_id).await? {
        Some(true) => {}
        Some(false) => errors.push(field("scheduleId", "This schedule is switched off")),
        None => errors.push(field("scheduleId", "Pick a schedule")),
    }
    let last_recorded = repo::last_recorded_day(&mut tx, employee_id).await?;
    let from = parse_date(&input.effective_from);
    match from {
        None => errors.push(field("effectiveFrom", "Enter a date")),
        Some(from) => {
            if let Some(problem) = date_problem(
                from,
                &employee.hire_date,
                &employee.separation_date,
                last_recorded.as_deref(),
            ) {
                errors.push(field("effectiveFrom", &problem));
            }
        }
    }
    if reason.is_some_and(|r| r.chars().count() > REASON_MAX) {
        errors.push(field("reason", "Keep the reason to 200 characters"));
    }
    let Some(from) = from.filter(|_| errors.is_empty()) else {
        return Err(AppError::Validation(errors));
    };
    let from_text = from.to_string();

    let before = repo::history(&mut tx, employee_id).await?;
    repo::put(
        &mut tx,
        &NewAssignment {
            employee_id,
            schedule_id: input.schedule_id,
            effective_from: &from_text,
            reason,
            created_by: actor.user_id,
            created_at: &time::to_db(now),
        },
    )
    .await?;
    if let Some(day) = overtime::first_clash_from(&mut tx, employee_id, from).await? {
        return Err(AppError::Validation(vec![field(
            "effectiveFrom",
            &format!(
                "Overtime filed for {} would fall inside the new hours. Cancel or reject it \
                 first.",
                show(day)
            ),
        )]));
    }
    let leave = leave_repo::live_through(&mut tx, employee_id, &from_text).await?;
    leave_requests::recount(&mut tx, actor, leave, ("scheduleFrom", &from_text), now).await?;
    let after = repo::history(&mut tx, employee_id).await?;
    let entry = Entry {
        action: "employee.schedule_change",
        entity: Some(("employee", employee_id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
}

fn show(d: NaiveDate) -> String {
    d.format("%b %-d, %Y").to_string()
}

/// Why the move can't start on `from`, if it can't.
fn date_problem(
    from: NaiveDate,
    hire_date: &str,
    separation_date: &Option<String>,
    last_recorded: Option<&str>,
) -> Option<String> {
    if parse_date(hire_date).is_some_and(|hired| from < hired) {
        return Some("The move can't start before the hire date".into());
    }
    if let Some(last) = separation_date.as_deref().and_then(parse_date) {
        if from > last {
            return Some(format!("The employee's last day is {}", show(last)));
        }
    }
    if let Some(day) = last_recorded.and_then(parse_date) {
        if from <= day {
            return Some(format!(
                "Attendance is already recorded up to {}. The move has to start after that.",
                show(day)
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support::actions;
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

    /// Juan (1), hired Jan 6, 2025, works the office schedule (1, 08:00–17:00 Mon–Fri).
    /// Schedule 2 is 09:00–18:00 Mon–Sat; 3 is switched off.
    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("hr");
        drop(conn);
        sqlx::query(
            "INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days, is_active) VALUES \
             (1, 'Office', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI', 1), \
             (2, 'Six days', '09:00', '18:00', 60, 10, 'MON,TUE,WED,THU,FRI,SAT', 1), \
             (3, 'Old', '06:00', '15:00', 60, 10, 'MON,TUE,WED,THU,FRI', 0); \
             INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, schedule_id, separation_date) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2025-01-06', 'REGULAR', 1, NULL), \
             (2, 'EMP-2', 'Kim', 'Tan', '2025-01-06', 'RESIGNED', 1, '2026-10-30')",
        )
        .execute(&db)
        .await
        .expect("fixture");
        (dir, db)
    }

    fn to(schedule_id: i64, from: &str) -> ScheduleChangeInput {
        ScheduleChangeInput {
            schedule_id,
            effective_from: from.into(),
            reason: Some("Moved to the store".into()),
        }
    }

    fn field_errors(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn rows(h: &[ScheduleAssignment]) -> Vec<(&str, Option<&str>)> {
        h.iter()
            .map(|a| (a.effective_from.as_str(), a.schedule_name.as_deref()))
            .collect()
    }

    #[tokio::test]
    async fn a_move_starts_on_its_date_and_keeps_the_old_schedule_before_it() {
        let (_d, db) = db().await;
        let h = change(&db, hr(), 1, to(2, "2026-10-12"), now())
            .await
            .expect("moved");
        assert_eq!(
            rows(&h),
            [
                ("2025-01-06", Some("Office")),
                ("2026-10-12", Some("Six days"))
            ]
        );
        assert_eq!(h[1].reason.as_deref(), Some("Moved to the store"));
        assert_eq!(h[1].created_by_name.as_deref(), Some("hr"));
        assert_eq!(rows(&history(&db, 1).await.expect("history")), rows(&h));
        assert_eq!(actions(&db).await, ["employee.schedule_change"]);
    }

    #[tokio::test]
    async fn a_second_move_on_the_same_date_replaces_the_first() {
        let (_d, db) = db().await;
        change(&db, hr(), 1, to(2, "2026-10-12"), now())
            .await
            .expect("first");
        let h = change(&db, hr(), 1, to(1, "2026-10-12"), now())
            .await
            .expect("fixed");
        assert_eq!(
            rows(&h),
            [
                ("2025-01-06", Some("Office")),
                ("2026-10-12", Some("Office"))
            ]
        );
    }

    #[tokio::test]
    async fn the_date_falls_within_employment_and_the_schedule_is_in_use() {
        let (_d, db) = db().await;
        let early = change(&db, hr(), 1, to(2, "2024-12-31"), now())
            .await
            .unwrap_err();
        assert_eq!(field_errors(early)[0].0, "effectiveFrom");
        let late = change(&db, hr(), 2, to(2, "2026-10-31"), now())
            .await
            .unwrap_err();
        assert_eq!(field_errors(late)[0].0, "effectiveFrom");
        let off = change(&db, hr(), 1, to(3, "2026-10-12"), now())
            .await
            .unwrap_err();
        assert_eq!(field_errors(off)[0].0, "scheduleId");
        let missing = change(&db, hr(), 1, to(99, "2026-10-12"), now())
            .await
            .unwrap_err();
        assert_eq!(field_errors(missing)[0].0, "scheduleId");
        let nobody = change(&db, hr(), 99, to(2, "2026-10-12"), now())
            .await
            .unwrap_err();
        assert!(matches!(nobody, AppError::NotFound(_)), "{nobody:?}");
    }

    /// Recorded days were worked out with the old hours. Moving them would change late
    /// minutes and pay after the fact, so the move has to start after the last one.
    #[tokio::test]
    async fn a_move_cant_reach_back_over_recorded_days() {
        let (_d, db) = db().await;
        sqlx::query(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source) \
             VALUES (1, '2026-10-07', '2026-10-07T08:00:00', 'PRESENT', 'CLOCK')",
        )
        .execute(&db)
        .await
        .expect("record");
        let err = change(&db, hr(), 1, to(2, "2026-10-07"), now())
            .await
            .unwrap_err();
        let errors = field_errors(err);
        assert_eq!(errors[0].0, "effectiveFrom");
        assert!(errors[0].1.contains("Oct 7, 2026"), "{errors:?}");
        change(&db, hr(), 1, to(2, "2026-10-08"), now())
            .await
            .expect("the day after is fine");
    }

    #[tokio::test]
    async fn leave_after_the_date_is_counted_again() {
        let (_d, db) = db().await;
        // Mon Oct 12 to Sat Oct 17 on Mon–Fri is 5 days; on Mon–Sat it's 6.
        sqlx::query(
            "INSERT INTO leave_balances (employee_id, leave_type_id, year, entitled_halfdays, \
             used_halfdays) SELECT 1, id, 2026, 20, 10 FROM leave_types WHERE code = 'VL'; \
             INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             halfdays, reason, status) SELECT 1, id, '2026-10-12', '2026-10-17', 10, 'Trip', \
             'APPROVED' FROM leave_types WHERE code = 'VL'",
        )
        .execute(&db)
        .await
        .expect("leave");
        change(&db, hr(), 1, to(2, "2026-10-12"), now())
            .await
            .expect("moved");
        let (halfdays, used): (i64, i64) = sqlx::query_as(
            "SELECT r.halfdays, b.used_halfdays FROM leave_requests r JOIN leave_balances b \
             ON b.employee_id = r.employee_id AND b.leave_type_id = r.leave_type_id",
        )
        .fetch_one(&db)
        .await
        .expect("read");
        assert_eq!((halfdays, used), (12, 12));
        assert_eq!(
            actions(&db).await,
            ["leave.recount", "employee.schedule_change"]
        );
    }

    /// Overtime is only paid outside the schedule. Overtime already filed for 17:00–19:00
    /// would land inside 09:00–18:00, so HR has to settle it first.
    #[tokio::test]
    async fn overtime_inside_the_new_hours_blocks_the_move() {
        let (_d, db) = db().await;
        sqlx::query(
            "INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, minutes, \
             reason, status) VALUES (1, '2026-10-13', '2026-10-13T17:00:00', \
             '2026-10-13T19:00:00', 120, 'Stock count', 'APPROVED')",
        )
        .execute(&db)
        .await
        .expect("overtime");
        let err = change(&db, hr(), 1, to(2, "2026-10-12"), now())
            .await
            .unwrap_err();
        let errors = field_errors(err);
        assert_eq!(errors[0].0, "effectiveFrom");
        assert!(errors[0].1.contains("Oct 13, 2026"), "{errors:?}");
        assert_eq!(
            rows(&history(&db, 1).await.expect("history")).len(),
            1,
            "nothing saved"
        );
    }

    #[tokio::test]
    async fn the_schedules_list_counts_who_is_on_each_one_today() {
        let (_d, db) = db().await;
        change(&db, hr(), 1, to(2, "2025-02-03"), now())
            .await
            .expect("moved");
        let counts: Vec<(String, i64)> = crate::services::org::list_schedules(&db)
            .await
            .expect("list")
            .into_iter()
            .map(|s| (s.name, s.employee_count))
            .collect();
        assert_eq!(
            counts,
            [
                ("Office".into(), 1),
                ("Old".into(), 0),
                ("Six days".into(), 1)
            ]
        );
    }
}
