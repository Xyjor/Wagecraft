//! SQL for salary history. No rules here: the compensation service decides.

use crate::domain::compensation::Compensation;
use sqlx::{SqliteConnection, SqliteExecutor};

const SELECT: &str = "SELECT c.id, c.employee_id, c.pay_basis, c.rate_cents, c.effective_from, \
    c.effective_to, c.reason, u.username AS created_by_name, c.created_at \
    FROM compensations c LEFT JOIN users u ON u.id = c.created_by";

/// Newest first, so the rate in effect now is the first row.
pub async fn for_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
) -> sqlx::Result<Vec<Compensation>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE c.employee_id = ? ORDER BY c.effective_from DESC"
    ))
    .bind(employee_id)
    .fetch_all(db)
    .await
}

pub async fn get<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<Compensation>> {
    sqlx::query_as(&format!("{SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn close(conn: &mut SqliteConnection, id: i64, effective_to: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE compensations SET effective_to = ? WHERE id = ?")
        .bind(effective_to)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub struct NewCompensation<'a> {
    pub employee_id: i64,
    pub pay_basis: &'a str,
    pub rate_cents: i64,
    pub effective_from: &'a str,
    pub reason: Option<&'a str>,
    pub created_by: Option<i64>,
    pub created_at: &'a str,
}

pub async fn insert(conn: &mut SqliteConnection, c: &NewCompensation<'_>) -> sqlx::Result<i64> {
    let id = sqlx::query(
        "INSERT INTO compensations \
         (employee_id, pay_basis, rate_cents, effective_from, reason, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(c.employee_id)
    .bind(c.pay_basis)
    .bind(c.rate_cents)
    .bind(c.effective_from)
    .bind(c.reason)
    .bind(c.created_by)
    .bind(c.created_at)
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}
