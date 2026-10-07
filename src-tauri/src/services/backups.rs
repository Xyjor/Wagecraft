//! Database backups (plan §6.7, ADR-009). `VACUUM INTO` writes a clean, consistent copy
//! while the app keeps running; a plain file copy is unsafe in WAL mode. Restoring one is
//! in `restore.rs`.

use crate::audit::{self, Actor, Entry};
use crate::domain::backup::BackupEntry;
use crate::error::AppError;
use crate::time;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

/// How many daily backups to keep (plan §6.7). Becomes a setting later.
pub const KEEP_DAILY: usize = 14;

/// Nobody starts the daily backup; audit entries name the app instead.
const SYSTEM: Actor<'static> = Actor {
    user_id: None,
    username: "system",
};

/// A backup just written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    pub id: i64,
    pub path: PathBuf,
    pub size_bytes: i64,
}

/// Writes `wagecraft-backup-2026-10-16_1730.db` (named by `local`, the PC's clock) into
/// `dir`, creating it if needed, and records it in the backup history. A second backup in
/// the same minute gets `-2`, `-3` and so on, so nothing is overwritten.
pub async fn create(
    db: &SqlitePool,
    dir: &Path,
    kind: &str,
    created_by: Option<i64>,
    local: NaiveDateTime,
    created_at: &str,
) -> Result<Backup, AppError> {
    std::fs::create_dir_all(dir).map_err(anyhow::Error::from)?;
    let stem = format!("wagecraft-backup-{}", local.format("%Y-%m-%d_%H%M"));
    let path = (1..)
        .map(|n| match n {
            1 => dir.join(format!("{stem}.db")),
            n => dir.join(format!("{stem}-{n}.db")),
        })
        .find(|p| !p.exists())
        .expect("some suffix is free");
    let text = path.to_string_lossy().into_owned();
    sqlx::query("VACUUM INTO ?").bind(&text).execute(db).await?;
    let size_bytes = std::fs::metadata(&path).map_err(anyhow::Error::from)?.len() as i64;
    let id = sqlx::query(
        "INSERT INTO backup_history (path, size_bytes, kind, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&text)
    .bind(size_bytes)
    .bind(kind)
    .bind(created_by)
    .bind(created_at)
    .execute(db)
    .await?
    .last_insert_rowid();
    Ok(Backup {
        id,
        path,
        size_bytes,
    })
}

/// Every backup on record, newest first.
pub async fn history(db: &SqlitePool) -> Result<Vec<BackupEntry>, AppError> {
    let rows: Vec<(i64, String, i64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT b.id, b.path, b.size_bytes, b.kind, u.username, b.created_at \
         FROM backup_history b LEFT JOIN users u ON u.id = b.created_by \
         ORDER BY b.created_at DESC, b.id DESC",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, path, size_bytes, kind, created_by_name, created_at)| BackupEntry {
                on_disk: Path::new(&path).is_file(),
                id,
                path,
                size_bytes,
                kind,
                created_by_name,
                created_at,
            },
        )
        .collect())
}

/// Admin's **Back up now**: a MANUAL backup into `dir`, which Admin picked.
pub async fn back_up_now(
    db: &SqlitePool,
    actor: Actor<'_>,
    dir: &Path,
    local: NaiveDateTime,
    now: DateTime<Utc>,
) -> Result<Backup, AppError> {
    let backup = create(db, dir, "MANUAL", actor.user_id, local, &time::to_db(now)).await?;
    audit_backup(db, actor, &backup, "MANUAL", now).await?;
    Ok(backup)
}

