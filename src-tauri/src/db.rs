use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::{path::Path, time::Duration};

/// The database's file name in the app data folder.
pub const DB_FILE: &str = "wagecraft.db";

/// Opens (or creates) the Wagecraft database at `path` and applies any pending migrations.
pub async fn open(path: &Path) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    // One connection: Wagecraft has one user at a time, and this rules out SQLITE_BUSY.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// The newest migration this build of the app has. A backup from a newer build can't be
/// restored (plan §6.7).
pub fn newest_migration() -> i64 {
    sqlx::migrate!("./migrations")
        .migrations
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many migration files the app ships with, so these tests don't change with every migration.
    fn shipped_migrations() -> i64 {
        sqlx::migrate!("./migrations").migrations.len() as i64
    }

    #[tokio::test]
    async fn open_creates_wal_database_and_runs_migrations() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = open(&dir.path().join("test.db")).await.expect("open");

        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("pragma");
        assert_eq!(mode, "wal");

        let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .expect("pragma");
        assert_eq!(fk, 1);

        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migrations table");
        assert_eq!(applied, shipped_migrations());
    }

    async fn add_employee(pool: &SqlitePool) -> i64 {
        sqlx::query(
            "INSERT INTO employees (employee_no, first_name, last_name, hire_date, employment_status) \
             VALUES ('E-0001', 'Juan', 'Dela Cruz', '2026-01-05', 'REGULAR')",
        )
        .execute(pool)
        .await
        .expect("employee")
        .last_insert_rowid()
    }

    #[tokio::test]
    async fn employees_can_be_archived_but_not_deleted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = open(&dir.path().join("test.db")).await.expect("open");
        add_employee(&pool).await;

        let err = sqlx::query("DELETE FROM employees")
            .execute(&pool)
            .await
            .expect_err("delete must be refused");
        assert!(err.to_string().contains("archived, not deleted"), "{err}");
    }

    #[tokio::test]
    async fn a_user_can_only_link_to_a_real_employee() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = open(&dir.path().join("test.db")).await.expect("open");
        let employee = add_employee(&pool).await;
        let insert = |employee_id: i64| {
            sqlx::query(
                "INSERT INTO users (username, password_hash, role, employee_id) \
                 VALUES (?, 'x', 'STAFF', ?)",
            )
            .bind(format!("user{employee_id}"))
            .bind(employee_id)
            .execute(&pool)
        };

        let ghost = insert(999).await.expect_err("no employee 999");
        assert!(ghost.to_string().contains("existing employee"), "{ghost}");
        insert(employee).await.expect("real employee");

        let relink = sqlx::query("UPDATE users SET employee_id = 999")
            .execute(&pool)
            .await
            .expect_err("relink to a missing employee");
        assert!(relink.to_string().contains("existing employee"), "{relink}");
    }

    #[tokio::test]
    async fn open_twice_does_not_rerun_migrations() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("test.db");
        open(&path).await.expect("first open").close().await;
        let pool = open(&path).await.expect("second open");

        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migrations table");
        assert_eq!(applied, shipped_migrations());
    }
}
