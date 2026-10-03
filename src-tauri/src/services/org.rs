//! Departments, positions and work schedules (plan §6.2). Deactivate instead of delete; every change audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::org::{
    Department, DepartmentInput, Position, PositionInput, WorkSchedule, WorkScheduleInput, WEEKDAYS,
};
use crate::error::{AppError, FieldError};
use crate::repositories::org::{self as repo, PositionFields, ScheduleFields};
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, NaiveTime, Utc};
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

pub async fn list_schedules(db: &SqlitePool) -> Result<Vec<WorkSchedule>, AppError> {
    Ok(repo::schedules(db).await?)
}

/// Longest unpaid break and grace period a schedule may have.
const MAX_BREAK_MINUTES: i64 = 240;
const MAX_GRACE_MINUTES: i64 = 60;

/// Trimmed and checked schedule fields, with times as `HH:MM` and work days in calendar
/// order.
struct CleanSchedule {
    name: String,
    start_time: String,
    end_time: String,
    work_days: String,
}

/// Accepts `8:00`, `08:00` or `08:00:00`; returns the time as `HH:MM`.
fn clean_time(s: &str) -> Option<String> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .ok()
        .map(|t| t.format("%H:%M").to_string())
}

/// `fri, mon,TUE` becomes `MON,TUE,FRI`. None if a day is unknown or none is given.
fn clean_work_days(s: &str) -> Option<String> {
    let picked: Vec<String> = s
        .split(',')
        .map(|d| d.trim().to_ascii_uppercase())
        .filter(|d| !d.is_empty())
        .collect();
    if picked.is_empty() || picked.iter().any(|d| !WEEKDAYS.contains(&d.as_str())) {
        return None;
    }
    let ordered: Vec<&str> = WEEKDAYS
        .into_iter()
        .filter(|w| picked.iter().any(|d| d == w))
        .collect();
    Some(ordered.join(","))
}

/// Minutes from start to end, wrapping past midnight: 22:00 to 06:00 is 480.
fn shift_minutes(start: &str, end: &str) -> i64 {
    let parse = |s: &str| NaiveTime::parse_from_str(s, "%H:%M").ok();
    match (parse(start), parse(end)) {
        (Some(a), Some(b)) => {
            let m = (b - a).num_minutes();
            if m <= 0 {
                m + 24 * 60
            } else {
                m
            }
        }
        _ => 0,
    }
}

fn clean_schedule(input: &WorkScheduleInput) -> Result<CleanSchedule, Vec<FieldError>> {
    let name = input.name.trim().to_string();
    let start_time = clean_time(&input.start_time);
    let end_time = clean_time(&input.end_time);
    let work_days = clean_work_days(&input.work_days);
    let mut errors = Vec::new();
    if name.is_empty() || name.chars().count() > 60 {
        errors.push(field("name", "Enter a name (up to 60 characters)"));
    }
    if start_time.is_none() {
        errors.push(field("startTime", "Enter a time like 08:00"));
    }
    match (&start_time, &end_time) {
        (_, None) => errors.push(field("endTime", "Enter a time like 17:00")),
        (Some(a), Some(b)) if a == b => {
            errors.push(field("endTime", "The shift must end at a different time"))
        }
        _ => {}
    }
    let length = match (&start_time, &end_time) {
        (Some(a), Some(b)) => shift_minutes(a, b),
        _ => 24 * 60,
    };
    if !(0..=MAX_BREAK_MINUTES).contains(&input.break_minutes) {
        errors.push(field("breakMinutes", "Enter 0 to 240 minutes"));
    } else if input.break_minutes >= length {
        errors.push(field(
            "breakMinutes",
            "The break must be shorter than the shift",
        ));
    }
    if !(0..=MAX_GRACE_MINUTES).contains(&input.grace_minutes) {
        errors.push(field("graceMinutes", "Enter 0 to 60 minutes"));
    }
    if work_days.is_none() {
        errors.push(field("workDays", "Pick at least one work day"));
    }
    match (start_time, end_time, work_days) {
        (Some(start_time), Some(end_time), Some(work_days)) if errors.is_empty() => {
            Ok(CleanSchedule {
                name,
                start_time,
                end_time,
                work_days,
            })
        }
        _ => Err(errors),
    }
}

fn schedule_fields<'a>(s: &'a CleanSchedule, input: &WorkScheduleInput) -> ScheduleFields<'a> {
    ScheduleFields {
        name: &s.name,
        start_time: &s.start_time,
        end_time: &s.end_time,
        break_minutes: input.break_minutes,
        grace_minutes: input.grace_minutes,
        work_days: &s.work_days,
    }
}

fn schedule_name_taken() -> AppError {
    AppError::Validation(vec![field(
        "name",
        "Another schedule already uses this name",
    )])
}

