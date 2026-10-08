//! Settings (plan §5.3): the company profile, the idle timeout and the daily backups'
//! folder and count. Missing rows mean the defaults.

use crate::audit::{self, Actor, Entry};
use crate::domain::settings::{Settings, SettingsInput, DEFAULT_BACKUP_KEEP, DEFAULT_IDLE_MINUTES};
use crate::domain::validation;
use crate::error::AppError;
use crate::services::auth::field;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const IDLE_MINUTES: std::ops::RangeInclusive<i64> = 5..=120;
pub const BACKUP_KEEP: std::ops::RangeInclusive<i64> = 3..=60;

pub async fn load(db: &SqlitePool, default_backup_dir: &Path) -> Result<Settings, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(db)
        .await?;
    let get = |key: &str| {
        rows.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let number = |key: &str, default: i64| get(key).parse().unwrap_or(default);
    Ok(Settings {
        company_name: get(COMPANY_NAME),
        company_address: get(COMPANY_ADDRESS),
        company_tin: get(COMPANY_TIN),
        idle_timeout_minutes: number(IDLE_TIMEOUT, DEFAULT_IDLE_MINUTES),
        backup_folder: get(BACKUP_FOLDER),
        default_backup_folder: default_backup_dir.display().to_string(),
        backup_keep: number(BACKUP_KEEP_KEY, DEFAULT_BACKUP_KEEP),
    })
}

const COMPANY_NAME: &str = "company_name";
const COMPANY_ADDRESS: &str = "company_address";
const COMPANY_TIN: &str = "company_tin";
const IDLE_TIMEOUT: &str = "idle_timeout_minutes";
const BACKUP_FOLDER: &str = "backup_folder";
const BACKUP_KEEP_KEY: &str = "backup_keep";

