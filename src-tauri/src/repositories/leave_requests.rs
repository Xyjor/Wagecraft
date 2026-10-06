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
            AND a.locked_by_period_id IS NOT NULL) \
    OR EXISTS (SELECT 1 FROM payroll_periods pp WHERE pp.status = 'POSTED' \
            AND pp.period_start <= r.end_date AND pp.period_end >= r.start_date) AS locked \
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
    pub hire_date: String,
    pub separation_date: Option<String>,
}

const EMPLOYEE: &str =
    "SELECT e.id, e.archived_at, e.hire_date, e.separation_date FROM employees e";

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

/// Whether any day in `[from, to]` is in a posted payroll period, or has attendance locked
/// by one.
pub async fn range_locked(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM attendance_records WHERE employee_id = ? \
         AND work_date BETWEEN ? AND ? AND locked_by_period_id IS NOT NULL) \
         OR EXISTS (SELECT 1 FROM payroll_periods WHERE status = 'POSTED' \
         AND period_start <= ? AND period_end >= ?)",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .bind(to)
    .bind(from)
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

/// Someone's approved leave on one date, for the attendance grid.
#[derive(Debug, FromRow)]
pub struct LeaveOnDay {
    pub employee_id: i64,
    pub leave_type_name: String,
    pub half_day: bool,
}

pub async fn approved_on<'e>(
    db: impl SqliteExecutor<'e>,
    date: &str,
) -> sqlx::Result<Vec<LeaveOnDay>> {
    sqlx::query_as(
        "SELECT r.employee_id, t.name AS leave_type_name, r.half_day FROM leave_requests r \
         JOIN leave_types t ON t.id = r.leave_type_id \
         WHERE r.status = 'APPROVED' AND ?1 BETWEEN r.start_date AND r.end_date",
    )
    .bind(date)
    .fetch_all(db)
    .await
}

/// Whether the employee has approved whole-day leave on `date`, or with `or_pending`,
/// pending whole-day leave too.
pub async fn full_day_leave_on(
    conn: &mut SqliteConnection,
    employee_id: i64,
    date: &str,
    or_pending: bool,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM leave_requests WHERE employee_id = ?1 \
         AND (status = 'APPROVED' OR (?3 AND status = 'PENDING')) AND half_day = 0 \
         AND ?2 BETWEEN start_date AND end_date)",
    )
    .bind(employee_id)
    .bind(date)
    .bind(or_pending)
    .fetch_one(conn)
    .await
}

/// The employee's pending and approved requests that end after `date`.
pub async fn live_after(
    conn: &mut SqliteConnection,
    employee_id: i64,
    date: &str,
) -> sqlx::Result<Vec<LeaveRequest>> {
    sqlx::query_as(&format!(
        "{REQUEST} WHERE r.employee_id = ?1 AND r.status IN ('PENDING', 'APPROVED') \
         AND r.end_date > ?2 ORDER BY r.start_date"
    ))
    .bind(employee_id)
    .bind(date)
    .fetch_all(conn)
    .await
}

/// The first date in `[from, to]` the employee has a time in, if any.
pub async fn first_worked_day(
    conn: &mut SqliteConnection,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT MIN(work_date) FROM attendance_records WHERE employee_id = ? \
         AND work_date BETWEEN ? AND ? AND time_in IS NOT NULL",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_one(conn)
    .await
}

/// A pending or approved request whose length may change with the holidays.
#[derive(Debug, FromRow)]
pub struct Recount {
    pub id: i64,
    pub employee_id: i64,
    pub employee_no: String,
    pub leave_type_id: i64,
    pub is_paid: bool,
    pub start_date: String,
    pub end_date: String,
    pub half_day: bool,
    pub halfdays: i64,
    pub status: String,
}

/// Pending and approved requests covering `date`.
pub async fn live_on(conn: &mut SqliteConnection, date: &str) -> sqlx::Result<Vec<Recount>> {
    sqlx::query_as(
        "SELECT r.id, r.employee_id, e.employee_no, r.leave_type_id, t.is_paid, r.start_date, \
         r.end_date, r.half_day, r.halfdays, r.status \
         FROM leave_requests r \
         JOIN employees e ON e.id = r.employee_id \
         JOIN leave_types t ON t.id = r.leave_type_id \
         WHERE r.status IN ('PENDING', 'APPROVED') AND ?1 BETWEEN r.start_date AND r.end_date \
         ORDER BY r.id",
    )
    .bind(date)
    .fetch_all(conn)
    .await
}

/// The employee's pending and approved requests that run on or past `date`.
pub async fn live_through(
    conn: &mut SqliteConnection,
    employee_id: i64,
    date: &str,
) -> sqlx::Result<Vec<Recount>> {
    sqlx::query_as(
        "SELECT r.id, r.employee_id, e.employee_no, r.leave_type_id, t.is_paid, r.start_date, \
         r.end_date, r.half_day, r.halfdays, r.status \
         FROM leave_requests r \
         JOIN employees e ON e.id = r.employee_id \
         JOIN leave_types t ON t.id = r.leave_type_id \
         WHERE r.status IN ('PENDING', 'APPROVED') AND r.employee_id = ? AND r.end_date >= ? \
         ORDER BY r.id",
    )
    .bind(employee_id)
    .bind(date)
    .fetch_all(conn)
    .await
}

pub async fn set_halfdays(
    conn: &mut SqliteConnection,
    id: i64,
    halfdays: i64,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE leave_requests SET halfdays = ?, updated_at = ? WHERE id = ?")
        .bind(halfdays)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
