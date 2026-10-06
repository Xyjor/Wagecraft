//! SQL for the holiday calendar. No rules here: the holidays service decides.

use crate::domain::org::Holiday;
use sqlx::{SqliteConnection, SqliteExecutor};

const HOLIDAY: &str = "SELECT h.id, h.date, h.name, h.type AS kind, \
    EXISTS (SELECT 1 FROM attendance_records a \
            WHERE a.work_date = h.date AND a.locked_by_period_id IS NOT NULL) \
    OR EXISTS (SELECT 1 FROM payroll_periods pp WHERE pp.status = 'POSTED' \
            AND h.date BETWEEN pp.period_start AND pp.period_end) AS locked \
    FROM holidays h";

/// Every holiday in `year`, by date then name.
pub async fn in_year<'e>(db: impl SqliteExecutor<'e>, year: i32) -> sqlx::Result<Vec<Holiday>> {
    sqlx::query_as(&format!(
        "{HOLIDAY} WHERE h.date BETWEEN ? AND ? ORDER BY h.date, h.name"
    ))
    .bind(format!("{year:04}-01-01"))
    .bind(format!("{year:04}-12-31"))
    .fetch_all(db)
    .await
}

pub async fn by_id<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<Holiday>> {
    sqlx::query_as(&format!("{HOLIDAY} WHERE h.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// How many other holidays fall on `date`.
pub async fn count_on(
    conn: &mut SqliteConnection,
    date: &str,
    except_id: Option<i64>,
) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM holidays WHERE date = ? AND id IS NOT ?")
        .bind(date)
        .bind(except_id)
        .fetch_one(conn)
        .await
}

/// Another holiday on `date` already using `name` (case-insensitive).
pub async fn name_taken(
    conn: &mut SqliteConnection,
    date: &str,
    name: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM holidays \
         WHERE date = ? AND lower(name) = lower(?) AND id IS NOT ?)",
    )
    .bind(date)
    .bind(name)
    .bind(except_id)
    .fetch_one(conn)
    .await
}

/// Whether `date` is in a posted payroll period, or any attendance on it is locked by one.
pub async fn date_locked(conn: &mut SqliteConnection, date: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM attendance_records \
         WHERE work_date = ? AND locked_by_period_id IS NOT NULL) \
         OR EXISTS (SELECT 1 FROM payroll_periods WHERE status = 'POSTED' \
         AND ? BETWEEN period_start AND period_end)",
    )
    .bind(date)
    .bind(date)
    .fetch_one(conn)
    .await
}

pub async fn insert(
    conn: &mut SqliteConnection,
    date: &str,
    name: &str,
    kind: &str,
) -> sqlx::Result<i64> {
    let res = sqlx::query("INSERT INTO holidays (date, name, type) VALUES (?, ?, ?)")
        .bind(date)
        .bind(name)
        .bind(kind)
        .execute(conn)
        .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update(
    conn: &mut SqliteConnection,
    id: i64,
    date: &str,
    name: &str,
    kind: &str,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE holidays SET date = ?, name = ?, type = ?, updated_at = ? WHERE id = ?")
        .bind(date)
        .bind(name)
        .bind(kind)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM holidays WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
