//! SQL for payroll periods. No rules here: the payroll service decides.

use crate::domain::payroll_period::{PayrollPeriod, PendingItem};
use sqlx::{SqliteConnection, SqliteExecutor};

const SELECT: &str = "SELECT p.id, p.period_start, p.period_end, p.pay_date, p.cutoff_no, \
    p.status, r.code AS rule_pack_code, u.username AS created_by_name, p.created_at \
    FROM payroll_periods p \
    JOIN rule_packs r ON r.id = p.rule_pack_id \
    LEFT JOIN users u ON u.id = p.created_by";

/// Newest first.
pub async fn all<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<PayrollPeriod>> {
    sqlx::query_as(&format!("{SELECT} ORDER BY p.period_start DESC"))
        .fetch_all(db)
        .await
}

pub async fn by_id<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<PayrollPeriod>> {
    sqlx::query_as(&format!("{SELECT} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn by_start<'e>(
    db: impl SqliteExecutor<'e>,
    period_start: &str,
) -> sqlx::Result<Option<PayrollPeriod>> {
    sqlx::query_as(&format!("{SELECT} WHERE p.period_start = ?"))
        .bind(period_start)
        .fetch_optional(db)
        .await
}

pub struct NewPeriod<'a> {
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub pay_date: &'a str,
    pub cutoff_no: i64,
    pub rule_pack_id: i64,
    pub created_by: Option<i64>,
    pub created_at: &'a str,
}

pub async fn insert(conn: &mut SqliteConnection, p: &NewPeriod<'_>) -> sqlx::Result<i64> {
    let id = sqlx::query(
        "INSERT INTO payroll_periods \
         (period_start, period_end, pay_date, cutoff_no, rule_pack_id, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(p.period_start)
    .bind(p.period_end)
    .bind(p.pay_date)
    .bind(p.cutoff_no)
    .bind(p.rule_pack_id)
    .bind(p.created_by)
    .bind(p.created_at)
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn delete(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM payroll_periods WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

const NAME: &str = "e.employee_no, e.last_name || ', ' || e.first_name AS employee_name";

/// Pending leave requests that touch the days `from` to `to`.
pub async fn pending_leave<'e>(
    db: impl SqliteExecutor<'e>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<PendingItem>> {
    sqlx::query_as(&format!(
        "SELECT {NAME}, l.start_date AS date FROM leave_requests l \
         JOIN employees e ON e.id = l.employee_id \
         WHERE l.status = 'PENDING' AND l.start_date <= ? AND l.end_date >= ? \
         ORDER BY l.start_date, e.employee_no"
    ))
    .bind(to)
    .bind(from)
    .fetch_all(db)
    .await
}

/// Pending overtime requests on the days `from` to `to`.
pub async fn pending_overtime<'e>(
    db: impl SqliteExecutor<'e>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<PendingItem>> {
    sqlx::query_as(&format!(
        "SELECT {NAME}, o.work_date AS date FROM overtime_requests o \
         JOIN employees e ON e.id = o.employee_id \
         WHERE o.status = 'PENDING' AND o.work_date BETWEEN ? AND ? \
         ORDER BY o.work_date, e.employee_no"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

/// Days from `from` to `to` where someone clocked in and never clocked out.
pub async fn missing_time_outs<'e>(
    db: impl SqliteExecutor<'e>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<PendingItem>> {
    sqlx::query_as(&format!(
        "SELECT {NAME}, a.work_date AS date FROM attendance_records a \
         JOIN employees e ON e.id = a.employee_id \
         WHERE a.time_in IS NOT NULL AND a.time_out IS NULL AND a.work_date BETWEEN ? AND ? \
         ORDER BY a.work_date, e.employee_no"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

pub async fn mark_computed(
    conn: &mut SqliteConnection,
    id: i64,
    by: Option<i64>,
    at: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE payroll_periods SET status = 'COMPUTED', computed_by = ?, computed_at = ? \
         WHERE id = ?",
    )
    .bind(by)
    .bind(at)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// The last day of the latest period that is computed, approved or posted.
pub async fn last_computed_end<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT MAX(period_end) FROM payroll_periods WHERE status <> 'DRAFT'")
        .fetch_one(db)
        .await
}
