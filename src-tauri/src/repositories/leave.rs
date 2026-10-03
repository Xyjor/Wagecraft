//! SQL for leave types and balances. No rules here: the leave service decides.

use crate::domain::leave::{LeaveBalance, LeaveType};
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const TYPE: &str = "SELECT id, code, name, is_paid, default_halfdays_per_year, \
    min_service_months, is_active FROM leave_types";

pub async fn types<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<LeaveType>> {
    sqlx::query_as(&format!("{TYPE} ORDER BY is_active DESC, name"))
        .fetch_all(db)
        .await
}

pub async fn type_by_id<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<LeaveType>> {
    sqlx::query_as(&format!("{TYPE} WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Another type already using `code` (case-insensitive).
pub async fn code_taken(
    conn: &mut SqliteConnection,
    code: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave_types WHERE code = ? AND id IS NOT ?)")
        .bind(code)
        .bind(except_id)
        .fetch_one(conn)
        .await
}

pub struct TypeFields<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub is_paid: bool,
    pub default_halfdays_per_year: i64,
    pub min_service_months: i64,
}

pub async fn insert_type(conn: &mut SqliteConnection, t: &TypeFields<'_>) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO leave_types \
         (code, name, is_paid, default_halfdays_per_year, min_service_months) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(t.code)
    .bind(t.name)
    .bind(t.is_paid)
    .bind(t.default_halfdays_per_year)
    .bind(t.min_service_months)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_type(
    conn: &mut SqliteConnection,
    id: i64,
    t: &TypeFields<'_>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE leave_types SET code = ?, name = ?, is_paid = ?, \
         default_halfdays_per_year = ?, min_service_months = ?, updated_at = ? WHERE id = ?",
    )
    .bind(t.code)
    .bind(t.name)
    .bind(t.is_paid)
    .bind(t.default_halfdays_per_year)
    .bind(t.min_service_months)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_type_active(
    conn: &mut SqliteConnection,
    id: i64,
    active: bool,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE leave_types SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

const BALANCE: &str = "SELECT b.id, b.employee_id, b.leave_type_id, \
    t.code AS leave_type_code, t.name AS leave_type_name, b.year, b.entitled_halfdays, \
    b.used_halfdays \
    FROM leave_balances b JOIN leave_types t ON t.id = b.leave_type_id";

/// One employee's balances for a year, by type name.
pub async fn balances<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    year: i32,
) -> sqlx::Result<Vec<LeaveBalance>> {
    sqlx::query_as(&format!(
        "{BALANCE} WHERE b.employee_id = ? AND b.year = ? ORDER BY t.name"
    ))
    .bind(employee_id)
    .bind(year)
    .fetch_all(db)
    .await
}

pub async fn balance_by_id<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<LeaveBalance>> {
    sqlx::query_as(&format!("{BALANCE} WHERE b.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn set_entitled(
    conn: &mut SqliteConnection,
    id: i64,
    entitled_halfdays: i64,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE leave_balances SET entitled_halfdays = ?, updated_at = ? WHERE id = ?")
        .bind(entitled_halfdays)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Who the yearly grant considers: current employees hired by `as_of`.
#[derive(Debug, FromRow)]
pub struct GrantEmployee {
    pub id: i64,
    pub hire_date: String,
}

pub async fn grant_employees(
    conn: &mut SqliteConnection,
    as_of: &str,
    only: Option<i64>,
) -> sqlx::Result<Vec<GrantEmployee>> {
    sqlx::query_as(
        "SELECT id, hire_date FROM employees WHERE archived_at IS NULL \
         AND employment_status NOT IN ('RESIGNED', 'TERMINATED') AND hire_date <= ?1 \
         AND (?2 IS NULL OR id = ?2) ORDER BY id",
    )
    .bind(as_of)
    .bind(only)
    .fetch_all(conn)
    .await
}

/// Adds a balance unless the employee already has one for that type and year.
/// Returns whether a row was added.
pub async fn insert_balance_if_missing(
    conn: &mut SqliteConnection,
    employee_id: i64,
    leave_type_id: i64,
    year: i32,
    entitled_halfdays: i64,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO leave_balances (employee_id, leave_type_id, year, entitled_halfdays) \
         VALUES (?, ?, ?, ?) ON CONFLICT (employee_id, leave_type_id, year) DO NOTHING",
    )
    .bind(employee_id)
    .bind(leave_type_id)
    .bind(year)
    .bind(entitled_halfdays)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}
