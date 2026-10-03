//! SQL for the `users` table. No rules here: the auth service decides, this only reads and writes.

use sqlx::{FromRow, SqliteConnection, SqliteExecutor, SqlitePool};

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub employee_id: Option<i64>,
    pub is_active: bool,
    pub must_change_password: bool,
    pub failed_attempts: i64,
    pub locked_until: Option<String>,
    pub theme: String,
}

const COLUMNS: &str = "id, username, password_hash, role, employee_id, is_active, \
    must_change_password, failed_attempts, locked_until, theme";

pub async fn count(conn: &mut SqliteConnection) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(conn)
        .await
}

/// Case-insensitive, because `username` is declared `COLLATE NOCASE`.
pub async fn by_username(db: &SqlitePool, username: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM users WHERE username = ?"))
        .bind(username)
        .fetch_optional(db)
        .await
}

pub async fn by_id<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM users WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn insert(
    conn: &mut SqliteConnection,
    username: &str,
    password_hash: &str,
    role: &str,
    must_change_password: bool,
) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO users (username, password_hash, role, must_change_password) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(must_change_password)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn record_failure(
    conn: &mut SqliteConnection,
    id: i64,
    failed_attempts: i64,
    locked_until: Option<&str>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET failed_attempts = ?, locked_until = ?, updated_at = ? WHERE id = ?",
    )
    .bind(failed_attempts)
    .bind(locked_until)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn record_success(conn: &mut SqliteConnection, id: i64, now: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET failed_attempts = 0, locked_until = NULL, last_login_at = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_password(
    conn: &mut SqliteConnection,
    id: i64,
    password_hash: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = ?, must_change_password = 0, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(password_hash)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// One row of the Users screen.
#[derive(Debug, FromRow)]
pub struct ListRow {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub employee_id: Option<i64>,
    /// "EMP-0001 Maria Santos" for a linked account.
    pub employee_label: Option<String>,
    pub is_active: bool,
    pub must_change_password: bool,
    pub locked_until: Option<String>,
    pub last_login_at: Option<String>,
}

pub async fn list(db: &SqlitePool) -> sqlx::Result<Vec<ListRow>> {
    sqlx::query_as(
        "SELECT u.id, u.username, u.role, u.employee_id, \
         e.employee_no || ' ' || e.first_name || ' ' || e.last_name AS employee_label, \
         u.is_active, u.must_change_password, u.locked_until, u.last_login_at \
         FROM users u LEFT JOIN employees e ON e.id = u.employee_id ORDER BY u.username",
    )
    .fetch_all(db)
    .await
}

pub async fn count_active_admins(conn: &mut SqliteConnection) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND is_active = 1")
        .fetch_one(conn)
        .await
}

pub async fn set_role(
    conn: &mut SqliteConnection,
    id: i64,
    role: &str,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
        .bind(role)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_active(
    conn: &mut SqliteConnection,
    id: i64,
    active: bool,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Sets a temporary password chosen by an Admin. The user must change it at next sign-in,
/// and any lockout is lifted.
pub async fn reset_password(
    conn: &mut SqliteConnection,
    id: i64,
    password_hash: &str,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = ?, must_change_password = 1, failed_attempts = 0, \
         locked_until = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(password_hash)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// The account linked to an employee, if any. `employee_id` is UNIQUE, so at most one.
pub async fn by_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM users WHERE employee_id = ?"
    ))
    .bind(employee_id)
    .fetch_optional(db)
    .await
}

/// Accounts not linked to any employee, for the "link an existing account" picker.
pub async fn unlinked<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<UserRow>> {
    sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM users WHERE employee_id IS NULL ORDER BY username"
    ))
    .fetch_all(db)
    .await
}

pub async fn set_employee(
    conn: &mut SqliteConnection,
    id: i64,
    employee_id: Option<i64>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET employee_id = ?, updated_at = ? WHERE id = ?")
        .bind(employee_id)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
