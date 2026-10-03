//! Sign-in rules: first-run setup, login with lockout, and password changes (plan §6.1).
//! Functions take the current time as a parameter so tests can move the clock.

use crate::audit::{self, Actor, Entry};
use crate::auth::{password, permissions::Role};
use crate::error::{AppError, FieldError};
use crate::repositories::users::{self, UserRow};
use crate::time;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::OnceLock;

const MAX_FAILED_ATTEMPTS: i64 = 5;
const LOCK_MINUTES: i64 = 15;
const MIN_PASSWORD_LEN: usize = 10;

/// A signed-in user as the rest of the app sees it. Never carries the password hash.
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub employee_id: Option<i64>,
    pub must_change_password: bool,
    pub theme: String,
}

impl TryFrom<UserRow> for User {
    type Error = AppError;

    fn try_from(r: UserRow) -> Result<Self, AppError> {
        let role = Role::from_db(&r.role)
            .ok_or_else(|| anyhow::anyhow!("user {} has unknown role {:?}", r.id, r.role))?;
        Ok(User {
            id: r.id,
            username: r.username,
            role,
            employee_id: r.employee_id,
            must_change_password: r.must_change_password,
            theme: r.theme,
        })
    }
}

/// Returns the new failed count and, when the account locks, the unlock time.
pub fn after_failed_login(failed: i64, now: DateTime<Utc>) -> (i64, Option<DateTime<Utc>>) {
    let failed = failed + 1;
    if failed >= MAX_FAILED_ATTEMPTS {
        (0, Some(now + chrono::Duration::minutes(LOCK_MINUTES)))
    } else {
        (failed, None)
    }
}

/// Why a password is not acceptable, or `None` if it is.
pub fn password_problem(username: &str, password: &str) -> Option<&'static str> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        Some("Use at least 10 characters")
    } else if password.eq_ignore_ascii_case(username) {
        Some("Don't use your username as your password")
    } else {
        None
    }
}

fn username_problem(username: &str) -> Option<&'static str> {
    let ok_len = (3..=32).contains(&username.chars().count());
    let ok_chars = username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok_len && ok_chars {
        None
    } else {
        Some("Use 3 to 32 letters, numbers, dots, dashes or underscores")
    }
}

fn field(name: &str, message: &str) -> FieldError {
    FieldError {
        field: name.to_string(),
        message: message.to_string(),
    }
}

/// True until the first user exists. The setup wizard shows only while this is true.
pub async fn needs_setup(db: &SqlitePool) -> Result<bool, AppError> {
    let mut conn = db.acquire().await?;
    Ok(users::count(&mut conn).await? == 0)
}

