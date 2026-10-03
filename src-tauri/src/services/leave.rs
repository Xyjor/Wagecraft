//! Leave types and yearly balances (plan §6.4). Admin manages the types; HR grants each
//! year's balances and adjusts one when needed. Every change is audited. Requests and
//! approvals build on this.

use crate::audit::{self, Actor, Entry};
use crate::domain::leave::{
    grant_as_of, has_served, BalanceAdjustment, LeaveBalance, LeaveType, LeaveTypeInput,
};
use crate::error::{AppError, FieldError};
use crate::repositories::leave::{self as repo, TypeFields};
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

/// A year of days, in half days. Anything above is a typo.
const MAX_HALFDAYS: i64 = 730;
const MAX_SERVICE_MONTHS: i64 = 120;

pub async fn list_types(db: &SqlitePool) -> Result<Vec<LeaveType>, AppError> {
    Ok(repo::types(db).await?)
}

/// Trimmed, upper-cased and checked leave type fields.
struct CleanType {
    code: String,
    name: String,
}

fn clean_type(input: &LeaveTypeInput) -> Result<CleanType, Vec<FieldError>> {
    let code = input.code.trim().to_uppercase();
    let name = input.name.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut errors = Vec::new();
    if !(2..=10).contains(&code.len()) || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        errors.push(field("code", "Use 2 to 10 letters or numbers"));
    }
    if name.is_empty() || name.chars().count() > 60 {
        errors.push(field("name", "Enter a name (up to 60 characters)"));
    }
    if !(0..=MAX_HALFDAYS).contains(&input.default_halfdays_per_year) {
        errors.push(field("defaultDays", "Enter 0 to 365 days"));
    } else if !input.is_paid && input.default_halfdays_per_year > 0 {
        errors.push(field(
            "defaultDays",
            "Unpaid leave uses no balance, so leave this at 0",
        ));
    }
    if !(0..=MAX_SERVICE_MONTHS).contains(&input.min_service_months) {
        errors.push(field("minServiceMonths", "Enter 0 to 120 months"));
    }
    if errors.is_empty() {
        Ok(CleanType { code, name })
    } else {
        Err(errors)
    }
}

fn type_fields<'a>(t: &'a CleanType, input: &LeaveTypeInput) -> TypeFields<'a> {
    TypeFields {
        code: &t.code,
        name: &t.name,
        is_paid: input.is_paid,
        default_halfdays_per_year: input.default_halfdays_per_year,
        min_service_months: input.min_service_months,
    }
}

fn code_taken() -> AppError {
    AppError::Validation(vec![field(
        "code",
        "Another leave type already uses this code",
    )])
}

