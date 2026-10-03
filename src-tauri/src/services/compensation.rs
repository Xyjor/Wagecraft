//! Salary history (plan §6.2, §6.11). Each change adds a row from the start of a pay period
//! and closes the previous row the day before, so payroll can always find the one rate in
//! effect for a period.

use crate::audit::{self, Actor, Entry};
use crate::domain::compensation::{pay_period_start, Compensation, CompensationInput, PAY_BASES};
use crate::error::{AppError, FieldError};
use crate::repositories::compensation::{self as repo, NewCompensation};
use crate::repositories::employees;
use crate::services::auth::field;
use crate::services::org::MAX_RATE_CENTS;
use crate::time;
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde_json::json;
use sqlx::SqlitePool;

const MAX_REASON_LEN: usize = 200;

/// Newest first. The first row with no end date is the rate in effect now.
pub async fn history(db: &SqlitePool, employee_id: i64) -> Result<Vec<Compensation>, AppError> {
    let mut conn = db.acquire().await?;
    employees::get(&mut *conn, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    Ok(repo::for_employee(&mut *conn, employee_id).await?)
}

pub async fn add(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    input: CompensationInput,
    now: DateTime<Utc>,
) -> Result<Compensation, AppError> {
    let mut tx = db.begin().await?;
    let employee = employees::get(&mut *tx, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "Unarchive this employee before changing their pay",
        ));
    }
    let current = repo::for_employee(&mut *tx, employee_id)
        .await?
        .into_iter()
        .next();

    let mut errors: Vec<FieldError> = Vec::new();
    if !PAY_BASES.contains(&input.pay_basis.as_str()) {
        errors.push(field("payBasis", "Pick monthly or daily"));
    }
    if !(1..=MAX_RATE_CENTS).contains(&input.rate_cents) {
        errors.push(field(
            "rateCents",
            "Enter an amount above ₱0 and up to ₱10,000,000",
        ));
    }
    let from = parse_date(&input.effective_from);
    match from {
        None => errors.push(field("effectiveFrom", "Enter the date the rate starts")),
        Some(from) => {
            if let Some(message) = start_problem(
                from,
                &employee.hire_date,
                &employee.separation_date,
                current.as_ref(),
            ) {
                errors.push(field("effectiveFrom", &message));
            }
        }
    }
    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    if reason.is_some_and(|r| r.chars().count() > MAX_REASON_LEN) {
        errors.push(field("reason", "Keep the reason under 200 characters"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let from = from.expect("checked above");

    // TODO(Phase 4): also refuse a start inside a computed, approved or posted payroll
    // period (plan §6.11) once payroll periods exist.
    if let Some(c) = &current {
        let day_before = from - Days::new(1);
        repo::close(&mut tx, c.id, &day_before.format("%Y-%m-%d").to_string()).await?;
    }
    let from_text = from.format("%Y-%m-%d").to_string();
    let at = time::to_db(now);
    let id = repo::insert(
        &mut tx,
        &NewCompensation {
            employee_id,
            pay_basis: &input.pay_basis,
            rate_cents: input.rate_cents,
            effective_from: &from_text,
            reason,
            created_by: actor.user_id,
            created_at: &at,
        },
    )
    .await?;
    let added = repo::get(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Compensation"))?;
    let entry = Entry {
        action: "employee.compensation_add",
        entity: Some(("employee", employee_id)),
        before: current.map(|c| json!(c)),
        after: Some(json!(added)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(added)
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
}

fn show(d: NaiveDate) -> String {
    d.format("%b %-d, %Y").to_string()
}

/// Why `from` can't start a new rate, if it can't. Payroll takes one rate per payslip,
/// so a rate must start on the first day of a pay period.
fn start_problem(
    from: NaiveDate,
    hire_date: &str,
    separation_date: &Option<String>,
    current: Option<&Compensation>,
) -> Option<String> {
    if pay_period_start(from) != from {
        return Some("Start on the 1st or 16th, the first day of a pay period".into());
    }
    if let Some(hired) = parse_date(hire_date) {
        let first = pay_period_start(hired);
        if from < first {
            return Some(format!(
                "Start on or after {}, the pay period they were hired in",
                show(first)
            ));
        }
    }
    if let Some(left) = separation_date.as_deref().and_then(parse_date) {
        if from > left {
            return Some(format!(
                "Start on or before their separation date, {}",
                show(left)
            ));
        }
    }
    if let Some(c) = current {
        if parse_date(&c.effective_from).is_some_and(|cur| from <= cur) {
            return Some(format!(
                "Start after the current rate, which began {}",
                parse_date(&c.effective_from).map(show).unwrap_or_default()
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::employee::EmployeeInput;
    use crate::services::employees as employee_service;
    use chrono::TimeZone;

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
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        drop(conn);
        let maria = EmployeeInput {
            employee_no: "EMP-0001".into(),
            first_name: "Maria".into(),
            last_name: "Santos".into(),
            hire_date: "2025-01-06".into(),
            employment_status: "REGULAR".into(),
            ..Default::default()
        };
        let today = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let e = employee_service::create(&pool, hr(), maria, today, now())
            .await
            .expect("employee");
        (dir, pool, e.id)
    }

    fn monthly(rate_cents: i64, from: &str) -> CompensationInput {
        CompensationInput {
            pay_basis: "MONTHLY".into(),
            rate_cents,
            effective_from: from.into(),
            reason: None,
        }
    }

    fn fields(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_first_rate_starts_the_history() {
        let (_dir, db, id) = db().await;
        let c = add(&db, hr(), id, monthly(2_500_000, "2025-01-01"), now())
            .await
            .expect("add");
        assert_eq!(c.rate_cents, 2_500_000);
        assert_eq!(c.effective_to, None);
        assert_eq!(c.created_by_name.as_deref(), Some("hr"));
        assert_eq!(history(&db, id).await.expect("history").len(), 1);
    }

    #[tokio::test]
    async fn a_raise_closes_the_previous_rate_the_day_before() {
        let (_dir, db, id) = db().await;
        add(&db, hr(), id, monthly(2_500_000, "2025-01-01"), now())
            .await
            .expect("first");
        let raise = CompensationInput {
            reason: Some("  Annual review ".into()),
            ..monthly(2_800_000, "2026-03-01")
        };
        add(&db, hr(), id, raise, now()).await.expect("raise");

        let rows = history(&db, id).await.expect("history");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rate_cents, 2_800_000);
        assert_eq!(rows[0].effective_to, None);
        assert_eq!(rows[0].reason.as_deref(), Some("Annual review"));
        assert_eq!(rows[1].rate_cents, 2_500_000);
        assert_eq!(rows[1].effective_to.as_deref(), Some("2026-02-28"));
    }

    #[tokio::test]
    async fn rates_start_on_the_first_day_of_a_pay_period() {
        let (_dir, db, id) = db().await;
        let err = add(&db, hr(), id, monthly(2_500_000, "2025-01-06"), now())
            .await
            .unwrap_err();
        assert_eq!(
            fields(err),
            [(
                "effectiveFrom".to_string(),
                "Start on the 1st or 16th, the first day of a pay period".to_string()
            )]
        );
        add(&db, hr(), id, monthly(2_500_000, "2025-01-16"), now())
            .await
            .expect("the 16th works");
    }

    #[tokio::test]
    async fn refuses_a_start_before_the_hire_period_or_not_after_the_current_rate() {
        let (_dir, db, id) = db().await;
        let err = add(&db, hr(), id, monthly(2_500_000, "2024-12-16"), now())
            .await
            .unwrap_err();
        assert_eq!(
            fields(err)[0].1,
            "Start on or after Jan 1, 2025, the pay period they were hired in"
        );

        add(&db, hr(), id, monthly(2_500_000, "2025-06-01"), now())
            .await
            .expect("first");
        for same_or_earlier in ["2025-06-01", "2025-05-16"] {
            let err = add(&db, hr(), id, monthly(2_600_000, same_or_earlier), now())
                .await
                .unwrap_err();
            assert_eq!(
                fields(err)[0].1,
                "Start after the current rate, which began Jun 1, 2025"
            );
        }
        assert_eq!(history(&db, id).await.expect("history").len(), 1);
    }

    #[tokio::test]
    async fn checks_every_field_at_once() {
        let (_dir, db, id) = db().await;
        let input = CompensationInput {
            pay_basis: "HOURLY".into(),
            rate_cents: 0,
            effective_from: "someday".into(),
            reason: Some("x".repeat(201)),
        };
        let names: Vec<String> = fields(add(&db, hr(), id, input, now()).await.unwrap_err())
            .into_iter()
            .map(|(f, _)| f)
            .collect();
        assert_eq!(names, ["payBasis", "rateCents", "effectiveFrom", "reason"]);
    }

    #[tokio::test]
    async fn audits_the_change_with_the_rate_it_replaced() {
        let (_dir, db, id) = db().await;
        add(&db, hr(), id, monthly(2_500_000, "2025-01-01"), now())
            .await
            .expect("first");
        add(&db, hr(), id, monthly(2_800_000, "2026-03-01"), now())
            .await
            .expect("raise");

        let rows = audit_rows::all(&db).await;
        let last = rows.last().expect("audit row");
        assert_eq!(last.action, "employee.compensation_add");
        assert_eq!(last.entity_id, Some(id));
        let before: serde_json::Value =
            serde_json::from_str(last.before_json.as_deref().unwrap()).unwrap();
        let after: serde_json::Value =
            serde_json::from_str(last.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(before["rateCents"], 2_500_000);
        assert_eq!(after["rateCents"], 2_800_000);
    }

    #[tokio::test]
    async fn refuses_changes_for_archived_or_missing_employees() {
        let (_dir, db, id) = db().await;
        sqlx::query("UPDATE employees SET archived_at = '2026-10-01T00:00:00Z' WHERE id = ?")
            .bind(id)
            .execute(&db)
            .await
            .unwrap();
        let err = add(&db, hr(), id, monthly(2_500_000, "2025-01-01"), now())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
        let err = add(&db, hr(), 999, monthly(2_500_000, "2025-01-01"), now())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }
}