/// Run at app start: takes an AUTO backup if the last one is 24 hours old or there is
/// none, then keeps only the newest `keep`. Returns the backup it took, if any.
pub async fn daily(
    db: &SqlitePool,
    dir: &Path,
    keep: usize,
    local: NaiveDateTime,
    now: DateTime<Utc>,
) -> Result<Option<Backup>, AppError> {
    let last: Option<String> =
        sqlx::query_scalar("SELECT MAX(created_at) FROM backup_history WHERE kind = 'AUTO'")
            .fetch_one(db)
            .await?;
    let due = match last.as_deref().and_then(time::from_db) {
        Some(last) => now - last >= Duration::hours(24),
        None => true,
    };
    if !due {
        return Ok(None);
    }
    let backup = create(db, dir, "AUTO", None, local, &time::to_db(now)).await?;
    audit_backup(db, SYSTEM, &backup, "AUTO", now).await?;
    prune_daily(db, keep).await?;
    Ok(Some(backup))
}

async fn audit_backup(
    db: &SqlitePool,
    actor: Actor<'_>,
    backup: &Backup,
    kind: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let entry = Entry {
        action: "backup.create",
        entity: Some(("backup", backup.id)),
        before: None,
        after: Some(json!({
            "kind": kind,
            "path": backup.path.to_string_lossy(),
            "sizeBytes": backup.size_bytes,
        })),
    };
    let mut conn = db.acquire().await?;
    audit::record(&mut conn, now, actor, entry).await?;
    Ok(())
}