pub async fn create_type(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: LeaveTypeInput,
    now: DateTime<Utc>,
) -> Result<LeaveType, AppError> {
    let t = clean_type(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    if repo::code_taken(&mut tx, &t.code, None).await? {
        return Err(code_taken());
    }
    let id = repo::insert_type(&mut tx, &type_fields(&t, &input)).await?;
    let created = repo::type_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave type"))?;
    let entry = Entry {
        action: "leave_type.create",
        entity: Some(("leave_type", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

/// Changes apply to grants from now on. Balances already granted keep their days.
pub async fn update_type(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: LeaveTypeInput,
    now: DateTime<Utc>,
) -> Result<LeaveType, AppError> {
    let t = clean_type(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    let before = repo::type_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave type"))?;
    if repo::code_taken(&mut tx, &t.code, Some(id)).await? {
        return Err(code_taken());
    }
    repo::update_type(&mut tx, id, &type_fields(&t, &input), &time::to_db(now)).await?;
    let after = repo::type_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave type"))?;
    let entry = Entry {
        action: "leave_type.update",
        entity: Some(("leave_type", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// An inactive type can't be filed or granted. Balances already granted stay.
pub async fn set_type_active(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    active: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::type_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave type"))?;
    if before.is_active == active {
        return Ok(());
    }
    repo::set_type_active(&mut tx, id, active, &time::to_db(now)).await?;
    let entry = Entry {
        action: "leave_type.set_active",
        entity: Some(("leave_type", id)),
        before: Some(json!({ "isActive": before.is_active })),
        after: Some(json!({ "isActive": active })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Adds the year's missing balances for current employees, or just `only`. Existing
/// balances are left alone, so running it again is safe and picks up anyone who has
/// since served long enough (such as a first work anniversary for SIL).
async fn grant_in(
    conn: &mut SqliteConnection,
    year: i32,
    today: NaiveDate,
    only: Option<i64>,
) -> Result<usize, AppError> {
    let as_of = grant_as_of(year, today);
    let types: Vec<_> = repo::types(&mut *conn)
        .await?
        .into_iter()
        .filter(|t| t.is_active && t.is_paid && t.default_halfdays_per_year > 0)
        .collect();
    let employees = repo::grant_employees(conn, &as_of.to_string(), only).await?;
    let mut granted = 0;
    for e in employees {
        let Ok(hired) = NaiveDate::parse_from_str(&e.hire_date, "%Y-%m-%d") else {
            continue;
        };
        for t in &types {
            if has_served(hired, t.min_service_months, as_of)
                && repo::insert_balance_if_missing(
                    conn,
                    e.id,
                    t.id,
                    year,
                    t.default_halfdays_per_year,
                )
                .await?
            {
                granted += 1;
            }
        }
    }
    Ok(granted)
}

fn check_grant_year(year: i32, today: NaiveDate) -> Result<(), AppError> {
    if year == today.year() || year == today.year() + 1 {
        Ok(())
    } else {
        Err(AppError::Validation(vec![field(
            "year",
            "Grant leave for this year or next year",
        )]))
    }
}

/// HR's yearly grant. Returns how many balances were added.
pub async fn grant(
    db: &SqlitePool,
    actor: Actor<'_>,
    year: i32,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<usize, AppError> {
    check_grant_year(year, today)?;
    let mut tx = db.begin().await?;
    let granted = grant_in(&mut tx, year, today, None).await?;
    let entry = Entry {
        action: "leave.grant",
        after: Some(json!({ "year": year, "balancesAdded": granted })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(granted)
}

/// One employee's balances for a year. For the current year, any missing balance is
/// granted first, so a new year works without HR pressing the button (plan Phase 4).
pub async fn balances(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    year: i32,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<LeaveBalance>, AppError> {
    if !(2000..=2100).contains(&year) {
        return Err(AppError::Validation(vec![field("year", "Pick a year")]));
    }
    if year == today.year() {
        let mut tx = db.begin().await?;
        let granted = grant_in(&mut tx, year, today, Some(employee_id)).await?;
        if granted > 0 {
            let entry = Entry {
                action: "leave.grant",
                entity: Some(("employee", employee_id)),
                after: Some(json!({ "year": year, "balancesAdded": granted })),
                ..Default::default()
            };
            audit::record(&mut tx, now, actor, entry).await?;
        }
        tx.commit().await?;
    }
    Ok(repo::balances(db, employee_id, year).await?)
}

/// HR changes a balance's yearly days, for example to carry days over. It can't go
/// below what's already used.
pub async fn adjust(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: BalanceAdjustment,
    now: DateTime<Utc>,
) -> Result<LeaveBalance, AppError> {
    let reason = input.reason.trim();
    let mut errors = Vec::new();
    if !(0..=MAX_HALFDAYS).contains(&input.entitled_halfdays) {
        errors.push(field("days", "Enter 0 to 365 days"));
    }
    if !(3..=200).contains(&reason.chars().count()) {
        errors.push(field("reason", "Say why, in 3 to 200 characters"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let mut tx = db.begin().await?;
    let before = repo::balance_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave balance"))?;
    if input.entitled_halfdays < before.used_halfdays {
        return Err(AppError::Validation(vec![field(
            "days",
            "That's less than what's already used",
        )]));
    }
    repo::set_entitled(&mut tx, id, input.entitled_halfdays, &time::to_db(now)).await?;
    let after = repo::balance_by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Leave balance"))?;
    let entry = Entry {
        action: "leave.adjust",
        entity: Some(("leave_balance", id)),
        before: Some(json!({ "entitledHalfdays": before.entitled_halfdays })),
        after: Some(json!({
            "entitledHalfdays": after.entitled_halfdays,
            "employeeId": after.employee_id,
            "type": after.leave_type_code,
            "year": after.year,
            "reason": reason,
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap()
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    /// 1: hired 2020 (gets SIL); 2: hired this year (no SIL yet); 3: resigned;
    /// 4: archived; 5: starts next month.
    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("hr");
        drop(conn);
        sqlx::query(
            "INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, archived_at) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2020-03-02', 'REGULAR', NULL), \
             (2, 'EMP-2', 'Ana', 'Reyes', '2026-02-02', 'PROBATIONARY', NULL), \
             (3, 'EMP-3', 'Leo', 'Santos', '2019-01-07', 'RESIGNED', NULL), \
             (4, 'EMP-4', 'Old', 'Hand', '2018-01-08', 'REGULAR', '2026-01-01T00:00:00Z'), \
             (5, 'EMP-5', 'New', 'Hire', '2026-11-02', 'PROBATIONARY', NULL);",
        )
        .execute(&db)
        .await
        .expect("fixtures");
        (dir, db)
    }

    fn codes(balances: &[LeaveBalance]) -> Vec<&str> {
        balances
            .iter()
            .map(|b| b.leave_type_code.as_str())
            .collect()
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_seeded_types_match_the_plan() {
        let (_d, db) = db().await;
        let types = list_types(&db).await.expect("types");
        let sil = types.iter().find(|t| t.code == "SIL").expect("SIL");
        assert_eq!(
            (sil.default_halfdays_per_year, sil.min_service_months),
            (10, 12)
        );
        let lwop = types.iter().find(|t| t.code == "LWOP").expect("LWOP");
        assert!(!lwop.is_paid);
        assert_eq!(types.len(), 4);
    }

    #[tokio::test]
    async fn the_grant_gives_current_employees_their_days() {
        let (_d, db) = db().await;
        let added = grant(&db, hr(), 2026, today(), t0()).await.expect("grant");
        // Juan: SIL, SL, VL. Ana: SL, VL (under a year, so no SIL yet).
        assert_eq!(added, 5);
        let juan = repo::balances(&db, 1, 2026).await.expect("juan");
        assert_eq!(codes(&juan), ["SIL", "SL", "VL"]);
        assert!(juan
            .iter()
            .all(|b| b.entitled_halfdays == 10 && b.used_halfdays == 0));
        assert_eq!(
            codes(&repo::balances(&db, 2, 2026).await.expect("ana")),
            ["SL", "VL"]
        );
        for nobody in [3, 4, 5] {
            assert!(repo::balances(&db, nobody, 2026)
                .await
                .expect("none")
                .is_empty());
        }
        // Running it again adds nothing.
        assert_eq!(
            grant(&db, hr(), 2026, today(), t0()).await.expect("again"),
            0
        );
        assert_eq!(
            audit_rows::actions(&db).await,
            ["leave.grant", "leave.grant"]
        );
    }

    #[tokio::test]
    async fn a_rerun_after_an_anniversary_adds_sil() {
        let (_d, db) = db().await;
        grant(&db, hr(), 2026, today(), t0()).await.expect("grant");
        sqlx::query("UPDATE employees SET hire_date = '2025-10-07' WHERE id = 2")
            .execute(&db)
            .await
            .expect("anniversary today");
        assert_eq!(
            grant(&db, hr(), 2026, today(), t0()).await.expect("again"),
            1
        );
    }

    #[tokio::test]
    async fn the_grant_year_is_this_year_or_next() {
        let (_d, db) = db().await;
        let err = grant(&db, hr(), 2025, today(), t0()).await.unwrap_err();
        assert_eq!(fields(err), ["year"]);
        // Next year counts service to January 1, so the November hire gets no SIL.
        grant(&db, hr(), 2027, today(), t0())
            .await
            .expect("next year");
        assert_eq!(
            codes(&repo::balances(&db, 5, 2027).await.expect("new")),
            ["SL", "VL"]
        );
    }

    #[tokio::test]
    async fn looking_at_this_years_balances_grants_them() {
        let (_d, db) = db().await;
        let b = balances(&db, hr(), 1, 2026, today(), t0())
            .await
            .expect("balances");
        assert_eq!(codes(&b), ["SIL", "SL", "VL"]);
        // Only Juan's were added, and a second look adds nothing.
        assert!(repo::balances(&db, 2, 2026).await.expect("ana").is_empty());
        balances(&db, hr(), 1, 2026, today(), t0())
            .await
            .expect("again");
        assert_eq!(audit_rows::actions(&db).await, ["leave.grant"]);
        // Another year is only read.
        assert!(balances(&db, hr(), 1, 2025, today(), t0())
            .await
            .expect("last year")
            .is_empty());
    }

    #[tokio::test]
    async fn inactive_and_unpaid_types_are_not_granted() {
        let (_d, db) = db().await;
        let vl = list_types(&db)
            .await
            .expect("types")
            .into_iter()
            .find(|t| t.code == "VL")
            .expect("VL");
        set_type_active(&db, hr(), vl.id, false, t0())
            .await
            .expect("off");
        grant(&db, hr(), 2026, today(), t0()).await.expect("grant");
        assert_eq!(
            codes(&repo::balances(&db, 1, 2026).await.expect("juan")),
            ["SIL", "SL"]
        );
    }

    #[tokio::test]
    async fn leave_types_are_checked_and_codes_unique() {
        let (_d, db) = db().await;
        let input = |code: &str, paid: bool, days: i64| LeaveTypeInput {
            code: code.into(),
            name: "Birthday Leave".into(),
            is_paid: paid,
            default_halfdays_per_year: days,
            min_service_months: 0,
        };
        let created = create_type(&db, hr(), input(" bday ", true, 2), t0())
            .await
            .expect("create");
        assert_eq!(created.code, "BDAY");
        let err = create_type(&db, hr(), input("sil", true, 2), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["code"]);
        let err = create_type(&db, hr(), input("X", false, 2), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["code", "defaultDays"]);
        let edited = update_type(&db, hr(), created.id, input("BDAY", true, 4), t0())
            .await
            .expect("keep own code");
        assert_eq!(edited.default_halfdays_per_year, 4);
    }

    #[tokio::test]
    async fn hr_adjusts_a_balance_but_not_below_what_is_used() {
        let (_d, db) = db().await;
        grant(&db, hr(), 2026, today(), t0()).await.expect("grant");
        let vl = repo::balances(&db, 1, 2026)
            .await
            .expect("juan")
            .into_iter()
            .find(|b| b.leave_type_code == "VL")
            .expect("VL");
        sqlx::query("UPDATE leave_balances SET used_halfdays = 4 WHERE id = ?")
            .bind(vl.id)
            .execute(&db)
            .await
            .expect("use some");

        let low = BalanceAdjustment {
            entitled_halfdays: 3,
            reason: "Typo".into(),
        };
        let err = adjust(&db, hr(), vl.id, low, t0()).await.unwrap_err();
        assert_eq!(fields(err), ["days"]);
        let carry = BalanceAdjustment {
            entitled_halfdays: 16,
            reason: "3 days carried over from 2025".into(),
        };
        let after = adjust(&db, hr(), vl.id, carry, t0()).await.expect("adjust");
        assert_eq!(after.entitled_halfdays, 16);
        let rows = audit_rows::all(&db).await;
        let last = rows.last().expect("audit");
        assert_eq!(last.action, "leave.adjust");
        assert!(last
            .after_json
            .as_deref()
            .unwrap_or("")
            .contains("carried over"));
    }

    #[tokio::test]
    async fn the_database_refuses_a_negative_balance() {
        let (_d, db) = db().await;
        grant(&db, hr(), 2026, today(), t0()).await.expect("grant");
        let r = sqlx::query(
            "UPDATE leave_balances SET used_halfdays = entitled_halfdays + 1 WHERE employee_id = 1",
        )
        .execute(&db)
        .await;
        assert!(r.is_err(), "used above entitled must be refused");
    }
}
