//! User accounts, managed by Admins (plan §3, Appendix B "Users").
//! Every change is audited in the same transaction.

use crate::audit::{self, Actor, Entry};
use crate::auth::permissions::Role;
use crate::error::AppError;
use crate::repositories::users::{self, ListRow};
use crate::services::auth::{field, hash_blocking, password_problem, username_problem};
use crate::time;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};
use ts_rs::TS;

/// One row of the Users screen.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserSummary {
    #[ts(type = "number")]
    pub id: i64,
    pub username: String,
    pub role: Role,
    #[ts(type = "number | null")]
    pub employee_id: Option<i64>,
    pub employee_label: Option<String>,
    pub is_active: bool,
    pub must_change_password: bool,
    /// True while too many wrong passwords keep the account locked.
    pub locked: bool,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewUser {
    pub username: String,
    pub role: Role,
    /// A temporary password. The user must replace it at first sign-in.
    pub password: String,
}

pub async fn list(db: &SqlitePool, now: DateTime<Utc>) -> Result<Vec<UserSummary>, AppError> {
    users::list(db)
        .await?
        .into_iter()
        .map(|r| summary(r, now))
        .collect()
}

pub async fn create(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: NewUser,
    now: DateTime<Utc>,
) -> Result<UserSummary, AppError> {
    let username = input.username.trim();
    let mut errors = Vec::new();
    if let Some(m) = username_problem(username) {
        errors.push(field("username", m));
    } else if users::by_username(db, username).await?.is_some() {
        errors.push(field("username", "That username is already taken"));
    }
    if input.role == Role::Staff {
        // Plan §3.1: Staff accounts must be linked to an employee record.
        errors.push(field(
            "role",
            "Staff accounts are created from the employee's profile",
        ));
    }
    if let Some(m) = password_problem(username, &input.password) {
        errors.push(field("password", m));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let hash = hash_blocking(input.password).await?;
    let mut tx = db.begin().await?;
    let id = users::insert(&mut tx, username, &hash, input.role.as_db(), true).await?;
    let entry = Entry {
        action: "user.create",
        entity: Some(("user", id)),
        after: Some(json!({ "username": username, "role": input.role.as_db() })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;

    list(db, now)
        .await?
        .into_iter()
        .find(|u| u.id == id)
        .ok_or(AppError::NotFound("User"))
}

pub async fn set_role(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    role: Role,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let user = users::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    if user.role == role.as_db() {
        return Ok(());
    }
    if role == Role::Staff && user.employee_id.is_none() {
        // Plan §3.1: Staff accounts must be linked to an employee record.
        return Err(AppError::Validation(vec![field(
            "role",
            "Link this account to an employee before making it Staff",
        )]));
    }
    if user.role == Role::Admin.as_db() {
        if actor.user_id == Some(id) {
            return Err(AppError::Conflict("You can't remove your own Admin role"));
        }
        if user.is_active {
            ensure_another_admin(&mut tx).await?;
        }
    }
    users::set_role(&mut tx, id, role.as_db(), &time::to_db(now)).await?;
    let entry = Entry {
        action: "user.update",
        entity: Some(("user", id)),
        before: Some(json!({ "role": user.role })),
        after: Some(json!({ "role": role.as_db() })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn set_active(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    active: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let user = users::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    if user.is_active == active {
        return Ok(());
    }
    if active && user.role == Role::Staff.as_db() && user.employee_id.is_none() {
        return Err(AppError::Conflict(
            "Link this Staff account to an employee before turning it back on",
        ));
    }
    if !active {
        if actor.user_id == Some(id) {
            return Err(AppError::Conflict("You can't deactivate your own account"));
        }
        if user.role == Role::Admin.as_db() {
            ensure_another_admin(&mut tx).await?;
        }
    }
    users::set_active(&mut tx, id, active, &time::to_db(now)).await?;
    let entry = Entry {
        action: "user.set_active",
        entity: Some(("user", id)),
        before: Some(json!({ "isActive": user.is_active })),
        after: Some(json!({ "isActive": active })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn reset_password(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    temporary_password: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let user = users::by_id(db, id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    if let Some(m) = password_problem(&user.username, temporary_password) {
        return Err(AppError::Validation(vec![field("password", m)]));
    }
    let hash = hash_blocking(temporary_password.to_string()).await?;
    let mut tx = db.begin().await?;
    users::reset_password(&mut tx, id, &hash, &time::to_db(now)).await?;
    let entry = Entry {
        action: "user.reset_password",
        entity: Some(("user", id)),
        after: Some(json!({ "mustChangePassword": true })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Refuses a change that would leave no active Admin. Call it before demoting or
/// deactivating an active Admin, inside the same transaction.
async fn ensure_another_admin(conn: &mut SqliteConnection) -> Result<(), AppError> {
    if users::count_active_admins(conn).await? <= 1 {
        return Err(AppError::Conflict(
            "Wagecraft needs at least one active Admin",
        ));
    }
    Ok(())
}

fn summary(r: ListRow, now: DateTime<Utc>) -> Result<UserSummary, AppError> {
    let role = Role::from_db(&r.role)
        .ok_or_else(|| anyhow::anyhow!("user {} has unknown role {:?}", r.id, r.role))?;
    let locked = r
        .locked_until
        .as_deref()
        .and_then(time::from_db)
        .is_some_and(|until| until > now);
    Ok(UserSummary {
        id: r.id,
        username: r.username,
        role,
        employee_id: r.employee_id,
        employee_label: r.employee_label,
        is_active: r.is_active,
        must_change_password: r.must_change_password,
        locked,
        last_login_at: r.last_login_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::services::auth::{create_admin, login};
    use chrono::{Duration, TimeZone};

    const PW: &str = "correct horse battery";
    const TEMP: &str = "temporary password";

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 6, 9, 0, 0).unwrap()
    }

    fn admin() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "admin",
        }
    }

    async fn with_admin() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        create_admin(&pool, "Rojyx Inc.", "admin", PW)
            .await
            .expect("setup");
        (dir, pool)
    }

    async fn add(pool: &SqlitePool, username: &str, role: Role) -> UserSummary {
        let input = NewUser {
            username: username.into(),
            role,
            password: TEMP.into(),
        };
        create(pool, admin(), input, t0()).await.expect("create")
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn conflict(e: AppError) -> &'static str {
        match e {
            AppError::Conflict(m) => m,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_new_user_must_change_the_temporary_password() {
        let (_dir, pool) = with_admin().await;
        let hr = add(&pool, "maria", Role::Hr).await;
        assert_eq!(hr.role, Role::Hr);
        assert!(hr.must_change_password);
        assert!(hr.is_active);

        let signed_in = login(&pool, "maria", TEMP, t0()).await.expect("login");
        assert!(signed_in.must_change_password);

        let names: Vec<_> = list(&pool, t0())
            .await
            .expect("list")
            .into_iter()
            .map(|u| u.username)
            .collect();
        assert_eq!(names, ["admin", "maria"]);
    }

    #[tokio::test]
    async fn create_checks_its_fields() {
        let (_dir, pool) = with_admin().await;
        let bad = NewUser {
            username: "a b".into(),
            role: Role::Hr,
            password: "short".into(),
        };
        let err = create(&pool, admin(), bad, t0()).await.unwrap_err();
        assert_eq!(fields(err), ["username", "password"]);

        let taken = NewUser {
            username: "ADMIN".into(),
            role: Role::Hr,
            password: TEMP.into(),
        };
        let err = create(&pool, admin(), taken, t0()).await.unwrap_err();
        assert_eq!(fields(err), ["username"]);
    }

    #[tokio::test]
    async fn staff_accounts_need_an_employee() {
        let (_dir, pool) = with_admin().await;
        let staff = NewUser {
            username: "juan".into(),
            role: Role::Staff,
            password: TEMP.into(),
        };
        let err = create(&pool, admin(), staff, t0()).await.unwrap_err();
        assert_eq!(fields(err), ["role"]);

        let hr = add(&pool, "maria", Role::Hr).await;
        let err = set_role(&pool, admin(), hr.id, Role::Staff, t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["role"]);
    }

    #[tokio::test]
    async fn the_last_active_admin_cannot_be_removed() {
        let (_dir, pool) = with_admin().await;
        let hr = add(&pool, "maria", Role::Hr).await;
        // Signed in as someone else would be the only way to try; the rule is on the data.
        let other = Actor {
            user_id: Some(hr.id),
            username: "maria",
        };
        let demote = set_role(&pool, other, 1, Role::Hr, t0()).await.unwrap_err();
        let deactivate = set_active(&pool, other, 1, false, t0()).await.unwrap_err();
        assert_eq!(
            conflict(demote),
            "Wagecraft needs at least one active Admin"
        );
        assert_eq!(
            conflict(deactivate),
            "Wagecraft needs at least one active Admin"
        );

        // With a second Admin, the first one can step down.
        set_role(&pool, admin(), hr.id, Role::Admin, t0())
            .await
            .expect("promote");
        set_role(&pool, other, 1, Role::Hr, t0())
            .await
            .expect("demote");
    }

    #[tokio::test]
    async fn admins_cannot_lock_themselves_out() {
        let (_dir, pool) = with_admin().await;
        let second = add(&pool, "boss2", Role::Hr).await;
        set_role(&pool, admin(), second.id, Role::Admin, t0())
            .await
            .expect("promote");

        let demote = set_role(&pool, admin(), 1, Role::Hr, t0())
            .await
            .unwrap_err();
        let deactivate = set_active(&pool, admin(), 1, false, t0())
            .await
            .unwrap_err();
        assert_eq!(conflict(demote), "You can't remove your own Admin role");
        assert_eq!(
            conflict(deactivate),
            "You can't deactivate your own account"
        );
    }

    #[tokio::test]
    async fn a_deactivated_user_cannot_sign_in_until_reactivated() {
        let (_dir, pool) = with_admin().await;
        let hr = add(&pool, "maria", Role::Hr).await;
        set_active(&pool, admin(), hr.id, false, t0())
            .await
            .expect("off");
        assert!(login(&pool, "maria", TEMP, t0()).await.is_err());
        set_active(&pool, admin(), hr.id, true, t0())
            .await
            .expect("on");
        assert!(login(&pool, "maria", TEMP, t0()).await.is_ok());
    }

    #[tokio::test]
    async fn reset_password_unlocks_and_forces_a_change() {
        let (_dir, pool) = with_admin().await;
        let hr = add(&pool, "maria", Role::Hr).await;
        for _ in 0..5 {
            let _ = login(&pool, "maria", "wrong password!", t0()).await;
        }
        let locked = list(&pool, t0()).await.expect("list");
        assert!(locked.iter().any(|u| u.username == "maria" && u.locked));

        reset_password(&pool, admin(), hr.id, "another temp pass", t0())
            .await
            .expect("reset");
        let user = login(
            &pool,
            "maria",
            "another temp pass",
            t0() + Duration::minutes(1),
        )
        .await
        .expect("login after reset");
        assert!(user.must_change_password);
    }

    #[tokio::test]
    async fn unknown_ids_are_not_found() {
        let (_dir, pool) = with_admin().await;
        let err = set_active(&pool, admin(), 99, false, t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound("User")));
    }

    #[tokio::test]
    async fn every_change_is_audited_without_secrets() {
        let (_dir, pool) = with_admin().await;
        let hr = add(&pool, "maria", Role::Hr).await;
        set_role(&pool, admin(), hr.id, Role::Admin, t0())
            .await
            .expect("role");
        set_active(&pool, admin(), hr.id, false, t0())
            .await
            .expect("off");
        reset_password(&pool, admin(), hr.id, "another temp pass", t0())
            .await
            .expect("reset");

        let rows = audit_rows::all(&pool).await;
        let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "auth.setup",
                "user.create",
                "user.update",
                "user.set_active",
                "user.reset_password"
            ]
        );
        for r in &rows[1..] {
            assert_eq!(r.actor_user_id, Some(1));
            assert_eq!(r.entity_id, Some(hr.id));
            let text = format!("{:?} {:?}", r.before_json, r.after_json);
            for secret in ["argon2", TEMP, "another temp pass"] {
                assert!(!text.contains(secret), "{} leaks {secret}", r.action);
            }
        }
        assert_eq!(rows[2].before_json.as_deref(), Some(r#"{"role":"HR"}"#));
        assert_eq!(rows[2].after_json.as_deref(), Some(r#"{"role":"ADMIN"}"#));
        assert_eq!(rows[3].after_json.as_deref(), Some(r#"{"isActive":false}"#));
    }
}