pub async fn create_schedule(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: WorkScheduleInput,
    now: DateTime<Utc>,
) -> Result<WorkSchedule, AppError> {
    let s = clean_schedule(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    if repo::schedule_name_taken(&mut tx, &s.name, None).await? {
        return Err(schedule_name_taken());
    }
    let id = repo::insert_schedule(&mut tx, &schedule_fields(&s, &input)).await?;
    let created = repo::schedule(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Work schedule"))?;
    let entry = Entry {
        action: "schedule.create",
        entity: Some(("work_schedule", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update_schedule(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: WorkScheduleInput,
    now: DateTime<Utc>,
) -> Result<WorkSchedule, AppError> {
    let s = clean_schedule(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    let before = repo::schedule(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Work schedule"))?;
    if repo::schedule_name_taken(&mut tx, &s.name, Some(id)).await? {
        return Err(schedule_name_taken());
    }
    repo::update_schedule(&mut tx, id, &schedule_fields(&s, &input), &time::to_db(now)).await?;
    let after = repo::schedule(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Work schedule"))?;
    let entry = Entry {
        action: "schedule.update",
        entity: Some(("work_schedule", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// Deactivating hides the schedule from new picks. Employees already on it keep it.
pub async fn set_schedule_active(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    active: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::schedule(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Work schedule"))?;
    if before.is_active == active {
        return Ok(());
    }
    repo::set_schedule_active(&mut tx, id, active, &time::to_db(now)).await?;
    let entry = Entry {
        action: "schedule.set_active",
        entity: Some(("work_schedule", id)),
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

    fn sched(name: &str, start: &str, end: &str, days: &str) -> WorkScheduleInput {
        WorkScheduleInput {
            name: name.into(),
            start_time: start.into(),
            end_time: end.into(),
            break_minutes: 60,
            grace_minutes: 10,
            work_days: days.into(),
        }
    }

    #[tokio::test]
    async fn creates_a_schedule_with_tidy_times_and_days() {
        let (_dir, db) = db().await;
        let s = create_schedule(
            &db,
            hr(),
            sched(" Day shift ", "8:00", "17:00:00", "fri,mon, Tue"),
            t0(),
        )
        .await
        .expect("create");
        assert_eq!(s.name, "Day shift");
        assert_eq!(
            (s.start_time.as_str(), s.end_time.as_str()),
            ("08:00", "17:00")
        );
        assert_eq!(s.work_days, "MON,TUE,FRI");
        assert_eq!(s.employee_count, 0);
        assert!(s.is_active);
    }

    #[tokio::test]
    async fn a_night_schedule_may_end_the_next_day() {
        let (_dir, db) = db().await;
        let s = create_schedule(&db, hr(), sched("Night", "22:00", "06:00", "MON"), t0())
            .await
            .expect("create");
        assert_eq!(s.end_time, "06:00");
    }

    #[tokio::test]
    async fn schedule_fields_are_checked() {
        let (_dir, db) = db().await;
        let mut bad = sched(" ", "25:00", "8 pm", "");
        bad.grace_minutes = 90;
        let err = create_schedule(&db, hr(), bad, t0()).await.unwrap_err();
        assert_eq!(
            fields(err),
            ["name", "startTime", "endTime", "graceMinutes", "workDays"]
        );

        let same = sched("Same", "08:00", "08:00", "MON");
        let err = create_schedule(&db, hr(), same, t0()).await.unwrap_err();
        assert_eq!(fields(err), ["endTime"]);

        let mut long_break = sched("Short", "09:00", "10:00", "MON");
        long_break.break_minutes = 60;
        let err = create_schedule(&db, hr(), long_break, t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["breakMinutes"]);

        let unknown_day = sched("Odd", "09:00", "17:00", "MON,FUNDAY");
        let err = create_schedule(&db, hr(), unknown_day, t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["workDays"]);
    }

    #[tokio::test]
    async fn schedule_names_are_unique_but_a_schedule_keeps_its_own() {
        let (_dir, db) = db().await;
        let day = create_schedule(&db, hr(), sched("Day", "08:00", "17:00", "MON"), t0())
            .await
            .expect("create");
        let err = create_schedule(&db, hr(), sched("day", "09:00", "18:00", "MON"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["name"]);
        update_schedule(
            &db,
            hr(),
            day.id,
            sched("Day", "07:00", "16:00", "MON"),
            t0(),
        )
        .await
        .expect("keeps its own name");
    }

    #[tokio::test]
    async fn schedule_changes_are_audited() {
        let (_dir, db) = db().await;
        let day = create_schedule(&db, hr(), sched("Day", "08:00", "17:00", "MON"), t0())
            .await
            .expect("create");
        update_schedule(
            &db,
            hr(),
            day.id,
            sched("Day", "07:30", "16:30", "MON,TUE"),
            t0(),
        )
        .await
        .expect("update");
        set_schedule_active(&db, hr(), day.id, false, t0())
            .await
            .expect("off");
        set_schedule_active(&db, hr(), day.id, false, t0())
            .await
            .expect("no-op");
        let rows = audit_rows::all(&db).await;
        let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            actions,
            ["schedule.create", "schedule.update", "schedule.set_active"]
        );
        assert!(rows[1]
            .before_json
            .as_deref()
            .unwrap()
            .contains("\"08:00\""));
        assert!(rows[1].after_json.as_deref().unwrap().contains("\"07:30\""));

        let err = update_schedule(&db, hr(), 999, sched("X", "08:00", "17:00", "MON"), t0())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }
}