/// Creates the company name setting and the first Admin. Refused once any user exists.
pub async fn create_admin(
    db: &SqlitePool,
    company_name: &str,
    username: &str,
    password: &str,
) -> Result<(), AppError> {
    let company_name = company_name.trim();
    let username = username.trim();
    let mut errors = Vec::new();
    if company_name.is_empty() || company_name.chars().count() > 100 {
        errors.push(field(
            "companyName",
            "Enter the company name (up to 100 characters)",
        ));
    }
    if let Some(m) = username_problem(username) {
        errors.push(field("username", m));
    }
    if let Some(m) = password_problem(username, password) {
        errors.push(field("password", m));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let hash = hash_blocking(password.to_string()).await?;
    let mut tx = db.begin().await?;
    if users::count(&mut tx).await? > 0 {
        return Err(AppError::Forbidden);
    }
    let id = users::insert(&mut tx, username, &hash, Role::Admin.as_db(), false).await?;
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('company_name', ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(company_name)
    .execute(&mut *tx)
    .await?;
    let actor = Actor {
        user_id: Some(id),
        username,
    };
    let entry = Entry {
        action: "auth.setup",
        entity: Some(("user", id)),
        after: Some(json!({
            "username": username,
            "role": Role::Admin.as_db(),
            "companyName": company_name,
        })),
        ..Default::default()
    };
    audit::record(&mut tx, Utc::now(), actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Checks a username and password. Every failure gives the same error, so the screen
/// never reveals which usernames exist or that an account is locked.
pub async fn login(
    db: &SqlitePool,
    username: &str,
    password: &str,
    now: DateTime<Utc>,
) -> Result<User, AppError> {
    let typed = username.trim();
    // Before sign-in there is no acting user, only the name someone typed.
    let stranger = Actor {
        user_id: None,
        username: typed,
    };

    let Some(row) = users::by_username(db, typed).await? else {
        // Spend the same time as a real check, so timing doesn't reveal the username.
        verify_blocking(password.to_string(), dummy_hash().to_string()).await?;
        let mut conn = db.acquire().await?;
        audit::record(&mut conn, now, stranger, failed_entry(None, "unknown_user")).await?;
        return Err(AppError::InvalidCredentials);
    };

    let locked = row
        .locked_until
        .as_deref()
        .and_then(time::from_db)
        .is_some_and(|until| until > now);
    if !row.is_active || locked {
        verify_blocking(password.to_string(), dummy_hash().to_string()).await?;
        let reason = if row.is_active { "locked" } else { "inactive" };
        let mut conn = db.acquire().await?;
        audit::record(&mut conn, now, stranger, failed_entry(Some(row.id), reason)).await?;
        return Err(AppError::InvalidCredentials);
    }

    if !verify_blocking(password.to_string(), row.password_hash.clone()).await? {
        let (failed, lock) = after_failed_login(row.failed_attempts, now);
        let lock = lock.map(time::to_db);
        let mut entry = failed_entry(Some(row.id), "wrong_password");
        entry.after = Some(json!({
            "reason": "wrong_password",
            // Counts this attempt, even when the lock resets the stored count to 0.
            "failedAttempts": row.failed_attempts + 1,
            "lockedUntil": lock,
        }));
        let mut tx = db.begin().await?;
        users::record_failure(&mut tx, row.id, failed, lock.as_deref(), &time::to_db(now)).await?;
        audit::record(&mut tx, now, stranger, entry).await?;
        tx.commit().await?;
        return Err(AppError::InvalidCredentials);
    }

    let mut tx = db.begin().await?;
    users::record_success(&mut tx, row.id, &time::to_db(now)).await?;
    let actor = Actor {
        user_id: Some(row.id),
        username: &row.username,
    };
    let entry = Entry {
        action: "auth.login",
        entity: Some(("user", row.id)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    User::try_from(row)
}

fn failed_entry(user_id: Option<i64>, reason: &str) -> Entry<'static> {
    Entry {
        action: "auth.login_failed",
        entity: user_id.map(|id| ("user", id)),
        after: Some(json!({ "reason": reason })),
        ..Default::default()
    }
}

/// Records the sign-out. The caller has already cleared the in-memory session.
pub async fn logout(db: &SqlitePool, actor: Actor<'_>, now: DateTime<Utc>) -> Result<(), AppError> {
    let entry = Entry {
        action: "auth.logout",
        entity: actor.user_id.map(|id| ("user", id)),
        ..Default::default()
    };
    let mut conn = db.acquire().await?;
    audit::record(&mut conn, now, actor, entry).await?;
    Ok(())
}

pub async fn find_user(db: &SqlitePool, id: i64) -> Result<User, AppError> {
    users::by_id(db, id)
        .await?
        .ok_or(AppError::NotFound("User"))?
        .try_into()
}

/// Changes the signed-in user's own password and clears `must_change_password`.
pub async fn change_password(
    db: &SqlitePool,
    user_id: i64,
    current: &str,
    new: &str,
) -> Result<(), AppError> {
    let row = users::by_id(db, user_id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    if !verify_blocking(current.to_string(), row.password_hash.clone()).await? {
        return Err(AppError::Validation(vec![field(
            "currentPassword",
            "Your current password is not correct",
        )]));
    }
    let problem = password_problem(&row.username, new).or(if new == current {
        Some("Choose a password different from your current one")
    } else {
        None
    });
    if let Some(m) = problem {
        return Err(AppError::Validation(vec![field("newPassword", m)]));
    }
    let hash = hash_blocking(new.to_string()).await?;
    let mut tx = db.begin().await?;
    users::set_password(&mut tx, user_id, &hash).await?;
    let actor = Actor {
        user_id: Some(user_id),
        username: &row.username,
    };
    let entry = Entry {
        action: "auth.password_change",
        entity: Some(("user", user_id)),
        ..Default::default()
    };
    audit::record(&mut tx, Utc::now(), actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

// Argon2 takes ~100 ms on purpose; run it off the async threads so the app stays responsive.
async fn hash_blocking(password: String) -> Result<String, AppError> {
    let hashed = tauri::async_runtime::spawn_blocking(move || password::hash(&password))
        .await
        .map_err(|e| anyhow::anyhow!("hash task failed: {e}"))??;
    Ok(hashed)
}

async fn verify_blocking(password: String, stored: String) -> Result<bool, AppError> {
    let ok = tauri::async_runtime::spawn_blocking(move || password::verify(&password, &stored))
        .await
        .map_err(|e| anyhow::anyhow!("verify task failed: {e}"))?;
    Ok(ok)
}

fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| password::hash("not a real password").expect("hash dummy password"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use chrono::{Duration, TimeZone};

    const PW: &str = "correct horse battery";

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, 8, 0, 0).unwrap()
    }

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        (dir, pool)
    }

    async fn with_admin() -> (tempfile::TempDir, SqlitePool) {
        let (dir, pool) = db().await;
        create_admin(&pool, "Rojyx Inc.", "admin", PW)
            .await
            .expect("setup");
        (dir, pool)
    }

    fn field_errors(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn fifth_failure_locks_for_15_minutes() {
        let now = t0();
        assert_eq!(after_failed_login(3, now), (4, None));
        assert_eq!(
            after_failed_login(4, now),
            (0, Some(now + Duration::minutes(15)))
        );
    }

    #[tokio::test]
    async fn setup_runs_once_and_stores_the_company_name() {
        let (_dir, pool) = db().await;
        assert!(needs_setup(&pool).await.expect("status"));

        create_admin(&pool, "Rojyx Inc.", "admin", PW)
            .await
            .expect("setup");

        assert!(!needs_setup(&pool).await.expect("status"));
        let company: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'company_name'")
                .fetch_one(&pool)
                .await
                .expect("company");
        assert_eq!(company, "Rojyx Inc.");
        assert!(matches!(
            create_admin(&pool, "Other", "admin2", PW).await,
            Err(AppError::Forbidden)
        ));
    }

    #[tokio::test]
    async fn setup_validates_its_fields() {
        let (_dir, pool) = db().await;
        let err = create_admin(&pool, "  ", "a", "short").await.unwrap_err();
        assert_eq!(
            field_errors(err),
            vec!["companyName", "username", "password"]
        );
        assert!(needs_setup(&pool).await.expect("status"));
    }

    #[tokio::test]
    async fn admin_signs_in_and_gets_the_admin_role() {
        let (_dir, pool) = with_admin().await;
        let user = login(&pool, "ADMIN", PW, t0()).await.expect("login");
        assert_eq!(user.username, "admin");
        assert_eq!(user.role, Role::Admin);
        assert!(!user.must_change_password);
    }

    #[tokio::test]
    async fn wrong_password_and_unknown_user_look_the_same() {
        let (_dir, pool) = with_admin().await;
        let wrong = login(&pool, "admin", "wrong password!", t0())
            .await
            .unwrap_err();
        let unknown = login(&pool, "nobody", PW, t0()).await.unwrap_err();
        assert!(matches!(wrong, AppError::InvalidCredentials));
        assert_eq!(wrong.to_string(), unknown.to_string());
    }

    #[tokio::test]
    async fn five_failures_lock_the_account_for_15_minutes() {
        let (_dir, pool) = with_admin().await;
        for _ in 0..5 {
            let _ = login(&pool, "admin", "wrong password!", t0()).await;
        }
        // Locked: even the right password fails, with the same message.
        let locked = login(&pool, "admin", PW, t0() + Duration::minutes(14))
            .await
            .unwrap_err();
        assert!(matches!(locked, AppError::InvalidCredentials));
        // After the lock runs out, the right password works again.
        assert!(login(&pool, "admin", PW, t0() + Duration::minutes(16))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn success_resets_the_failure_count() {
        let (_dir, pool) = with_admin().await;
        for _ in 0..4 {
            let _ = login(&pool, "admin", "wrong password!", t0()).await;
        }
        login(&pool, "admin", PW, t0()).await.expect("login");
        // Four more failures must not lock, because the count started again.
        for _ in 0..4 {
            let _ = login(&pool, "admin", "wrong password!", t0()).await;
        }
        assert!(login(&pool, "admin", PW, t0()).await.is_ok());
    }

    #[tokio::test]
    async fn inactive_user_cannot_sign_in() {
        let (_dir, pool) = with_admin().await;
        sqlx::query("UPDATE users SET is_active = 0 WHERE username = 'admin'")
            .execute(&pool)
            .await
            .expect("deactivate");
        assert!(matches!(
            login(&pool, "admin", PW, t0()).await,
            Err(AppError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn change_password_checks_the_current_one_and_the_rules() {
        let (_dir, pool) = with_admin().await;
        let id = login(&pool, "admin", PW, t0()).await.expect("login").id;

        let wrong = change_password(&pool, id, "not it", "a brand new password")
            .await
            .unwrap_err();
        assert_eq!(field_errors(wrong), vec!["currentPassword"]);

        let same = change_password(&pool, id, PW, PW).await.unwrap_err();
        assert_eq!(field_errors(same), vec!["newPassword"]);

        change_password(&pool, id, PW, "a brand new password")
            .await
            .expect("change");
        assert!(login(&pool, "admin", "a brand new password", t0())
            .await
            .is_ok());
        assert!(login(&pool, "admin", PW, t0()).await.is_err());
    }

    #[tokio::test]
    async fn changing_the_password_clears_must_change() {
        let (_dir, pool) = with_admin().await;
        sqlx::query("UPDATE users SET must_change_password = 1")
            .execute(&pool)
            .await
            .expect("flag");
        let user = login(&pool, "admin", PW, t0()).await.expect("login");
        assert!(user.must_change_password);

        change_password(&pool, user.id, PW, "a brand new password")
            .await
            .expect("change");
        let user = find_user(&pool, user.id).await.expect("find");
        assert!(!user.must_change_password);
    }

    // ── Audit trail (plan §6.1: every auth event leaves an entry) ──

    fn after(row: &audit_rows::Row) -> serde_json::Value {
        serde_json::from_str(row.after_json.as_deref().expect("after_json")).expect("json")
    }

    #[tokio::test]
    async fn setup_is_audited_as_the_new_admin() {
        let (_dir, pool) = with_admin().await;
        let rows = audit_rows::all(&pool).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "auth.setup");
        assert_eq!(rows[0].actor_user_id, Some(1));
        assert_eq!(rows[0].actor_username, "admin");
        assert_eq!(rows[0].entity_type.as_deref(), Some("user"));
        assert_eq!(rows[0].entity_id, Some(1));
        assert_eq!(after(&rows[0])["companyName"], "Rojyx Inc.");
        assert_eq!(after(&rows[0])["role"], "ADMIN");
    }

    #[tokio::test]
    async fn sign_in_is_audited() {
        let (_dir, pool) = with_admin().await;
        login(&pool, "Admin", PW, t0()).await.expect("login");

        let rows = audit_rows::all(&pool).await;
        let last = rows.last().expect("entry");
        assert_eq!(last.action, "auth.login");
        assert_eq!(last.at, "2026-10-05T08:00:00Z");
        assert_eq!(last.actor_user_id, Some(1));
        assert_eq!(last.actor_username, "admin");
    }

    #[tokio::test]
    async fn wrong_password_is_audited_with_the_count() {
        let (_dir, pool) = with_admin().await;
        let _ = login(&pool, "admin", "wrong password!", t0()).await;

        let rows = audit_rows::all(&pool).await;
        let last = rows.last().expect("entry");
        assert_eq!(last.action, "auth.login_failed");
        // Nobody is signed in yet, so there is no acting user, only the name typed.
        assert_eq!(last.actor_user_id, None);
        assert_eq!(last.actor_username, "admin");
        assert_eq!(last.entity_id, Some(1));
        assert_eq!(after(last)["failedAttempts"], 1);
        assert_eq!(after(last)["lockedUntil"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn the_locking_failure_records_when_the_lock_ends() {
        let (_dir, pool) = with_admin().await;
        for _ in 0..5 {
            let _ = login(&pool, "admin", "wrong password!", t0()).await;
        }
        let rows = audit_rows::all(&pool).await;
        assert_eq!(
            after(rows.last().unwrap())["lockedUntil"],
            "2026-10-05T08:15:00Z"
        );
    }

    #[tokio::test]
    async fn unknown_users_and_locked_accounts_are_audited_too() {
        let (_dir, pool) = with_admin().await;
        let _ = login(&pool, "ghost", PW, t0()).await;
        sqlx::query("UPDATE users SET is_active = 0")
            .execute(&pool)
            .await
            .expect("deactivate");
        let _ = login(&pool, "admin", PW, t0()).await;

        let rows = audit_rows::all(&pool).await;
        let [.., ghost, inactive] = rows.as_slice() else {
            panic!("expected two entries")
        };
        assert_eq!(ghost.action, "auth.login_failed");
        assert_eq!(ghost.actor_username, "ghost");
        assert_eq!(ghost.entity_id, None);
        assert_eq!(after(ghost)["reason"], "unknown_user");
        assert_eq!(inactive.action, "auth.login_failed");
        assert_eq!(after(inactive)["reason"], "inactive");
    }

    #[tokio::test]
    async fn password_change_and_sign_out_are_audited() {
        let (_dir, pool) = with_admin().await;
        let user = login(&pool, "admin", PW, t0()).await.expect("login");
        change_password(&pool, user.id, PW, "a brand new password")
            .await
            .expect("change");
        let actor = Actor {
            user_id: Some(user.id),
            username: &user.username,
        };
        logout(&pool, actor, t0()).await.expect("logout");

        let actions = audit_rows::actions(&pool).await;
        assert_eq!(
            actions,
            [
                "auth.setup",
                "auth.login",
                "auth.password_change",
                "auth.logout"
            ]
        );
    }

    #[tokio::test]
    async fn no_audit_entry_holds_a_password_or_hash() {
        let (_dir, pool) = with_admin().await;
        let _ = login(&pool, "admin", "wrong password!", t0()).await;
        let user = login(&pool, "admin", PW, t0()).await.expect("login");
        change_password(&pool, user.id, PW, "a brand new password")
            .await
            .expect("change");

        for row in audit_rows::all(&pool).await {
            let text = format!("{:?} {:?}", row.before_json, row.after_json);
            for secret in ["argon2", PW, "wrong password!", "a brand new password"] {
                assert!(!text.contains(secret), "{} leaks {secret}", row.action);
            }
        }
    }

    #[test]
    fn password_rules() {
        assert!(password_problem("admin", "short").is_some());
        assert!(password_problem("administrator", "Administrator").is_some());
        assert!(password_problem("admin", "ten chars!").is_none());
    }
}
