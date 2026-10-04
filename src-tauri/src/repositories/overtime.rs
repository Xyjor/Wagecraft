//! SQL for overtime requests. No rules here: the overtime service decides.

use super::schedules;
use crate::domain::overtime::OvertimeRequest;
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const REQUEST: &str = "SELECT o.id, o.employee_id, e.employee_no, \
    e.last_name || ', ' || e.first_name AS employee_name, o.work_date, o.start_at, o.end_at, \
    o.minutes, o.reason, o.status, f.username AS filed_by_username, \
    d.username AS decided_by_username, o.decided_at, o.decision_note, \
    EXISTS (SELECT 1 FROM attendance_records a WHERE a.employee_id = o.employee_id \
            AND a.work_date = o.work_date AND a.locked_by_period_id IS NOT NULL) AS locked \
    FROM overtime_requests o \
    JOIN employees e ON e.id = o.employee_id \
    LEFT JOIN users f ON f.id = o.filed_by \
    LEFT JOIN users d ON d.id = o.decided_by";

pub async fn by_id<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<OvertimeRequest>> {
    sqlx::query_as(&format!("{REQUEST} WHERE o.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// One employee's requests between two work dates, newest first.
pub async fn for_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<OvertimeRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE o.employee_id = ? AND o.work_date BETWEEN ? AND ? \
         ORDER BY o.work_date DESC, o.start_at DESC"
    ))
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

/// Everyone's requests between two work dates, optionally with one status, oldest first so
/// HR works through the queue in order.
pub async fn list<'e>(
    db: impl SqliteExecutor<'e>,
    status: Option<&str>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<OvertimeRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE o.work_date BETWEEN ?1 AND ?2 AND (?3 IS NULL OR o.status = ?3) \
         ORDER BY o.work_date, o.start_at, employee_name"
    ))
    .bind(from)
    .bind(to)
    .bind(status)
    .fetch_all(db)
    .await
}

/// Every pending request, whatever its date, oldest first.
pub async fn pending<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<OvertimeRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE o.status = 'PENDING' ORDER BY o.work_date, o.start_at, employee_name"
    ))
    .fetch_all(db)
    .await
}

/// The employee and schedule facts overtime needs. Never sent to the UI.
#[derive(Debug, FromRow)]
pub struct OvertimeEmployee {
    pub id: i64,
    pub archived_at: Option<String>,
    pub hire_date: String,
    pub separation_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub break_minutes: Option<i64>,
    pub grace_minutes: Option<i64>,
    pub work_days: Option<String>,
}

/// The employee with their work schedule on the date bound first.
fn employee_query(filter: &str) -> String {
    format!(
        "SELECT e.id, e.archived_at, e.hire_date, e.separation_date, s.start_time, s.end_time, \
         s.break_minutes, s.grace_minutes, s.work_days FROM employees e {} WHERE {filter}",
        schedules::join_on("?")
    )
}

pub async fn employee_by_id(
    conn: &mut SqliteConnection,
    id: i64,
    work_date: &str,
) -> sqlx::Result<Option<OvertimeEmployee>> {
    sqlx::query_as(&employee_query("e.id = ?"))
        .bind(work_date)
        .bind(id)
        .fetch_optional(conn)
        .await
}

pub async fn employee_by_no(
    conn: &mut SqliteConnection,
    employee_no: &str,
    work_date: &str,
) -> sqlx::Result<Option<OvertimeEmployee>> {
    sqlx::query_as(&employee_query("e.employee_no = ?"))
        .bind(work_date)
        .bind(employee_no)
        .fetch_optional(conn)
        .await
}

/// Whether the employee's attendance on `work_date` is in a posted payroll period.
pub async fn day_locked(
    conn: &mut SqliteConnection,
    employee_id: i64,
    work_date: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM attendance_records WHERE employee_id = ? \
         AND work_date = ? AND locked_by_period_id IS NOT NULL)",
    )
    .bind(employee_id)
    .bind(work_date)
    .fetch_one(conn)
    .await
}

/// Whether a pending or approved request of the employee's overlaps `[start_at, end_at)`.
pub async fn overlaps_active(
    conn: &mut SqliteConnection,
    employee_id: i64,
    start_at: &str,
    end_at: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM overtime_requests WHERE employee_id = ? \
         AND status IN ('PENDING', 'APPROVED') AND start_at < ? AND ? < end_at)",
    )
    .bind(employee_id)
    .bind(end_at)
    .bind(start_at)
    .fetch_one(conn)
    .await
}

pub struct NewRequest<'a> {
    pub employee_id: i64,
    pub work_date: &'a str,
    pub start_at: &'a str,
    pub end_at: &'a str,
    pub minutes: i64,
    pub reason: &'a str,
    pub filed_by: Option<i64>,
}

pub async fn insert(conn: &mut SqliteConnection, r: &NewRequest<'_>) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO overtime_requests \
         (employee_id, work_date, start_at, end_at, minutes, reason, filed_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(r.employee_id)
    .bind(r.work_date)
    .bind(r.start_at)
    .bind(r.end_at)
    .bind(r.minutes)
    .bind(r.reason)
    .bind(r.filed_by)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

/// Moves a request to a final status. `decided_by` is null for the employee's own cancel.
/// It only applies while the request is still `from`, so of two people acting on the same
/// request at once, only the first changes it. Returns whether it changed.
pub async fn set_status(
    conn: &mut SqliteConnection,
    id: i64,
    from: &str,
    status: &str,
    decided_by: Option<i64>,
    note: Option<&str>,
    now: &str,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE overtime_requests SET status = ?, decided_by = ?, decided_at = ?, \
         decision_note = ?, updated_at = ? WHERE id = ? AND status = ?",
    )
    .bind(status)
    .bind(decided_by)
    .bind(now)
    .bind(note)
    .bind(now)
    .bind(id)
    .bind(from)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// The first work date in `[from, to]` with the employee's pending or approved overtime.
pub async fn first_live_in(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT MIN(work_date) FROM overtime_requests WHERE employee_id = ? \
         AND status IN ('PENDING', 'APPROVED') AND work_date BETWEEN ? AND ?",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_one(conn)
    .await
}

/// Pending and approved overtime worked on `work_date`.
pub async fn live_on(
    conn: &mut SqliteConnection,
    work_date: &str,
) -> sqlx::Result<Vec<OvertimeRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE o.work_date = ? AND o.status IN ('PENDING', 'APPROVED') ORDER BY o.id"
    ))
    .bind(work_date)
    .fetch_all(conn)
    .await
}

/// The employee's pending and approved overtime after `date`.
pub async fn live_after(
    conn: &mut SqliteConnection,
    employee_id: i64,
    date: &str,
) -> sqlx::Result<Vec<OvertimeRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE o.employee_id = ? AND o.status IN ('PENDING', 'APPROVED') \
         AND o.work_date > ? ORDER BY o.work_date"
    ))
    .bind(employee_id)
    .bind(date)
    .fetch_all(conn)
    .await
}
