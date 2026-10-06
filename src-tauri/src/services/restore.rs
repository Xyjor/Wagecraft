//! Restoring a backup (plan §6.7, ADR-009). Check the file, show what it holds, take a
//! PRE_RESTORE backup, swap the database file while the pool is closed, then restart.
//! A marker file carries who restored what across the restart, so the restored database
//! gets the audit entry.

use crate::audit::{self, Actor, Entry};
use crate::db::DB_FILE;
use crate::domain::backup::RestorePreview;
use crate::error::AppError;
use crate::services::backups;
use crate::time;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{ConnectOptions, SqlitePool};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The file the app writes before restarting, next to the database.
pub const MARKER: &str = "restore-marker.json";

const NOT_WAGECRAFT: &str = "This file isn't a Wagecraft backup";
const DAMAGED: &str = "This backup is damaged and can't be restored";
const TOO_NEW: &str =
    "This backup is from a newer version of Wagecraft. Update Wagecraft, then restore it";

/// Opens `path` read-only and checks it can be restored by an app whose newest migration
/// is `newest_version`. Returns what it holds.
pub async fn inspect(path: &Path, newest_version: i64) -> Result<RestorePreview, AppError> {
    // Immutable: no locks, no WAL, no writes, so checking a backup can't change it.
    let mut conn = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .immutable(true)
        .connect()
        .await
        .map_err(unreadable)?;
    let checks: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(&mut conn)
        .await
        .map_err(unreadable)?;
    if checks != ["ok"] {
        log::warn!("integrity check failed for {}: {checks:?}", path.display());
        return Err(AppError::Conflict(DAMAGED));
    }
    let required = ["_sqlx_migrations", "users", "settings", "audit_logs"];
    for table in required {
        if !has_table(&mut conn, table).await? {
            return Err(AppError::Conflict(NOT_WAGECRAFT));
        }
    }
    let version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
    )
    .fetch_one(&mut conn)
    .await?;
    if version > newest_version {
        return Err(AppError::Conflict(TOO_NEW));
    }
    // Older backups may predate these tables; migrations add them after the restore.
    let employee_count = if has_table(&mut conn, "employees").await? {
        sqlx::query_scalar("SELECT COUNT(*) FROM employees")
            .fetch_one(&mut conn)
            .await?
    } else {
        0
    };
    let last_posted: Option<(String, String)> = if has_table(&mut conn, "payroll_periods").await? {
        sqlx::query_as(
            "SELECT period_start, period_end FROM payroll_periods WHERE status = 'POSTED' \
             ORDER BY period_start DESC LIMIT 1",
        )
        .fetch_optional(&mut conn)
        .await?
    } else {
        None
    };
    let saved_at = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| time::to_db(DateTime::<Utc>::from(t)));
    let (last_posted_start, last_posted_end) = last_posted.unzip();
    Ok(RestorePreview {
        file_name: path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default(),
        saved_at,
        employee_count,
        last_posted_start,
        last_posted_end,
    })
}

/// SQLite says "not a database" for any other kind of file; anything else it can't read
/// is a damaged database.
fn unreadable(e: sqlx::Error) -> AppError {
    const SQLITE_NOTADB: &str = "26";
    let code = e.as_database_error().and_then(|d| d.code());
    log::warn!("can't read the chosen backup: {e}");
    if code.as_deref() == Some(SQLITE_NOTADB) {
        AppError::Conflict(NOT_WAGECRAFT)
    } else {
        AppError::Conflict(DAMAGED)
    }
}

async fn has_table(conn: &mut SqliteConnection, name: &str) -> Result<bool, AppError> {
    let found: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(name)
            .fetch_one(conn)
            .await?;
    Ok(found > 0)
}