/// Saves every setting and audits the ones that changed as `settings.update`.
pub async fn update(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: SettingsInput,
    default_backup_dir: &Path,
    now: DateTime<Utc>,
) -> Result<Settings, AppError> {
    let name = input.company_name.trim();
    let address = input.company_address.trim();
    let tin_text = input.company_tin.trim();
    let folder = input.backup_folder.trim();
    let mut errors = Vec::new();
    if name.is_empty() {
        errors.push(field("companyName", "Enter the company name"));
    } else if name.chars().count() > 100 {
        errors.push(field("companyName", "Use 100 characters or fewer"));
    }
    if address.chars().count() > 300 {
        errors.push(field("companyAddress", "Use 300 characters or fewer"));
    }
    let tin = if tin_text.is_empty() {
        Some(String::new())
    } else {
        validation::tin(tin_text)
    };
    if tin.is_none() {
        errors.push(field(
            "companyTin",
            "Enter a TIN like 123-456-789 or 123-456-789-000",
        ));
    }
    if !IDLE_MINUTES.contains(&input.idle_timeout_minutes) {
        errors.push(field(
            "idleTimeoutMinutes",
            "Choose between 5 and 120 minutes",
        ));
    }
    if !folder.is_empty() && !(Path::new(folder).is_absolute() && Path::new(folder).is_dir()) {
        errors.push(field(
            "backupFolder",
            "Choose a folder that exists on this PC",
        ));
    }
    if !BACKUP_KEEP.contains(&input.backup_keep) {
        errors.push(field("backupKeep", "Keep between 3 and 60 daily backups"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let before = load(db, default_backup_dir).await?;
    let after = Settings {
        company_name: name.to_string(),
        company_address: address.to_string(),
        company_tin: tin.unwrap_or_default(),
        idle_timeout_minutes: input.idle_timeout_minutes,
        backup_folder: folder.to_string(),
        default_backup_folder: before.default_backup_folder.clone(),
        backup_keep: input.backup_keep,
    };
    let (old, new) = changes(&before, &after);
    if new.is_empty() {
        return Ok(after);
    }
    let mut tx = db.begin().await?;
    let values = [
        (COMPANY_NAME, after.company_name.clone()),
        (COMPANY_ADDRESS, after.company_address.clone()),
        (COMPANY_TIN, after.company_tin.clone()),
        (IDLE_TIMEOUT, after.idle_timeout_minutes.to_string()),
        (BACKUP_FOLDER, after.backup_folder.clone()),
        (BACKUP_KEEP_KEY, after.backup_keep.to_string()),
    ];
    for (key, value) in values {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }
    let entry = Entry {
        action: "settings.update",
        entity: None,
        before: Some(Value::Object(old)),
        after: Some(Value::Object(new)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// The settings that differ, as before and after snapshots keyed like the screen's fields.
fn changes(before: &Settings, after: &Settings) -> (Map<String, Value>, Map<String, Value>) {
    let (Ok(Value::Object(b)), Ok(Value::Object(a))) =
        (serde_json::to_value(before), serde_json::to_value(after))
    else {
        return (Map::new(), Map::new());
    };
    let mut old = Map::new();
    let mut new = Map::new();
    for (key, value) in a {
        if b.get(&key) != Some(&value) {
            old.insert(key.clone(), b.get(&key).cloned().unwrap_or(Value::Null));
            new.insert(key, value);
        }
    }
    (old, new)
}

/// How long a session may sit idle before it ends.
pub async fn idle_timeout(db: &SqlitePool) -> Result<Duration, AppError> {
    let minutes = number(db, IDLE_TIMEOUT, DEFAULT_IDLE_MINUTES).await?;
    Ok(Duration::from_secs(minutes.unsigned_abs() * 60))
}

/// Where daily backups and the backups before posting or restoring go.
pub async fn backup_dir(db: &SqlitePool, default: &Path) -> Result<PathBuf, AppError> {
    let folder = value(db, BACKUP_FOLDER).await?.unwrap_or_default();
    Ok(if folder.is_empty() {
        default.to_path_buf()
    } else {
        PathBuf::from(folder)
    })
}

/// How many daily backups to keep.
pub async fn backup_keep(db: &SqlitePool) -> Result<usize, AppError> {
    let keep = number(db, BACKUP_KEEP_KEY, DEFAULT_BACKUP_KEEP).await?;
    Ok(usize::try_from(keep).unwrap_or(DEFAULT_BACKUP_KEEP as usize))
}

async fn value(db: &SqlitePool, key: &str) -> Result<Option<String>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(db)
            .await?,
    )
}

async fn number(db: &SqlitePool, key: &str, default: i64) -> Result<i64, AppError> {
    Ok(value(db, key)
        .await?
        .and_then(|v| v.parse().ok())
        .unwrap_or(default))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::settings::DEFAULT_BACKUP_KEEP;
    use chrono::TimeZone;

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("test.db")).await.unwrap();
        (dir, pool)
    }

    fn admin() -> Actor<'static> {
        Actor {
            user_id: None,
            username: "ana",
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 1, 0, 0).unwrap()
    }

    fn input(tmp: &Path) -> SettingsInput {
        SettingsInput {
            company_name: "  Acme Trading  ".into(),
            company_address: "12 Rizal St, Makati".into(),
            company_tin: "123-456-789-000".into(),
            idle_timeout_minutes: 30,
            backup_folder: tmp.display().to_string(),
            backup_keep: 20,
        }
    }

    fn fields(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected validation errors, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_empty_database_has_the_defaults() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");

        let s = load(&db, &default).await.unwrap();

        assert_eq!(
            s,
            Settings {
                company_name: String::new(),
                company_address: String::new(),
                company_tin: String::new(),
                idle_timeout_minutes: 15,
                backup_folder: String::new(),
                default_backup_folder: default.display().to_string(),
                backup_keep: 14,
            }
        );
        assert_eq!(
            idle_timeout(&db).await.unwrap(),
            Duration::from_secs(15 * 60)
        );
        assert_eq!(backup_dir(&db, &default).await.unwrap(), default);
        assert_eq!(
            backup_keep(&db).await.unwrap(),
            DEFAULT_BACKUP_KEEP as usize
        );
    }

    #[tokio::test]
    async fn saves_tidied_values_that_the_rest_of_the_app_then_uses() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");

        let saved = update(&db, admin(), input(tmp.path()), &default, now())
            .await
            .unwrap();

        assert_eq!(saved.company_name, "Acme Trading");
        assert_eq!(saved.company_tin, "123456789000");
        assert_eq!(load(&db, &default).await.unwrap(), saved);
        assert_eq!(
            idle_timeout(&db).await.unwrap(),
            Duration::from_secs(30 * 60)
        );
        assert_eq!(backup_dir(&db, &default).await.unwrap(), tmp.path());
        assert_eq!(backup_keep(&db).await.unwrap(), 20);
    }

    #[tokio::test]
    async fn an_empty_folder_and_tin_are_allowed() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");
        let mut i = input(tmp.path());
        i.backup_folder = "  ".into();
        i.company_tin = String::new();

        update(&db, admin(), i, &default, now()).await.unwrap();

        assert_eq!(backup_dir(&db, &default).await.unwrap(), default);
        assert_eq!(load(&db, &default).await.unwrap().company_tin, "");
    }

    #[tokio::test]
    async fn refuses_bad_values_field_by_field() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");
        let bad = SettingsInput {
            company_name: " ".into(),
            company_address: "x".repeat(301),
            company_tin: "12345".into(),
            idle_timeout_minutes: 4,
            backup_folder: tmp.path().join("missing").display().to_string(),
            backup_keep: 61,
        };

        let e = update(&db, admin(), bad, &default, now())
            .await
            .unwrap_err();

        let names: Vec<String> = fields(e).into_iter().map(|(f, _)| f).collect();
        assert_eq!(
            names,
            [
                "companyName",
                "companyAddress",
                "companyTin",
                "idleTimeoutMinutes",
                "backupFolder",
                "backupKeep"
            ]
        );
        assert_eq!(load(&db, &default).await.unwrap().idle_timeout_minutes, 15);
    }

    #[tokio::test]
    async fn the_limits_themselves_are_allowed() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");
        for (minutes, keep) in [(5, 3), (120, 60)] {
            let mut i = input(tmp.path());
            i.idle_timeout_minutes = minutes;
            i.backup_keep = keep;
            update(&db, admin(), i, &default, now())
                .await
                .expect("in range");
        }
        for (minutes, keep) in [(121, 14), (15, 2)] {
            let mut i = input(tmp.path());
            i.idle_timeout_minutes = minutes;
            i.backup_keep = keep;
            assert!(update(&db, admin(), i, &default, now()).await.is_err());
        }
    }

    #[tokio::test]
    async fn a_relative_backup_folder_is_refused() {
        let (tmp, db) = db().await;
        let mut i = input(tmp.path());
        i.backup_folder = ".".into();

        let e = update(&db, admin(), i, &tmp.path().join("b"), now())
            .await
            .unwrap_err();

        assert_eq!(fields(e)[0].0, "backupFolder");
    }

    #[tokio::test]
    async fn audits_only_what_changed() {
        let (tmp, db) = db().await;
        let default = tmp.path().join("backups");
        update(&db, admin(), input(tmp.path()), &default, now())
            .await
            .unwrap();
        let mut i = input(tmp.path());
        i.idle_timeout_minutes = 10;

        update(&db, admin(), i, &default, now()).await.unwrap();
        update(
            &db,
            admin(),
            input_with_timeout(tmp.path(), 10),
            &default,
            now(),
        )
        .await
        .unwrap();

        let rows = audit_rows::all(&db).await;
        assert_eq!(rows.len(), 2, "saving with no changes writes no entry");
        let last = rows.last().unwrap();
        assert_eq!(last.action, "settings.update");
        assert_eq!(last.actor_username, "ana");
        let before: Value = serde_json::from_str(last.before_json.as_deref().unwrap()).unwrap();
        let after: Value = serde_json::from_str(last.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(before, serde_json::json!({ "idleTimeoutMinutes": 30 }));
        assert_eq!(after, serde_json::json!({ "idleTimeoutMinutes": 10 }));
    }

    fn input_with_timeout(tmp: &Path, minutes: i64) -> SettingsInput {
        let mut i = input(tmp);
        i.idle_timeout_minutes = minutes;
        i
    }
}
