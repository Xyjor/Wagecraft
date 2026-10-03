//! Departments and positions (plan §6.2). Deactivate instead of delete; every change audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::org::{Department, DepartmentInput, Position, PositionInput};
use crate::error::{AppError, FieldError};
use crate::repositories::org::{self as repo, PositionFields};
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;

/// Largest rate accepted anywhere: ₱10,000,000.00 a month, far above any real salary.
pub const MAX_RATE_CENTS: i64 = 1_000_000_000;

pub async fn list_departments(db: &SqlitePool) -> Result<Vec<Department>, AppError> {
    Ok(repo::departments(db).await?)
}

/// Trimmed, upper-cased and checked department fields.
struct CleanDepartment {
    code: String,
    name: String,
    description: Option<String>,
}

fn clean_department(input: &DepartmentInput) -> Result<CleanDepartment, Vec<FieldError>> {
    let code = input.code.trim().to_uppercase();
    let name = input.name.trim().to_string();
    let description = input
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_string);
    let mut errors = Vec::new();
    let code_ok = (2..=10).contains(&code.len())
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !code_ok {
        errors.push(field("code", "Use 2 to 10 letters, numbers or dashes"));
    }
    if name.is_empty() || name.chars().count() > 100 {
        errors.push(field("name", "Enter a name (up to 100 characters)"));
    }
    if description
        .as_ref()
        .is_some_and(|d| d.chars().count() > 500)
    {
        errors.push(field(
            "description",
            "Keep the description under 500 characters",
        ));
    }
    if errors.is_empty() {
        Ok(CleanDepartment {
            code,
            name,
            description,
        })
    } else {
        Err(errors)
    }
}