/// Deletes all but the newest `keep` daily backups, files first. Other kinds are never
/// pruned: Admin chose where manual ones go, and pre-post ones are each payroll's undo.
async fn prune_daily(db: &SqlitePool, keep: usize) -> Result<(), AppError> {
    let old: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, path FROM backup_history WHERE kind = 'AUTO' \
         ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?",
    )
    .bind(keep as i64)
    .fetch_all(db)
    .await?;
    for (id, path) in old {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                // Keep the row so the file still shows up and gets another try tomorrow.
                log::warn!("couldn't delete old backup {path}: {e}");
                continue;
            }
        }
        sqlx::query("DELETE FROM backup_history WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    async fn db(dir: &Path) -> SqlitePool {
        crate::db::open(&dir.join("test.db")).await.expect("open")
    }

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(h, m, 30)
            .unwrap()
    }

    #[tokio::test]
    async fn writes_a_named_copy_that_opens_with_the_same_data() {
        let tmp = tempfile::tempdir().unwrap();
        let db = db(tmp.path()).await;
        sqlx::query("INSERT INTO settings (key, value) VALUES ('company_name', 'Acme')")
            .execute(&db)
            .await
            .unwrap();
        let dir = tmp.path().join("backups");

        let b = create(
            &db,
            &dir,
            "PRE_POST",
            None,
            at(17, 30),
            "2026-11-02T09:30:30Z",
        )
        .await
        .expect("backup");
        assert_eq!(b.path, dir.join("wagecraft-backup-2026-11-02_1730.db"));
        assert_eq!(
            b.size_bytes,
            std::fs::metadata(&b.path).unwrap().len() as i64
        );
        assert!(b.size_bytes > 0);

        let copy = crate::db::open(&b.path).await.expect("open the copy");
        let name: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'company_name'")
                .fetch_one(&copy)
                .await
                .unwrap();
        assert_eq!(name, "Acme");

        let row: (String, i64, String, String) =
            sqlx::query_as("SELECT path, size_bytes, kind, created_at FROM backup_history")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(
            row,
            (
                b.path.to_string_lossy().into_owned(),
                b.size_bytes,
                "PRE_POST".into(),
                "2026-11-02T09:30:30Z".into()
            )
        );
    }

    #[tokio::test]
    async fn never_overwrites_a_backup_from_the_same_minute() {
        let tmp = tempfile::tempdir().unwrap();
        let db = db(tmp.path()).await;
        let dir = tmp.path().join("backups");
        let stamp = "2026-11-02T09:30:30Z";
        let first = create(&db, &dir, "MANUAL", None, at(17, 30), stamp)
            .await
            .unwrap();
        let second = create(&db, &dir, "MANUAL", None, at(17, 30), stamp)
            .await
            .unwrap();
        let third = create(&db, &dir, "MANUAL", None, at(17, 30), stamp)
            .await
            .unwrap();
        let names: Vec<_> = [first, second, third]
            .iter()
            .map(|b| b.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "wagecraft-backup-2026-11-02_1730.db",
                "wagecraft-backup-2026-11-02_1730-2.db",
                "wagecraft-backup-2026-11-02_1730-3.db",
            ]
        );
    }

    fn utc(day: u32, h: u32) -> DateTime<Utc> {
        use chrono::TimeZone;
        Utc.with_ymd_and_hms(2026, 11, day, h, 0, 0).unwrap()
    }

    fn local(day: u32, h: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, day)
            .unwrap()
            .and_hms_opt(h, 0, 0)
            .unwrap()
    }

    async fn admin(db: &SqlitePool) -> i64 {
        let mut conn = db.acquire().await.unwrap();
        crate::repositories::users::insert(&mut conn, "ana", "x", "ADMIN", false)
            .await
            .unwrap()
    }

    async fn kinds(db: &SqlitePool) -> Vec<String> {
        sqlx::query_scalar("SELECT kind FROM backup_history ORDER BY id")
            .fetch_all(db)
            .await
            .unwrap()
    }

    mod history {
        use super::*;

        #[tokio::test]
        async fn lists_newest_first_with_who_made_it_and_whether_the_file_is_still_there() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let ana = admin(&db).await;
            let dir = tmp.path().join("backups");
            let old = create(&db, &dir, "AUTO", None, local(1, 8), "2026-11-01T00:00:00Z")
                .await
                .unwrap();
            let new = create(
                &db,
                &dir,
                "MANUAL",
                Some(ana),
                local(2, 8),
                "2026-11-02T00:00:00Z",
            )
            .await
            .unwrap();
            // Inserted out of order, so the order must come from created_at.
            create(
                &db,
                &dir,
                "PRE_POST",
                Some(ana),
                local(1, 9),
                "2026-11-01T01:00:00Z",
            )
            .await
            .unwrap();
            std::fs::remove_file(&old.path).unwrap();

            let list = history(&db).await.unwrap();

            let summary: Vec<_> = list
                .iter()
                .map(|b| (b.kind.as_str(), b.created_by_name.as_deref(), b.on_disk))
                .collect();
            assert_eq!(
                summary,
                [
                    ("MANUAL", Some("ana"), true),
                    ("PRE_POST", Some("ana"), true),
                    ("AUTO", None, false),
                ]
            );
            assert_eq!(
                list[0],
                BackupEntry {
                    id: new.id,
                    path: new.path.to_string_lossy().into_owned(),
                    size_bytes: new.size_bytes,
                    kind: "MANUAL".into(),
                    created_by_name: Some("ana".into()),
                    created_at: "2026-11-02T00:00:00Z".into(),
                    on_disk: true,
                }
            );
        }
    }

    mod back_up_now {
        use super::*;
        use crate::audit::test_support as audit_rows;

        #[tokio::test]
        async fn writes_a_manual_backup_and_audits_it() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let ana = admin(&db).await;
            let usb = tmp.path().join("usb");
            let actor = Actor {
                user_id: Some(ana),
                username: "ana",
            };

            let b = back_up_now(&db, actor, &usb, local(2, 17), utc(2, 9))
                .await
                .unwrap();

            assert_eq!(b.path, usb.join("wagecraft-backup-2026-11-02_1700.db"));
            assert!(b.path.exists());
            assert_eq!(kinds(&db).await, ["MANUAL"]);
            let created_by: Option<i64> =
                sqlx::query_scalar("SELECT created_by FROM backup_history")
                    .fetch_one(&db)
                    .await
                    .unwrap();
            assert_eq!(created_by, Some(ana));
            let rows = audit_rows::all(&db).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].action, "backup.create");
            assert_eq!(rows[0].actor_username, "ana");
            assert_eq!(rows[0].entity_type.as_deref(), Some("backup"));
            assert_eq!(rows[0].entity_id, Some(b.id));
            let after: serde_json::Value =
                serde_json::from_str(rows[0].after_json.as_deref().unwrap()).unwrap();
            assert_eq!(
                after,
                serde_json::json!({
                    "kind": "MANUAL",
                    "path": b.path.to_string_lossy(),
                    "sizeBytes": b.size_bytes,
                })
            );
        }
    }

    mod daily {
        use super::*;
        use crate::audit::test_support as audit_rows;

        #[tokio::test]
        async fn takes_the_first_one_and_audits_it_as_the_system() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let dir = tmp.path().join("backups");

            let b = daily(&db, &dir, 14, local(2, 8), utc(2, 0))
                .await
                .unwrap()
                .expect("a backup");

            assert!(b.path.exists());
            assert_eq!(kinds(&db).await, ["AUTO"]);
            let rows = audit_rows::all(&db).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].action, "backup.create");
            assert_eq!(rows[0].actor_user_id, None);
            assert_eq!(rows[0].actor_username, "system");
        }

        #[tokio::test]
        async fn waits_24_hours_after_the_last_daily_backup() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let dir = tmp.path().join("backups");
            daily(&db, &dir, 14, local(2, 8), utc(2, 0)).await.unwrap();

            let early = daily(&db, &dir, 14, local(2, 23), utc(2, 23))
                .await
                .unwrap();
            let due = daily(&db, &dir, 14, local(3, 8), utc(3, 0)).await.unwrap();

            assert!(early.is_none());
            assert!(due.is_some());
            assert_eq!(kinds(&db).await, ["AUTO", "AUTO"]);
        }

        #[tokio::test]
        async fn other_kinds_of_backup_do_not_count_as_the_daily_one() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let dir = tmp.path().join("backups");
            for kind in ["MANUAL", "PRE_POST", "PRE_RESTORE"] {
                create(&db, &dir, kind, None, local(2, 7), "2026-11-02T00:00:00Z")
                    .await
                    .unwrap();
            }

            let b = daily(&db, &dir, 14, local(2, 8), utc(2, 1)).await.unwrap();

            assert!(b.is_some());
        }

        #[tokio::test]
        async fn keeps_only_the_newest_daily_backups_and_deletes_the_rest() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let dir = tmp.path().join("backups");
            let manual = create(
                &db,
                &dir,
                "MANUAL",
                None,
                local(1, 7),
                "2026-11-01T00:00:00Z",
            )
            .await
            .unwrap();
            let posted = create(
                &db,
                &dir,
                "PRE_POST",
                None,
                local(1, 7),
                "2026-11-01T00:00:00Z",
            )
            .await
            .unwrap();
            let mut taken = Vec::new();
            for day in 2..=5 {
                let b = daily(&db, &dir, 2, local(day, 8), utc(day, 0))
                    .await
                    .unwrap()
                    .expect("due");
                taken.push(b);
            }

            let left: Vec<String> = sqlx::query_scalar(
                "SELECT path FROM backup_history WHERE kind = 'AUTO' ORDER BY id",
            )
            .fetch_all(&db)
            .await
            .unwrap();
            let newest: Vec<String> = taken[2..]
                .iter()
                .map(|b| b.path.to_string_lossy().into_owned())
                .collect();
            assert_eq!(left, newest);
            assert!(!taken[0].path.exists());
            assert!(!taken[1].path.exists());
            assert!(taken[2].path.exists() && taken[3].path.exists());
            // Manual and pre-post backups are never pruned.
            assert!(manual.path.exists() && posted.path.exists());
            assert_eq!(kinds(&db).await, ["MANUAL", "PRE_POST", "AUTO", "AUTO"]);
        }

        #[tokio::test]
        async fn a_daily_backup_someone_already_deleted_is_still_pruned() {
            let tmp = tempfile::tempdir().unwrap();
            let db = db(tmp.path()).await;
            let dir = tmp.path().join("backups");
            let first = daily(&db, &dir, 1, local(2, 8), utc(2, 0))
                .await
                .unwrap()
                .unwrap();
            std::fs::remove_file(&first.path).unwrap();

            daily(&db, &dir, 1, local(3, 8), utc(3, 0)).await.unwrap();

            assert_eq!(kinds(&db).await, ["AUTO"]);
        }
    }
}
