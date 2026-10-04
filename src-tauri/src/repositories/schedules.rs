//! Which work schedule an employee followed on a date (`schedule_assignments`).

use crate::domain::schedule::{ScheduleAssignment, ScheduleHistory};
use chrono::NaiveDate;
use sqlx::SqliteConnection;

/// Joins `work_schedules s` to the schedule employee `e` followed on the date bound at
/// `param` (for example `"?"` or `"?1"`). `s` is all NULL when they had none.
pub fn join_on(param: &str) -> String {
    format!(
        "LEFT JOIN work_schedules s ON s.id = (SELECT a.schedule_id FROM schedule_assignments a \
         WHERE a.employee_id = e.id AND a.effective_from <= {param} \
         ORDER BY a.effective_from DESC LIMIT 1)"
    )
}

/// Every work-day pattern the employee has followed, by start date.
pub async fn work_days_history(
    conn: &mut SqliteConnection,
    employee_id: i64,
) -> sqlx::Result<ScheduleHistory<String>> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT a.effective_from, s.work_days FROM schedule_assignments a \
         LEFT JOIN work_schedules s ON s.id = a.schedule_id WHERE a.employee_id = ?",
    )
    .bind(employee_id)
    .fetch_all(conn)
    .await?;
    Ok(ScheduleHistory::new(
        rows.into_iter()
            .filter_map(|(from, days)| {
                Some((NaiveDate::parse_from_str(&from, "%Y-%m-%d").ok()?, days))
            })
            .collect(),
    ))
}

/// The employee's schedule history, oldest first.
pub async fn history(
    conn: &mut SqliteConnection,
    employee_id: i64,
) -> sqlx::Result<Vec<ScheduleAssignment>> {
    sqlx::query_as(
        "SELECT a.id, a.schedule_id, s.name AS schedule_name, a.effective_from, a.reason, \
         u.username AS created_by_name FROM schedule_assignments a \
         LEFT JOIN work_schedules s ON s.id = a.schedule_id \
         LEFT JOIN users u ON u.id = a.created_by \
         WHERE a.employee_id = ? ORDER BY a.effective_from",
    )
    .bind(employee_id)
    .fetch_all(conn)
    .await
}

pub struct NewAssignment<'a> {
    pub employee_id: i64,
    pub schedule_id: i64,
    pub effective_from: &'a str,
    pub reason: Option<&'a str>,
    pub created_by: Option<i64>,
    pub created_at: &'a str,
}

/// Adds the assignment, or replaces the one already starting on that date.
pub async fn put(conn: &mut SqliteConnection, a: &NewAssignment<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO schedule_assignments \
         (employee_id, schedule_id, effective_from, reason, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT (employee_id, effective_from) DO UPDATE SET \
         schedule_id = excluded.schedule_id, reason = excluded.reason, \
         created_by = excluded.created_by, created_at = excluded.created_at",
    )
    .bind(a.employee_id)
    .bind(a.schedule_id)
    .bind(a.effective_from)
    .bind(a.reason)
    .bind(a.created_by)
    .bind(a.created_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The latest day the employee has an attendance record for.
pub async fn last_recorded_day(
    conn: &mut SqliteConnection,
    employee_id: i64,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT MAX(work_date) FROM attendance_records WHERE employee_id = ?")
        .bind(employee_id)
        .fetch_one(conn)
        .await
}

/// The dates the employee's schedules start on, oldest first.
pub async fn starts(conn: &mut SqliteConnection, employee_id: i64) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT effective_from FROM schedule_assignments WHERE employee_id = ? \
         ORDER BY effective_from",
    )
    .bind(employee_id)
    .fetch_all(conn)
    .await
}

pub async fn move_start(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE schedule_assignments SET effective_from = ? \
         WHERE employee_id = ? AND effective_from = ?",
    )
    .bind(to)
    .bind(employee_id)
    .bind(from)
    .execute(conn)
    .await?;
    Ok(())
}