pub async fn create_department(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: DepartmentInput,
    now: DateTime<Utc>,
) -> Result<Department, AppError> {
    let d = clean_department(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    if repo::department_code_taken(&mut tx, &d.code, None).await? {
        return Err(code_taken());
    }
    let id = repo::insert_department(&mut tx, &d.code, &d.name, d.description.as_deref()).await?;
    let created = repo::department(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Department"))?;
    let entry = Entry {
        action: "department.create",
        entity: Some(("department", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update_department(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: DepartmentInput,
    now: DateTime<Utc>,
) -> Result<Department, AppError> {
    let d = clean_department(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    let before = repo::department(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Department"))?;
    if repo::department_code_taken(&mut tx, &d.code, Some(id)).await? {
        return Err(code_taken());
    }
    repo::update_department(
        &mut tx,
        id,
        &d.code,
        &d.name,
        d.description.as_deref(),
        &time::to_db(now),
    )
    .await?;
    let after = repo::department(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Department"))?;
    let entry = Entry {
        action: "department.update",
        entity: Some(("department", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

fn code_taken() -> AppError {
    AppError::Validation(vec![field(
        "code",
        "Another department already uses this code",
    )])
}

pub async fn set_department_active(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    active: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::department(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Department"))?;
    if before.is_active == active {
        return Ok(());
    }
    repo::set_department_active(&mut tx, id, active, &time::to_db(now)).await?;
    let entry = Entry {
        action: "department.set_active",
        entity: Some(("department", id)),
        before: Some(json!({ "isActive": before.is_active })),
        after: Some(json!({ "isActive": active })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn list_positions(db: &SqlitePool) -> Result<Vec<Position>, AppError> {
    Ok(repo::positions(db).await?)
}

/// Checks a position's own fields. The department and title checks need the database.
fn check_position(input: &PositionInput) -> (String, Vec<FieldError>) {
    let title = input.title.trim().to_string();
    let mut errors = Vec::new();
    if title.is_empty() || title.chars().count() > 100 {
        errors.push(field("title", "Enter a title (up to 100 characters)"));
    }
    let in_range = |r: Option<i64>| r.is_none_or(|c| (0..=MAX_RATE_CENTS).contains(&c));
    if !in_range(input.min_rate_cents) {
        errors.push(field(
            "minRateCents",
            "Enter an amount from ₱0 to ₱10,000,000",
        ));
    }
    if !in_range(input.max_rate_cents) {
        errors.push(field(
            "maxRateCents",
            "Enter an amount from ₱0 to ₱10,000,000",
        ));
    } else if let (Some(min), Some(max)) = (input.min_rate_cents, input.max_rate_cents) {
        if errors.is_empty() && min > max {
            errors.push(field(
                "maxRateCents",
                "The maximum must not be below the minimum",
            ));
        }
    }
    (title, errors)
}

/// Runs every position check, including the ones that read the database.
async fn validate_position(
    conn: &mut sqlx::SqliteConnection,
    input: &PositionInput,
    except_id: Option<i64>,
) -> Result<String, AppError> {
    let (title, mut errors) = check_position(input);
    let department = repo::department(&mut *conn, input.department_id).await?;
    if !department.is_some_and(|d| d.is_active) {
        errors.insert(0, field("departmentId", "Pick an active department"));
    } else if errors.iter().all(|e| e.field != "title")
        && repo::position_title_taken(conn, input.department_id, &title, except_id).await?
    {
        errors.insert(
            0,
            field("title", "This department already has that position"),
        );
    }
    if errors.is_empty() {
        Ok(title)
    } else {
        Err(AppError::Validation(errors))
    }
}

pub async fn create_position(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: PositionInput,
    now: DateTime<Utc>,
) -> Result<Position, AppError> {
    let mut tx = db.begin().await?;
    let title = validate_position(&mut tx, &input, None).await?;
    let fields = PositionFields {
        department_id: input.department_id,
        title: &title,
        min_rate_cents: input.min_rate_cents,
        max_rate_cents: input.max_rate_cents,
    };
    let id = repo::insert_position(&mut tx, &fields).await?;
    let created = repo::position(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Position"))?;
    let entry = Entry {
        action: "position.create",
        entity: Some(("position", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update_position(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: PositionInput,
    now: DateTime<Utc>,
) -> Result<Position, AppError> {
    let mut tx = db.begin().await?;
    let before = repo::position(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Position"))?;
    let title = validate_position(&mut tx, &input, Some(id)).await?;
    let fields = PositionFields {
        department_id: input.department_id,
        title: &title,
        min_rate_cents: input.min_rate_cents,
        max_rate_cents: input.max_rate_cents,
    };
    repo::update_position(&mut tx, id, &fields, &time::to_db(now)).await?;
    let after = repo::position(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Position"))?;
    let entry = Entry {
        action: "position.update",
        entity: Some(("position", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn set_position_active(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    active: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::position(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Position"))?;
    if before.is_active == active {
        return Ok(());
    }
    repo::set_position_active(&mut tx, id, active, &time::to_db(now)).await?;
    let entry = Entry {
        action: "position.set_active",
        entity: Some(("position", id)),
        before: Some(json!({ "isActive": before.is_active })),
        after: Some(json!({ "isActive": active })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        // The audit log needs a real acting user.
        let mut conn = pool.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        drop(conn);
        (dir, pool)
    }

    fn dept(code: &str, name: &str) -> DepartmentInput {
        DepartmentInput {
            code: code.into(),
            name: name.into(),
            description: None,
        }
    }

    fn pos(department_id: i64, title: &str, min: Option<i64>, max: Option<i64>) -> PositionInput {
        PositionInput {
            department_id,
            title: title.into(),
            min_rate_cents: min,
            max_rate_cents: max,
        }
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn creates_a_department_with_a_tidy_code() {
        let (_dir, db) = db().await;
        let input = DepartmentInput {
            code: " hr ".into(),
            name: " Human Resources ".into(),
            description: Some("  ".into()),
        };
        let d = create_department(&db, hr(), input, t0())
            .await
            .expect("create");
        assert_eq!(d.code, "HR");
        assert_eq!(d.name, "Human Resources");
        assert_eq!(d.description, None);
        assert!(d.is_active);
    }

    #[tokio::test]
    async fn department_fields_are_checked() {
        let (_dir, db) = db().await;
        let err = create_department(&db, hr(), dept("a b!", " "), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["code", "name"]);

        create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("first");
        let dup = create_department(&db, hr(), dept("ops", "Other"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(dup), ["code"]);
    }

    #[tokio::test]
    async fn a_department_can_keep_its_own_code_when_edited() {
        let (_dir, db) = db().await;
        let d = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("create");
        let renamed = update_department(&db, hr(), d.id, dept("OPS", "Field Ops"), t0())
            .await
            .expect("update");
        assert_eq!(renamed.name, "Field Ops");
    }

    #[tokio::test]
    async fn positions_belong_to_an_active_department() {
        let (_dir, db) = db().await;
        let ops = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("dept");
        let p = create_position(
            &db,
            hr(),
            pos(ops.id, "Driver", Some(1_800_000), None),
            t0(),
        )
        .await
        .expect("position");
        assert_eq!(p.department_name, "Operations");

        let missing = create_position(&db, hr(), pos(99, "Clerk", None, None), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(missing), ["departmentId"]);

        set_department_active(&db, hr(), ops.id, false, t0())
            .await
            .expect("deactivate");
        let inactive = create_position(&db, hr(), pos(ops.id, "Clerk", None, None), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(inactive), ["departmentId"]);
    }

    #[tokio::test]
    async fn position_fields_are_checked() {
        let (_dir, db) = db().await;
        let ops = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("dept");
        create_position(&db, hr(), pos(ops.id, "Driver", None, None), t0())
            .await
            .expect("first");

        let dup = create_position(&db, hr(), pos(ops.id, "DRIVER", None, None), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(dup), ["title"]);

        let bad = create_position(&db, hr(), pos(ops.id, " ", Some(-1), Some(5)), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(bad), ["title", "minRateCents"]);

        let upside_down = create_position(
            &db,
            hr(),
            pos(ops.id, "Clerk", Some(3_000_000), Some(2_000_000)),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(upside_down), ["maxRateCents"]);
    }

    #[tokio::test]
    async fn moving_a_position_checks_the_new_department() {
        let (_dir, db) = db().await;
        let ops = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("dept");
        let fin = create_department(&db, hr(), dept("FIN", "Finance"), t0())
            .await
            .expect("dept");
        let p = create_position(&db, hr(), pos(ops.id, "Clerk", None, None), t0())
            .await
            .expect("position");
        let moved = update_position(&db, hr(), p.id, pos(fin.id, "Clerk", None, None), t0())
            .await
            .expect("move");
        assert_eq!(moved.department_name, "Finance");
    }

    #[tokio::test]
    async fn unknown_ids_are_not_found() {
        let (_dir, db) = db().await;
        let d = update_department(&db, hr(), 9, dept("X1", "X"), t0()).await;
        let p = set_position_active(&db, hr(), 9, false, t0()).await;
        assert!(matches!(d, Err(AppError::NotFound("Department"))));
        assert!(matches!(p, Err(AppError::NotFound("Position"))));
    }

    #[tokio::test]
    async fn every_change_is_audited_with_before_and_after() {
        let (_dir, db) = db().await;
        let ops = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("dept");
        update_department(&db, hr(), ops.id, dept("OPS", "Field Ops"), t0())
            .await
            .expect("update");
        let p = create_position(&db, hr(), pos(ops.id, "Driver", None, None), t0())
            .await
            .expect("position");
        update_position(
            &db,
            hr(),
            p.id,
            pos(ops.id, "Senior Driver", None, None),
            t0(),
        )
        .await
        .expect("update");
        set_position_active(&db, hr(), p.id, false, t0())
            .await
            .expect("off");
        set_department_active(&db, hr(), ops.id, false, t0())
            .await
            .expect("off");

        let rows = audit_rows::all(&db).await;
        let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "department.create",
                "department.update",
                "position.create",
                "position.update",
                "position.set_active",
                "department.set_active",
            ]
        );
        let update = &rows[1];
        assert_eq!(update.entity_type.as_deref(), Some("department"));
        assert!(update
            .before_json
            .as_deref()
            .unwrap()
            .contains("\"Operations\""));
        assert!(update
            .after_json
            .as_deref()
            .unwrap()
            .contains("\"Field Ops\""));
    }

    #[tokio::test]
    async fn setting_the_same_state_twice_changes_nothing() {
        let (_dir, db) = db().await;
        let ops = create_department(&db, hr(), dept("OPS", "Operations"), t0())
            .await
            .expect("dept");
        set_department_active(&db, hr(), ops.id, true, t0())
            .await
            .expect("no-op");
        assert_eq!(audit_rows::actions(&db).await, ["department.create"]);
    }
}
