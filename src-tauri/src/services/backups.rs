//! Database backups (plan §6.7, ADR-009). `VACUUM INTO` writes a clean, consistent copy
//! while the app keeps running; a plain file copy is unsafe in WAL mode. The backup
//! screen, daily backups and restore come later; posting a pay period uses this now.

use crate::error::AppError;
use chrono::NaiveDateTime;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

/// A backup just written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
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
    sqlx::query(
        "INSERT INTO backup_history (path, size_bytes, kind, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&text)
    .bind(size_bytes)
    .bind(kind)
    .bind(created_by)
    .bind(created_at)
    .execute(db)
    .await?;
    Ok(Backup { path, size_bytes })
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
}
