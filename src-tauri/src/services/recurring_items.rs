//! An employee's recurring allowances, loans and other deductions (plan §5.3, §7.6).
//! Compute reads them fresh each time, so a change shows on the next compute or
//! recompute. Posted payslips keep what they used.

use crate::audit::{self, Actor, Entry};
use crate::domain::recurring_item::{RecurringItem, RecurringItemInput, KINDS, SCHEDULES};
use crate::error::{AppError, FieldError};
use crate::repositories::employees;
use crate::repositories::recurring_items::{self as repo, ItemFields};
use crate::services::auth::field;
use crate::services::org::MAX_RATE_CENTS;
use crate::time;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

const MAX_LABEL_LEN: usize = 60;
const ARCHIVED: &str = "Unarchive this employee before changing their allowances or deductions";

/// Allowances, then loans, then other deductions; oldest first within each.
pub async fn list(db: &SqlitePool, employee_id: i64) -> Result<Vec<RecurringItem>, AppError> {
    let mut conn = db.acquire().await?;
    employees::get(&mut *conn, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    Ok(repo::for_employee(&mut *conn, employee_id).await?)
}

pub async fn create(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    input: RecurringItemInput,
    now: DateTime<Utc>,
) -> Result<RecurringItem, AppError> {
    let mut tx = db.begin().await?;
    editable(&mut tx, employee_id).await?;
    let fields = validate(&input)?;
    let at = time::to_db(now);
    let id = repo::insert(&mut tx, employee_id, &fields, actor.user_id, &at).await?;
    let added = fetch(&mut tx, id).await?;
    let entry = Entry {
        action: "employee.recurring_item_create",
        entity: Some(("employee", employee_id)),
        before: None,
        after: Some(json!(added)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(added)
}

/// Everything but the kind can change: a loan stays a loan.
pub async fn update(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: RecurringItemInput,
    now: DateTime<Utc>,
) -> Result<RecurringItem, AppError> {
    let mut tx = db.begin().await?;
    let before = fetch(&mut tx, id).await?;
    editable(&mut tx, before.employee_id).await?;
    if input.kind != before.kind {
        let message = format!(
            "A {} stays a {}. Remove it and add a new item instead",
            noun(&before.kind),
            noun(&before.kind)
        );
        return Err(AppError::Validation(vec![field("kind", &message)]));
    }
    let fields = validate(&input)?;
    repo::update(&mut tx, id, &fields).await?;
    let after = fetch(&mut tx, id).await?;
    let entry = Entry {
        action: "employee.recurring_item_update",
        entity: Some(("employee", before.employee_id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = fetch(&mut tx, id).await?;
    editable(&mut tx, before.employee_id).await?;
    repo::delete(&mut tx, id).await?;
    let entry = Entry {
        action: "employee.recurring_item_delete",
        entity: Some(("employee", before.employee_id)),
        before: Some(json!(before)),
        after: None,
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

async fn fetch(conn: &mut SqliteConnection, id: i64) -> Result<RecurringItem, AppError> {
    repo::get(&mut *conn, id)
        .await?
        .ok_or(AppError::NotFound("Recurring item"))
}

/// The employee exists and isn't archived.
async fn editable(conn: &mut SqliteConnection, employee_id: i64) -> Result<(), AppError> {
    let employee = employees::get(&mut *conn, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(ARCHIVED));
    }
    Ok(())
}

fn validate(input: &RecurringItemInput) -> Result<ItemFields<'_>, AppError> {
    let mut errors: Vec<FieldError> = Vec::new();
    let kind_ok = KINDS.contains(&input.kind.as_str());
    if !kind_ok {
        errors.push(field("kind", "Pick allowance, loan or deduction"));
    }
    let label = input.label.trim();
    if label.is_empty() {
        errors.push(field("label", "Enter a name, such as Rice subsidy"));
    } else if label.chars().count() > MAX_LABEL_LEN {
        errors.push(field("label", "Keep the name under 60 characters"));
    }
    if !(1..=MAX_RATE_CENTS).contains(&input.amount_cents) {
        errors.push(field(
            "amountCents",
            "Enter an amount above ₱0 and up to ₱10,000,000",
        ));
    }
    if !SCHEDULES.contains(&input.schedule.as_str()) {
        errors.push(field("schedule", "Pick which cutoffs it applies to"));
    }
    let start = parse_date(&input.start_date);
    if start.is_none() {
        errors.push(field("startDate", "Enter the date it starts"));
    }
    let end_text = input
        .end_date
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty());
    match end_text.map(parse_date) {
        Some(None) => errors.push(field(
            "endDate",
            "Enter a valid end date, or leave it empty",
        )),
        Some(Some(end)) if start.is_some_and(|s| end < s) => {
            errors.push(field("endDate", "End on or after the start date"))
        }
        _ => {}
    }
    let loan = input.kind == "LOAN";
    if kind_ok && input.taxable && input.kind != "ALLOWANCE" {
        errors.push(field("taxable", "Only allowances can be taxable"));
    }
    if loan
        && !input
            .remaining_balance_cents
            .is_some_and(|b| (0..=MAX_RATE_CENTS).contains(&b))
    {
        errors.push(field("remainingBalanceCents", "Enter what is still owed"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    Ok(ItemFields {
        kind: &input.kind,
        label,
        amount_cents: input.amount_cents,
        taxable: input.taxable,
        schedule: &input.schedule,
        start_date: input.start_date.trim(),
        end_date: end_text,
        remaining_balance_cents: if loan {
            input.remaining_balance_cents
        } else {
            None
        },
    })
}

fn noun(kind: &str) -> &'static str {
    match kind {
        "ALLOWANCE" => "allowance",
        "LOAN" => "loan",
        _ => "deduction",
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
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

    fn input(kind: &str, label: &str, amount_cents: i64) -> RecurringItemInput {
        RecurringItemInput {
            kind: kind.into(),
            label: label.into(),
            amount_cents,
            taxable: false,
            schedule: "EVERY_CUTOFF".into(),
            start_date: "2026-10-16".into(),
            end_date: None,
            remaining_balance_cents: (kind == "LOAN").then_some(2_400_000),
        }
    }

    fn fields(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn adds_items_and_lists_them_by_kind() {
        let (_d, db, id) = db().await;
        create(&db, hr(), id, input("DEDUCTION", "Uniform", 50_000), now())
            .await
            .unwrap();
        let loan = create(
            &db,
            hr(),
            id,
            input("LOAN", " SSS salary loan ", 100_000),
            now(),
        )
        .await
        .unwrap();
        assert_eq!(loan.label, "SSS salary loan", "trimmed");
        assert_eq!(loan.remaining_balance_cents, Some(2_400_000));
        let mut rice = input("ALLOWANCE", "Rice subsidy", 100_000);
        rice.schedule = "SECOND_CUTOFF".into();
        rice.end_date = Some("2026-12-31".into());
        create(&db, hr(), id, rice, now()).await.unwrap();

        let items = list(&db, id).await.unwrap();
        let shape: Vec<_> = items
            .iter()
            .map(|i| (i.kind.as_str(), i.label.as_str(), i.schedule.as_str()))
            .collect();
        assert_eq!(
            shape,
            [
                ("ALLOWANCE", "Rice subsidy", "SECOND_CUTOFF"),
                ("LOAN", "SSS salary loan", "EVERY_CUTOFF"),
                ("DEDUCTION", "Uniform", "EVERY_CUTOFF"),
            ]
        );
        assert_eq!(items[0].end_date.as_deref(), Some("2026-12-31"));
    }

    #[tokio::test]
    async fn checks_every_field_at_once() {
        let (_d, db, id) = db().await;
        let bad = RecurringItemInput {
            kind: "BONUS".into(),
            label: "  ".into(),
            amount_cents: 0,
            taxable: false,
            schedule: "WEEKLY".into(),
            start_date: "Oct 16".into(),
            end_date: Some("2026-13-01".into()),
            remaining_balance_cents: None,
        };
        let names: Vec<_> = fields(create(&db, hr(), id, bad, now()).await.unwrap_err())
            .into_iter()
            .map(|(f, _)| f)
            .collect();
        assert_eq!(
            names,
            [
                "kind",
                "label",
                "amountCents",
                "schedule",
                "startDate",
                "endDate"
            ]
        );

        let mut long = input("ALLOWANCE", &"x".repeat(61), MAX_RATE_CENTS + 1);
        long.start_date = "2026-10-16".into();
        long.end_date = Some("2026-10-15".into());
        assert_eq!(
            fields(create(&db, hr(), id, long, now()).await.unwrap_err()),
            [
                ("label".into(), "Keep the name under 60 characters".into()),
                (
                    "amountCents".into(),
                    "Enter an amount above ₱0 and up to ₱10,000,000".into()
                ),
                ("endDate".into(), "End on or after the start date".into()),
            ]
        );
    }

    #[tokio::test]
    async fn a_loan_needs_its_balance_and_only_allowances_are_taxable() {
        let (_d, db, id) = db().await;
        let mut no_balance = input("LOAN", "Company loan", 100_000);
        no_balance.remaining_balance_cents = None;
        assert_eq!(
            fields(create(&db, hr(), id, no_balance, now()).await.unwrap_err()),
            [(
                "remainingBalanceCents".into(),
                "Enter what is still owed".into()
            )]
        );
        let mut taxed = input("DEDUCTION", "Uniform", 100_000);
        taxed.taxable = true;
        assert_eq!(
            fields(create(&db, hr(), id, taxed, now()).await.unwrap_err()),
            [("taxable".into(), "Only allowances can be taxable".into())]
        );

        // A balance on anything but a loan is dropped.
        let mut stray = input("DEDUCTION", "Uniform", 100_000);
        stray.remaining_balance_cents = Some(5);
        let saved = create(&db, hr(), id, stray, now()).await.unwrap();
        assert_eq!(saved.remaining_balance_cents, None);

        let mut paid_off = input("LOAN", "Old loan", 100_000);
        paid_off.remaining_balance_cents = Some(0);
        assert!(create(&db, hr(), id, paid_off, now()).await.is_ok());
    }

    #[tokio::test]
    async fn updates_everything_but_the_kind() {
        let (_d, db, id) = db().await;
        let loan = create(&db, hr(), id, input("LOAN", "Company loan", 100_000), now())
            .await
            .unwrap();
        let mut changed = input("LOAN", "Company loan (restructured)", 80_000);
        changed.remaining_balance_cents = Some(1_000_000);
        changed.end_date = Some("2027-06-30".into());
        let saved = update(&db, hr(), loan.id, changed, now()).await.unwrap();
        assert_eq!(
            (
                saved.label.as_str(),
                saved.amount_cents,
                saved.remaining_balance_cents,
                saved.end_date.as_deref()
            ),
            (
                "Company loan (restructured)",
                80_000,
                Some(1_000_000),
                Some("2027-06-30")
            )
        );

        let as_deduction = input("DEDUCTION", "Company loan", 80_000);
        assert_eq!(
            fields(
                update(&db, hr(), loan.id, as_deduction, now())
                    .await
                    .unwrap_err()
            ),
            [(
                "kind".into(),
                "A loan stays a loan. Remove it and add a new item instead".into()
            )]
        );
        assert!(matches!(
            update(&db, hr(), 999, input("LOAN", "x", 1), now()).await,
            Err(AppError::NotFound("Recurring item"))
        ));
    }

    #[tokio::test]
    async fn deletes_an_item() {
        let (_d, db, id) = db().await;
        let item = create(&db, hr(), id, input("ALLOWANCE", "Rice", 100), now())
            .await
            .unwrap();
        delete(&db, hr(), item.id, now()).await.unwrap();
        assert!(list(&db, id).await.unwrap().is_empty());
        assert!(matches!(
            delete(&db, hr(), item.id, now()).await,
            Err(AppError::NotFound("Recurring item"))
        ));
    }

    #[tokio::test]
    async fn every_change_is_audited_on_the_employee() {
        let (_d, db, id) = db().await;
        let item = create(&db, hr(), id, input("ALLOWANCE", "Rice", 100), now())
            .await
            .unwrap();
        update(&db, hr(), item.id, input("ALLOWANCE", "Rice", 200), now())
            .await
            .unwrap();
        delete(&db, hr(), item.id, now()).await.unwrap();

        let rows = audit_rows::all(&db).await;
        let ours: Vec<_> = rows
            .iter()
            .filter(|r| r.action.starts_with("employee.recurring_item"))
            .collect();
        let actions: Vec<_> = ours.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "employee.recurring_item_create",
                "employee.recurring_item_update",
                "employee.recurring_item_delete",
            ]
        );
        assert!(ours.iter().all(|r| r.entity_id == Some(id)));
        let amount = |json: Option<&str>| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(json.unwrap()).unwrap()["amountCents"].clone()
        };
        assert_eq!(amount(ours[1].before_json.as_deref()), 100);
        assert_eq!(amount(ours[1].after_json.as_deref()), 200);
        assert_eq!(amount(ours[2].before_json.as_deref()), 200);
        assert!(ours[2].after_json.is_none());
    }

    #[tokio::test]
    async fn refuses_changes_for_archived_or_missing_employees() {
        let (_d, db, id) = db().await;
        assert!(matches!(
            list(&db, 999).await,
            Err(AppError::NotFound("Employee"))
        ));
        assert!(matches!(
            create(&db, hr(), 999, input("ALLOWANCE", "Rice", 100), now()).await,
            Err(AppError::NotFound("Employee"))
        ));
        let item = create(&db, hr(), id, input("ALLOWANCE", "Rice", 100), now())
            .await
            .unwrap();
        sqlx::query("UPDATE employees SET archived_at = '2026-10-07T00:00:00Z'")
            .execute(&db)
            .await
            .unwrap();
        assert!(matches!(
            create(&db, hr(), id, input("ALLOWANCE", "Rice", 100), now()).await,
            Err(AppError::Conflict(ARCHIVED))
        ));
        assert!(matches!(
            update(&db, hr(), item.id, input("ALLOWANCE", "Rice", 200), now()).await,
            Err(AppError::Conflict(ARCHIVED))
        ));
        assert!(matches!(
            delete(&db, hr(), item.id, now()).await,
            Err(AppError::Conflict(ARCHIVED))
        ));
    }
}