/// Replaces the database at `db_path` with a copy of `backup`, and removes the old WAL
/// files so SQLite can't replay them onto it. The pool must be closed first.
pub fn swap_in(db_path: &Path, backup: &Path) -> Result<(), AppError> {
    // Copy next to the database first, so the final step is a rename on the same disk.
    let incoming = with_suffix(db_path, ".restoring");
    std::fs::copy(backup, &incoming).map_err(anyhow::Error::from)?;
    for suffix in ["-wal", "-shm"] {
        match std::fs::remove_file(with_suffix(db_path, suffix)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(anyhow::Error::from(e).into()),
        }
    }
    std::fs::rename(&incoming, db_path).map_err(anyhow::Error::from)?;
    Ok(())
}

/// `wagecraft.db` → `wagecraft.db-wal`.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

/// Everything a restore does before the restart: checks `backup` again, backs up the
/// current data (PRE_RESTORE), closes `db`, swaps the file in and leaves the marker.
/// Once the pool is closed the app must restart whatever this returns. If the swap
/// fails, the old database is still in place and no marker is written.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    db: &SqlitePool,
    data_dir: &Path,
    backup_dir: &Path,
    backup: &Path,
    newest_version: i64,
    actor: Actor<'_>,
    local: NaiveDateTime,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    // Checked again, in case the file changed since Admin chose it.
    inspect(backup, newest_version).await?;
    let at = time::to_db(now);
    let pre = backups::create(db, backup_dir, "PRE_RESTORE", actor.user_id, local, &at).await?;
    let marker = Marker {
        restored_from: backup.display().to_string(),
        pre_restore_path: pre.path.display().to_string(),
        pre_restore_size: pre.size_bytes,
        user_id: actor.user_id,
        username: actor.username.to_string(),
        at,
    };
    db.close().await;
    swap_in(&data_dir.join(DB_FILE), backup)?;
    write_marker(data_dir, &marker)
}

/// Who restored which file, and the PRE_RESTORE backup taken first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub restored_from: String,
    pub pre_restore_path: String,
    pub pre_restore_size: i64,
    pub user_id: Option<i64>,
    pub username: String,
    pub at: String,
}

pub fn write_marker(dir: &Path, marker: &Marker) -> Result<(), AppError> {
    let text = serde_json::to_string(marker).map_err(anyhow::Error::from)?;
    std::fs::write(dir.join(MARKER), text).map_err(anyhow::Error::from)?;
    Ok(())
}

