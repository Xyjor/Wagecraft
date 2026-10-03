//! Links between sign-in accounts and employee records (plan §3.1, §6.2).
//! Staff accounts must be linked, so they are created here, from the employee's side.
//! Admin and HR accounts may be linked too, so their own payslips work later.

use crate::audit::{self, Actor, Entry};
use crate::auth::permissions::Role;
use crate::error::AppError;
use crate::repositories::{employees, users};
use crate::services::auth::{field, hash_blocking, password_problem, username_problem};
use crate::time;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};
use ts_rs::TS;

/// The account linked to an employee, or one that could be.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountSummary {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewStaffAccount {
    pub username: String,
    /// A temporary password. The employee must replace it at first sign-in.
    pub password: String,
}

fn summary(u: users::UserRow) -> Result<AccountSummary, AppError> {
    let role = Role::from_db(&u.role)
        .ok_or_else(|| anyhow::anyhow!("user {} has unknown role {:?}", u.id, u.role))?;
    Ok(AccountSummary {
        user_id: u.id,
        username: u.username,
        role,
        is_active: u.is_active,
    })
}

pub async fn linked(db: &SqlitePool, employee_id: i64) -> Result<Option<AccountSummary>, AppError> {
    let mut conn = db.acquire().await?;
    employees::get(&mut *conn, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    users::by_employee(&mut *conn, employee_id)
        .await?
        .map(summary)
        .transpose()
}

/// The employee a user account is linked to right now, if any.
pub async fn employee_of(db: &SqlitePool, user_id: i64) -> Result<Option<i64>, AppError> {
    Ok(users::by_id(db, user_id).await?.and_then(|u| u.employee_id))
}

/// Accounts not linked to anyone yet.
pub async fn linkable(db: &SqlitePool) -> Result<Vec<AccountSummary>, AppError> {
    users::unlinked(db)
        .await?
        .into_iter()
        .map(summary)
        .collect()
}

/// Creates a Staff account for an employee who has none. It must change its password at
/// first sign-in, like every account an Admin or HR sets up.
pub async fn create_staff(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    input: NewStaffAccount,
    now: DateTime<Utc>,
) -> Result<AccountSummary, AppError> {
    let username = input.username.trim();
    let mut errors = Vec::new();
    if let Some(m) = username_problem(username) {
        errors.push(field("username", m));
    } else if users::by_username(db, username).await?.is_some() {
        errors.push(field("username", "That username is already taken"));
    }
    if let Some(m) = password_problem(username, &input.password) {
        errors.push(field("password", m));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let hash = hash_blocking(input.password).await?;

    let mut tx = db.begin().await?;
    linkable_employee(&mut tx, employee_id).await?;
    let id = users::insert(&mut tx, username, &hash, Role::Staff.as_db(), true).await?;
    users::set_employee(&mut tx, id, Some(employee_id), &time::to_db(now)).await?;
    let entry = Entry {
        action: "user.create",
        entity: Some(("user", id)),
        after: Some(json!({
            "username": username,
            "role": Role::Staff.as_db(),
            "employeeId": employee_id,
        })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let created = users::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    tx.commit().await?;
    summary(created)
}

/// Links an existing account that isn't linked to anyone.
pub async fn link(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    user_id: i64,
    now: DateTime<Utc>,
) -> Result<AccountSummary, AppError> {
    let mut tx = db.begin().await?;
    linkable_employee(&mut tx, employee_id).await?;
    let user = users::by_id(&mut *tx, user_id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    if user.employee_id.is_some() {
        return Err(AppError::Conflict(
            "That account is already linked to another employee",
        ));
    }
    users::set_employee(&mut tx, user_id, Some(employee_id), &time::to_db(now)).await?;
    let entry = Entry {
        action: "employee.link_user",
        entity: Some(("employee", employee_id)),
        after: Some(json!({ "userId": user_id, "username": user.username })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let linked = users::by_id(&mut *tx, user_id)
        .await?
        .ok_or(AppError::NotFound("User"))?;
    tx.commit().await?;
    summary(linked)
}

/// Removes the link. A Staff account can't exist unlinked and active, so it is turned off
/// in the same step; linking it again later and switching it on brings it back.
pub async fn unlink(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let Some(user) = users::by_employee(&mut *tx, employee_id).await? else {
        return Ok(());
    };
    if actor.user_id == Some(user.id) {
        return Err(AppError::Conflict(
            "You can't unlink your own account. Ask another Admin or HR user",
        ));
    }
    let at = time::to_db(now);
    users::set_employee(&mut tx, user.id, None, &at).await?;
    let deactivated = user.role == Role::Staff.as_db() && user.is_active;
    if deactivated {
        users::set_active(&mut tx, user.id, false, &at).await?;
    }
    let entry = Entry {
        action: "employee.unlink_user",
        entity: Some(("employee", employee_id)),
        before: Some(json!({ "userId": user.id, "username": user.username })),
        after: Some(json!({ "deactivated": deactivated })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Turns off the Staff account of an employee being archived: people who have left
/// shouldn't be able to sign in. Runs inside the archive's transaction.
pub(crate) async fn deactivate_staff_of(
    conn: &mut SqliteConnection,
    actor: Actor<'_>,
    employee_id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let Some(user) = users::by_employee(&mut *conn, employee_id).await? else {
        return Ok(());
    };
    if user.role != Role::Staff.as_db() || !user.is_active {
        return Ok(());
    }
    users::set_active(conn, user.id, false, &time::to_db(now)).await?;
    let entry = Entry {
        action: "user.set_active",
        entity: Some(("user", user.id)),
        before: Some(json!({ "isActive": true })),
        after: Some(json!({ "isActive": false, "reason": "employee archived" })),
    };
    audit::record(conn, now, actor, entry).await?;
    Ok(())
}

/// The employee exists, is current, and has no account yet.
async fn linkable_employee(conn: &mut SqliteConnection, employee_id: i64) -> Result<(), AppError> {
    let employee = employees::get(&mut *conn, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "Unarchive this employee before giving them an account",
        ));
    }
    if users::by_employee(&mut *conn, employee_id).await?.is_some() {
        return Err(AppError::Conflict(
            "This employee already has an account. Unlink it first",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::employee::EmployeeInput;
    use crate::services::{auth, employees as employee_service, users as user_service};
    use chrono::{NaiveDate, TimeZone};

    const TEMP: &str = "temporary password";

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 1, 0, 0).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    async fn db() -> (tempfile::TempDir, SqlitePool, i64) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = pool.acquire().await.expect("conn");
        users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        drop(conn);
        let id = employee(&pool, "EMP-0001").await;
        (dir, pool, id)
    }

    async fn employee(pool: &SqlitePool, no: &str) -> i64 {
        let input = EmployeeInput {
            employee_no: no.into(),
            first_name: "Juan".into(),
            last_name: "Dela Cruz".into(),
            hire_date: "2025-01-06".into(),
            employment_status: "REGULAR".into(),
            ..Default::default()
        };
        let today = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        employee_service::create(pool, hr(), input, today, now())
            .await
            .expect("employee")
            .id
    }

    fn juan() -> NewStaffAccount {
        NewStaffAccount {
            username: " juan ".into(),
            password: TEMP.into(),
        }
    }

    fn conflict(e: AppError) -> &'static str {
        match e {
            AppError::Conflict(m) => m,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn creates_a_linked_staff_account_that_can_sign_in() {
        let (_dir, db, id) = db().await;
        let account = create_staff(&db, hr(), id, juan(), now())
            .await
            .expect("create");
        assert_eq!(account.username, "juan");
        assert_eq!(account.role, Role::Staff);
        assert_eq!(
            linked(&db, id).await.unwrap().unwrap().user_id,
            account.user_id
        );

        let me = auth::login(&db, "juan", TEMP, now()).await.expect("login");
        assert_eq!(me.employee_id, Some(id));
        assert_eq!(employee_of(&db, me.id).await.unwrap(), Some(id));
        assert!(me.must_change_password);

        let rows = audit_rows::all(&db).await;
        let created = rows.iter().find(|r| r.action == "user.create").unwrap();
        assert!(created
            .after_json
            .as_deref()
            .unwrap()
            .contains("\"employeeId\""));
    }

    #[tokio::test]
    async fn one_account_per_employee_and_one_employee_per_account() {
        let (_dir, db, id) = db().await;
        create_staff(&db, hr(), id, juan(), now())
            .await
            .expect("first");
        let again = NewStaffAccount {
            username: "juan2".into(),
            password: TEMP.into(),
        };
        let err = create_staff(&db, hr(), id, again, now()).await.unwrap_err();
        assert_eq!(
            conflict(err),
            "This employee already has an account. Unlink it first"
        );

        let other = employee(&db, "EMP-0002").await;
        let juan_id = linked(&db, id).await.unwrap().unwrap().user_id;
        let err = link(&db, hr(), other, juan_id, now()).await.unwrap_err();
        assert_eq!(
            conflict(err),
            "That account is already linked to another employee"
        );
    }

    #[tokio::test]
    async fn links_an_existing_hr_account_and_lists_only_unlinked_ones() {
        let (_dir, db, id) = db().await;
        assert_eq!(linkable(&db).await.unwrap().len(), 1);
        let account = link(&db, hr(), id, 1, now()).await.expect("link");
        assert_eq!(account.role, Role::Hr);
        assert!(linkable(&db).await.unwrap().is_empty());
        let actions = audit_rows::actions(&db).await;
        assert_eq!(
            actions.last().map(String::as_str),
            Some("employee.link_user")
        );
    }

    #[tokio::test]
    async fn unlinking_staff_turns_the_account_off_until_relinked() {
        let (_dir, db, id) = db().await;
        let account = create_staff(&db, hr(), id, juan(), now()).await.unwrap();
        unlink(&db, hr(), id, now()).await.expect("unlink");
        assert!(linked(&db, id).await.unwrap().is_none());
        assert_eq!(employee_of(&db, account.user_id).await.unwrap(), None);

        let err = auth::login(&db, "juan", TEMP, now()).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidCredentials), "{err:?}");
        let err = user_service::set_active(&db, hr(), account.user_id, true, now())
            .await
            .unwrap_err();
        assert_eq!(
            conflict(err),
            "Link this Staff account to an employee before turning it back on"
        );

        link(&db, hr(), id, account.user_id, now())
            .await
            .expect("relink");
        user_service::set_active(&db, hr(), account.user_id, true, now())
            .await
            .expect("back on");
        auth::login(&db, "juan", TEMP, now())
            .await
            .expect("signs in again");
    }

    #[tokio::test]
    async fn nobody_unlinks_their_own_account() {
        let (_dir, db, id) = db().await;
        link(&db, hr(), id, 1, now()).await.unwrap();
        let err = unlink(&db, hr(), id, now()).await.unwrap_err();
        assert_eq!(
            conflict(err),
            "You can't unlink your own account. Ask another Admin or HR user"
        );
    }

    #[tokio::test]
    async fn archiving_an_employee_turns_off_their_staff_account() {
        let (_dir, db, id) = db().await;
        create_staff(&db, hr(), id, juan(), now()).await.unwrap();
        sqlx::query(
            "UPDATE employees SET employment_status = 'RESIGNED', separation_date = '2026-09-30'",
        )
        .execute(&db)
        .await
        .unwrap();
        employee_service::set_archived(&db, hr(), id, true, now())
            .await
            .expect("archive");

        let account = linked(&db, id).await.unwrap().unwrap();
        assert!(!account.is_active);
        let err = create_staff(&db, hr(), id, juan(), now())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Validation(_) | AppError::Conflict(_)
        ));
    }

    #[tokio::test]
    async fn checks_username_and_password_like_the_users_screen() {
        let (_dir, db, id) = db().await;
        let bad = NewStaffAccount {
            username: "hr".into(),
            password: "short".into(),
        };
        match create_staff(&db, hr(), id, bad, now()).await.unwrap_err() {
            AppError::Validation(f) => {
                let names: Vec<_> = f.iter().map(|f| f.field.as_str()).collect();
                assert_eq!(names, ["username", "password"]);
            }
            other => panic!("expected validation, got {other:?}"),
        }
    }
}
