//! Which work schedule an employee followed on a date (`schedule_assignments`).

use crate::domain::schedule::ScheduleHistory;
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