/// Run at app start, after migrations. If a restore just happened, records the
/// PRE_RESTORE backup and a `backup.restore` audit entry in the restored database, then
/// removes the marker. Returns whether it found one.
pub async fn finish(db: &SqlitePool, dir: &Path, now: DateTime<Utc>) -> Result<bool, AppError> {
    let path = dir.join(MARKER);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(anyhow::Error::from(e).into()),
    };
    let marker: Marker = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(e) => {
            // A broken marker would fail every start; drop it and say so in the log.
            let _ = std::fs::remove_file(&path);
            return Err(anyhow::Error::from(e)
                .context("unreadable restore marker")
                .into());
        }
    };
    let mut tx = db.begin().await?;
    // The restored database may not have the account that did the restore, or may have a
    // different one under that id, so only link it when both id and name match.
    let user_id = match marker.user_id {
        Some(id) => {
            sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE id = ? AND username = ?")
                .bind(id)
                .bind(&marker.username)
                .fetch_optional(&mut *tx)
                .await?
        }
        None => None,
    };
    let backup_id = sqlx::query(
        "INSERT INTO backup_history (path, size_bytes, kind, created_by, created_at) \
         VALUES (?, ?, 'PRE_RESTORE', ?, ?)",
    )
    .bind(&marker.pre_restore_path)
    .bind(marker.pre_restore_size)
    .bind(user_id)
    .bind(&marker.at)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    let actor = Actor {
        user_id,
        username: &marker.username,
    };
    let entry = Entry {
        action: "backup.restore",
        entity: Some(("backup", backup_id)),
        before: None,
        after: Some(json!({
            "restoredFrom": marker.restored_from,
            "preRestoreBackup": marker.pre_restore_path,
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    std::fs::remove_file(&path).map_err(anyhow::Error::from)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::services::backups;
    use crate::services::payroll_fixtures::{self as fx, sql};
    use chrono::{NaiveDate, TimeZone};

    fn newest() -> i64 {
        crate::db::newest_migration()
    }

    fn local() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(17, 30, 0)
            .unwrap()
    }

    async fn backup_of(db: &SqlitePool, dir: &Path) -> std::path::PathBuf {
        backups::create(db, dir, "MANUAL", None, local(), "2026-11-02T09:30:00Z")
            .await
            .expect("backup")
            .path
    }

    fn conflict(e: AppError) -> &'static str {
        match e {
            AppError::Conflict(m) => m,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    mod inspect {
        use super::*;

        #[tokio::test]
        async fn reports_what_a_backup_holds() {
            let (tmp, db, _) = fx::db().await;
            fx::employee(&db, 1, "Santos", "2026-01-05", "MONTHLY", 3_000_000).await;
            fx::employee(&db, 2, "Reyes", "2026-01-05", "MONTHLY", 3_000_000).await;
            // Newest posted period inserted first, so the answer can't be "the last row".
            fx::posted(&db, "2026-10-01", "2026-10-15").await;
            fx::posted(&db, "2026-09-16", "2026-09-30").await;
            let file = backup_of(&db, &tmp.path().join("b")).await;

            let p = inspect(&file, newest()).await.expect("inspect");

            assert_eq!(p.file_name, "wagecraft-backup-2026-11-02_1730.db");
            assert_eq!(p.employee_count, 2);
            assert_eq!(p.last_posted_start.as_deref(), Some("2026-10-01"));
            assert_eq!(p.last_posted_end.as_deref(), Some("2026-10-15"));
            assert!(p.saved_at.is_some());
        }

        #[tokio::test]
        async fn never_changes_the_file() {
            let (tmp, db, _) = fx::db().await;
            let file = backup_of(&db, &tmp.path().join("b")).await;
            let before = std::fs::read(&file).unwrap();

            inspect(&file, newest()).await.expect("inspect");

            assert_eq!(std::fs::read(&file).unwrap(), before);
            let wal = file.with_extension("db-wal");
            assert!(!wal.exists(), "opening it must not leave a WAL behind");
        }

        #[tokio::test]
        async fn accepts_a_backup_from_an_older_version() {
            let tmp = tempfile::tempdir().unwrap();
            let file = tmp.path().join("old.db");
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&file)
                .create_if_missing(true);
            let old = SqlitePool::connect_with(options).await.unwrap();
            let all = sqlx::migrate!("./migrations");
            // Up to employees, before payroll periods existed.
            let early = sqlx::migrate::Migrator {
                migrations: std::borrow::Cow::Owned(all.migrations[..3].to_vec()),
                ..all
            };
            early.run(&old).await.unwrap();
            old.close().await;

            let p = inspect(&file, newest()).await.expect("inspect");

            assert_eq!(p.employee_count, 0);
            assert_eq!(p.last_posted_start, None);
        }

        #[tokio::test]
        async fn refuses_a_backup_from_a_newer_version() {
            let (tmp, db, _) = fx::db().await;
            let file = backup_of(&db, &tmp.path().join("b")).await;

            let e = inspect(&file, newest() - 1).await.unwrap_err();

            assert_eq!(conflict(e), TOO_NEW);
        }

        #[tokio::test]
        async fn refuses_a_file_that_is_not_a_database() {
            let tmp = tempfile::tempdir().unwrap();
            let file = tmp.path().join("notes.db");
            std::fs::write(&file, "payroll notes, not a database ".repeat(200)).unwrap();

            let e = inspect(&file, newest()).await.unwrap_err();

            assert_eq!(conflict(e), NOT_WAGECRAFT);
        }

        #[tokio::test]
        async fn refuses_another_apps_database() {
            let tmp = tempfile::tempdir().unwrap();
            let file = tmp.path().join("other.db");
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&file)
                .create_if_missing(true);
            let other = SqlitePool::connect_with(options).await.unwrap();
            sql(&other, "CREATE TABLE notes (body TEXT)").await;
            other.close().await;

            let e = inspect(&file, newest()).await.unwrap_err();

            assert_eq!(conflict(e), NOT_WAGECRAFT);
        }

        #[tokio::test]
        async fn refuses_a_backup_that_fails_the_integrity_check() {
            let (tmp, db, _) = fx::db().await;
            let file = backup_of(&db, &tmp.path().join("b")).await;
            // Make an index disagree with its table: every page still reads fine, but
            // PRAGMA integrity_check reports rows missing from the index.
            let mut copy = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&file)
                .connect()
                .await
                .unwrap();
            for q in [
                "CREATE TABLE scratch (a INTEGER)",
                "CREATE INDEX scratch_a ON scratch (a)",
                "INSERT INTO scratch VALUES (1), (2), (3)",
                "PRAGMA writable_schema = ON",
                "UPDATE sqlite_master SET sql = 'CREATE INDEX scratch_a ON scratch (a DESC)' \
                 WHERE name = 'scratch_a'",
            ] {
                sqlx::query(q).execute(&mut copy).await.expect(q);
            }
            drop(copy);

            let e = inspect(&file, newest()).await.unwrap_err();

            assert_eq!(conflict(e), DAMAGED);
        }

        #[tokio::test]
        async fn refuses_a_backup_it_cannot_read() {
            let (tmp, db, _) = fx::db().await;
            for id in 1..=40 {
                fx::employee(&db, id, "Santos", "2026-01-05", "MONTHLY", 3_000_000).await;
            }
            let file = backup_of(&db, &tmp.path().join("b")).await;
            let mut bytes = std::fs::read(&file).unwrap();
            // Scribble over everything after the first page, keeping the header readable.
            let page = 4096;
            for b in &mut bytes[page..] {
                *b = 0xA5;
            }
            std::fs::write(&file, bytes).unwrap();

            let e = inspect(&file, newest()).await.unwrap_err();

            assert_eq!(conflict(e), DAMAGED);
        }
    }

    mod swap_in {
        use super::*;

        #[tokio::test]
        async fn puts_the_backup_in_place_of_the_database() {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("wagecraft.db");
            let db = crate::db::open(&path).await.unwrap();
            sql(
                &db,
                "INSERT INTO settings (key, value) VALUES ('company_name', 'Acme')",
            )
            .await;
            let file = backup_of(&db, &tmp.path().join("b")).await;
            sql(
                &db,
                "UPDATE settings SET value = 'Changed' WHERE key = 'company_name'",
            )
            .await;
            db.close().await;

            swap_in(&path, &file).expect("swap");

            let db = crate::db::open(&path).await.unwrap();
            let name: String =
                sqlx::query_scalar("SELECT value FROM settings WHERE key = 'company_name'")
                    .fetch_one(&db)
                    .await
                    .unwrap();
            assert_eq!(name, "Acme");
            assert!(file.exists(), "the backup itself stays where it was");
        }

        #[test]
        fn removes_leftover_wal_files() {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("wagecraft.db");
            std::fs::write(&path, "old").unwrap();
            std::fs::write(tmp.path().join("wagecraft.db-wal"), "old wal").unwrap();
            std::fs::write(tmp.path().join("wagecraft.db-shm"), "old shm").unwrap();
            let file = tmp.path().join("backup.db");
            std::fs::write(&file, "new").unwrap();

            swap_in(&path, &file).expect("swap");

            assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
            assert!(!tmp.path().join("wagecraft.db-wal").exists());
            assert!(!tmp.path().join("wagecraft.db-shm").exists());
        }
    }

    mod finish {
        use super::*;

        fn marker(user_id: Option<i64>, username: &str) -> Marker {
            Marker {
                restored_from: "E:\\wagecraft-backup-2026-11-01_0800.db".into(),
                pre_restore_path: "C:\\data\\backups\\wagecraft-backup-2026-11-02_1730.db".into(),
                pre_restore_size: 123_456,
                user_id,
                username: username.into(),
                at: "2026-11-02T09:30:00Z".into(),
            }
        }

        fn now() -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 11, 2, 9, 31, 0).unwrap()
        }

        /// The setup audits creating a pay period, so pick out the restore entries.
        async fn restores(db: &SqlitePool) -> Vec<audit_rows::Row> {
            let mut rows = audit_rows::all(db).await;
            rows.retain(|r| r.action == "backup.restore");
            rows
        }

        #[tokio::test]
        async fn does_nothing_without_a_marker() {
            let (tmp, db, _) = fx::db().await;

            let before = audit_rows::all(&db).await.len();

            let found = finish(&db, tmp.path(), now()).await.unwrap();

            assert!(!found);
            assert_eq!(audit_rows::all(&db).await.len(), before);
        }

        #[tokio::test]
        async fn records_the_restore_in_the_restored_database() {
            let (tmp, db, _) = fx::db().await; // user 1 is "hr"
            write_marker(tmp.path(), &marker(Some(1), "hr")).unwrap();

            let found = finish(&db, tmp.path(), now()).await.unwrap();

            assert!(found);
            assert!(!tmp.path().join(MARKER).exists());
            let row: (String, i64, String, Option<i64>, String) = sqlx::query_as(
                "SELECT path, size_bytes, kind, created_by, created_at FROM backup_history",
            )
            .fetch_one(&db)
            .await
            .unwrap();
            assert_eq!(
                row,
                (
                    "C:\\data\\backups\\wagecraft-backup-2026-11-02_1730.db".into(),
                    123_456,
                    "PRE_RESTORE".into(),
                    Some(1),
                    "2026-11-02T09:30:00Z".into()
                )
            );
            let rows = restores(&db).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].actor_user_id, Some(1));
            assert_eq!(rows[0].actor_username, "hr");
            assert_eq!(rows[0].at, "2026-11-02T09:31:00Z");
            let after: serde_json::Value =
                serde_json::from_str(rows[0].after_json.as_deref().unwrap()).unwrap();
            assert_eq!(
                after,
                serde_json::json!({
                    "restoredFrom": "E:\\wagecraft-backup-2026-11-01_0800.db",
                    "preRestoreBackup": "C:\\data\\backups\\wagecraft-backup-2026-11-02_1730.db",
                })
            );
        }

        #[tokio::test]
        async fn names_the_user_without_linking_an_account_the_backup_does_not_have() {
            // The restored database's user 1 is "hr", not the "ana" who restored it.
            let (tmp, db, _) = fx::db().await;
            write_marker(tmp.path(), &marker(Some(1), "ana")).unwrap();

            finish(&db, tmp.path(), now()).await.unwrap();

            let rows = restores(&db).await;
            assert_eq!(rows[0].actor_user_id, None);
            assert_eq!(rows[0].actor_username, "ana");
            let created_by: Option<i64> =
                sqlx::query_scalar("SELECT created_by FROM backup_history")
                    .fetch_one(&db)
                    .await
                    .unwrap();
            assert_eq!(created_by, None);
        }
    }

    mod run {
        use super::*;

        fn ana() -> Actor<'static> {
            Actor {
                user_id: Some(1),
                username: "hr",
            }
        }

        async fn company(db: &SqlitePool) -> String {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'company_name'")
                .fetch_one(db)
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn back_up_change_restore_and_the_original_data_is_back() {
            let tmp = tempfile::tempdir().unwrap();
            let data = tmp.path();
            let path = data.join(DB_FILE);
            let db = crate::db::open(&path).await.unwrap();
            let mut conn = db.acquire().await.unwrap();
            crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
                .await
                .unwrap();
            drop(conn);
            sql(
                &db,
                "INSERT INTO settings (key, value) VALUES ('company_name', 'Acme')",
            )
            .await;
            let file = backup_of(&db, &data.join("usb")).await;
            sql(
                &db,
                "UPDATE settings SET value = 'Changed' WHERE key = 'company_name'",
            )
            .await;
            let backups_dir = data.join("backups");
            let at = Utc.with_ymd_and_hms(2026, 11, 2, 9, 30, 0).unwrap();

            run(&db, data, &backups_dir, &file, newest(), ana(), local(), at)
                .await
                .expect("restore");

            assert!(db.is_closed());
            // What the app does when it starts again.
            let db = crate::db::open(&path).await.unwrap();
            assert!(finish(&db, data, at).await.unwrap());
            assert_eq!(company(&db).await, "Acme");
            let pre: String =
                sqlx::query_scalar("SELECT path FROM backup_history WHERE kind = 'PRE_RESTORE'")
                    .fetch_one(&db)
                    .await
                    .unwrap();
            // The pre-restore backup still holds the data that was replaced.
            let before = crate::db::open(Path::new(&pre)).await.unwrap();
            assert_eq!(company(&before).await, "Changed");
            assert!(pre.starts_with(&*backups_dir.to_string_lossy()));
            let actions = audit_rows::actions(&db).await;
            assert_eq!(actions.last().map(String::as_str), Some("backup.restore"));
        }

        #[tokio::test]
        async fn a_backup_from_an_older_version_is_brought_up_to_date() {
            let tmp = tempfile::tempdir().unwrap();
            let data = tmp.path();
            let old_file = data.join("old.db");
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&old_file)
                .create_if_missing(true);
            let old = SqlitePool::connect_with(options).await.unwrap();
            let all = sqlx::migrate!("./migrations");
            let early = sqlx::migrate::Migrator {
                migrations: std::borrow::Cow::Owned(all.migrations[..3].to_vec()),
                ..all
            };
            early.run(&old).await.unwrap();
            old.close().await;
            let db = crate::db::open(&data.join(DB_FILE)).await.unwrap();
            let mut conn = db.acquire().await.unwrap();
            crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
                .await
                .unwrap();
            drop(conn);
            let at = Utc.with_ymd_and_hms(2026, 11, 2, 9, 30, 0).unwrap();

            run(
                &db,
                data,
                &data.join("backups"),
                &old_file,
                newest(),
                ana(),
                local(),
                at,
            )
            .await
            .expect("restore");

            let db = crate::db::open(&data.join(DB_FILE)).await.unwrap();
            assert!(finish(&db, data, at).await.unwrap());
            let applied: i64 = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
                .fetch_one(&db)
                .await
                .unwrap();
            assert_eq!(applied, newest());
        }

        #[tokio::test]
        async fn a_bad_file_changes_nothing() {
            let (tmp, db, _) = fx::db().await;
            let file = tmp.path().join("notes.db");
            std::fs::write(&file, "not a database ".repeat(300)).unwrap();
            let at = Utc.with_ymd_and_hms(2026, 11, 2, 9, 30, 0).unwrap();

            let e = run(
                &db,
                tmp.path(),
                &tmp.path().join("backups"),
                &file,
                newest(),
                ana(),
                local(),
                at,
            )
            .await
            .unwrap_err();

            assert_eq!(conflict(e), NOT_WAGECRAFT);
            assert!(!db.is_closed());
            assert!(
                !tmp.path().join("backups").exists(),
                "no pre-restore backup either"
            );
            assert!(!tmp.path().join(MARKER).exists());
        }
    }
}
