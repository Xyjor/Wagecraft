use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::{path::Path, time::Duration};

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
