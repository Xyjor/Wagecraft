//! SQL for the `users` table. No rules here: the auth service decides, this only reads and writes.

use sqlx::{FromRow, SqliteConnection, SqlitePool};

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

pub async fn by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<UserRow>> {
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
