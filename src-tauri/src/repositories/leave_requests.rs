//! SQL for leave requests. No rules here: the leave request service decides.

use crate::domain::leave::LeaveRequest;
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const REQUEST: &str = "SELECT r.id, r.employee_id, e.employee_no, \
    e.last_name || ', ' || e.first_name AS employee_name, r.leave_type_id, \
    t.code AS leave_type_code, t.name AS leave_type_name, t.is_paid, r.start_date, \
    r.end_date, r.half_day, r.halfdays, r.reason, r.status, \
    f.username AS filed_by_username, d.username AS decided_by_username, r.decided_at, \
    r.decision_note, \
    EXISTS (SELECT 1 FROM attendance_records a WHERE a.employee_id = r.employee_id \
            AND a.work_date BETWEEN r.start_date AND r.end_date \
            AND a.locked_by_period_id IS NOT NULL) AS locked \
    FROM leave_requests r \
    JOIN employees e ON e.id = r.employee_id \
    JOIN leave_types t ON t.id = r.leave_type_id \
    LEFT JOIN users f ON f.id = r.filed_by \
    LEFT JOIN users d ON d.id = r.decided_by";

pub async fn by_id<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<LeaveRequest>> {
    sqlx::query_as(&format!("{REQUEST} WHERE r.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// One employee's requests touching `[from, to]`, newest first.
pub async fn for_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<LeaveRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE r.employee_id = ?1 AND r.start_date <= ?3 AND r.end_date >= ?2 \
         ORDER BY r.start_date DESC, r.id DESC"
    ))
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

/// Everyone's requests touching `[from, to]`, optionally with one status.
pub async fn list<'e>(
    db: impl SqliteExecutor<'e>,
    status: Option<&str>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<LeaveRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE r.start_date <= ?2 AND r.end_date >= ?1 \
         AND (?3 IS NULL OR r.status = ?3) ORDER BY r.start_date, employee_name"
    ))
    .bind(from)
    .bind(to)
    .bind(status)
    .fetch_all(db)
    .await
}

/// Every pending request, whatever its dates, oldest first.
pub async fn pending<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<LeaveRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE r.status = 'PENDING' ORDER BY r.start_date, employee_name"
    ))
    .fetch_all(db)
    .await
}

/// The employee facts filing needs. Never sent to the UI.
#[derive(Debug, FromRow)]
pub struct LeaveEmployee {
    pub id: i64,
    pub archived_at: Option<String>,
    pub work_days: Option<String>,
}

const EMPLOYEE: &str = "SELECT e.id, e.archived_at, s.work_days \
    FROM employees e LEFT JOIN work_schedules s ON s.id = e.schedule_id";

pub async fn employee_by_id(
    conn: &mut SqliteConnection,
    id: i64,
) -> sqlx::Result<Option<LeaveEmployee>> {
    sqlx::query_as(&format!("{EMPLOYEE} WHERE e.id = ?"))
        .bind(id)
        .fetch_optional(conn)
        .await
}

pub async fn employee_by_no(
    conn: &mut SqliteConnection,
    employee_no: &str,
) -> sqlx::Result<Option<LeaveEmployee>> {
    sqlx::query_as(&format!("{EMPLOYEE} WHERE e.employee_no = ?"))
        .bind(employee_no)
        .fetch_optional(conn)
        .await
}

/// Holiday dates in `[from, to]` that are days off (regular and special non-working).
pub async fn days_off(
    conn: &mut SqliteConnection,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT DISTINCT date FROM holidays WHERE date BETWEEN ? AND ? \
         AND type IN ('REGULAR', 'SPECIAL_NON_WORKING')",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await
}

/// Whether any of the employee's attendance in `[from, to]` is in a posted payroll period.
pub async fn range_locked(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM attendance_records WHERE employee_id = ? \
         AND work_date BETWEEN ? AND ? AND locked_by_period_id IS NOT NULL)",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_one(conn)
    .await
}

/// Whether a pending or approved request of the employee's touches `[from, to]`.
pub async fn overlaps_active(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM leave_requests WHERE employee_id = ?1 \
         AND status IN ('PENDING', 'APPROVED') AND start_date <= ?3 AND end_date >= ?2)",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_one(conn)
    .await
}

/// Half days of a type and year still waiting for a decision.
pub async fn pending_halfdays(
    conn: &mut SqliteConnection,
    employee_id: i64,
    leave_type_id: i64,
    year: i32,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(halfdays), 0) FROM leave_requests WHERE employee_id = ? \
         AND leave_type_id = ? AND substr(start_date, 1, 4) = ? AND status = 'PENDING'",
    )
    .bind(employee_id)
    .bind(leave_type_id)
    .bind(format!("{year:04}"))
    .fetch_one(conn)
    .await
}

/// `(entitled, used)` for one employee, type and year, if granted.
pub async fn balance(
    conn: &mut SqliteConnection,
    employee_id: i64,
    leave_type_id: i64,
    year: i32,
) -> sqlx::Result<Option<(i64, i64)>> {
    sqlx::query_as(
        "SELECT entitled_halfdays, used_halfdays FROM leave_balances \
         WHERE employee_id = ? AND leave_type_id = ? AND year = ?",
    )
    .bind(employee_id)
    .bind(leave_type_id)
    .bind(year)
    .fetch_optional(conn)
    .await
}

/// Takes `halfdays` from the balance if enough is left. Returns whether it did.
pub async fn use_balance(
    conn: &mut SqliteConnection,
    employee_id: i64,
    leave_type_id: i64,
    year: i32,
    halfdays: i64,
    now: &str,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE leave_balances SET used_halfdays = used_halfdays + ?1, updated_at = ?5 \
         WHERE employee_id = ?2 AND leave_type_id = ?3 AND year = ?4 \
         AND used_halfdays + ?1 <= entitled_halfdays",
    )
    .bind(halfdays)
    .bind(employee_id)
    .bind(leave_type_id)
    .bind(year)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Gives `halfdays` back to the balance when approved leave is cancelled.
pub async fn restore_balance(
    conn: &mut SqliteConnection,
    employee_id: i64,
    leave_type_id: i64,
    year: i32,
    halfdays: i64,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE leave_balances SET used_halfdays = MAX(used_halfdays - ?1, 0), \
         updated_at = ?5 WHERE employee_id = ?2 AND leave_type_id = ?3 AND year = ?4",
    )
    .bind(halfdays)
    .bind(employee_id)
    .bind(leave_type_id)
    .bind(year)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

pub struct NewRequest<'a> {
    pub employee_id: i64,
    pub leave_type_id: i64,
    pub start_date: &'a str,
    pub end_date: &'a str,
    pub half_day: bool,
    pub halfdays: i64,
    pub reason: &'a str,
    pub filed_by: Option<i64>,
}

pub async fn insert(conn: &mut SqliteConnection, r: &NewRequest<'_>) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
         half_day, halfdays, reason, filed_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(r.employee_id)
    .bind(r.leave_type_id)
    .bind(r.start_date)
    .bind(r.end_date)
    .bind(r.half_day)
    .bind(r.halfdays)
    .bind(r.reason)
    .bind(r.filed_by)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

/// Moves a request to a final status. `decided_by` is the account that acted.
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
        "UPDATE leave_requests SET status = ?, decided_by = ?, decided_at = ?, \
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
