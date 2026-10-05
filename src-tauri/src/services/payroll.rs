//! Payroll runs (plan §6.5). This part creates and lists pay periods; computing,
//! approving and posting them arrive in later PRs. Every change is audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::payroll_period::{
    cutoff, PayrollPeriod, PayrollPeriodInput, PeriodChecks, RulePackSummary,
};
use crate::error::{AppError, FieldError};
use crate::repositories::payroll_periods::{self as repo, NewPeriod};
use crate::repositories::rule_packs;
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Datelike, Days, NaiveDate, Utc};
use serde_json::json;
use sqlx::SqlitePool;

/// Years a period may fall in. Anything outside is almost surely a typo.
const YEARS: std::ops::RangeInclusive<i32> = 2000..=2100;
/// Latest pay date after the period ends. Later than this is almost surely a typo.
const MAX_PAY_DELAY_DAYS: u64 = 31;

pub async fn list(db: &SqlitePool) -> Result<Vec<PayrollPeriod>, AppError> {
    Ok(repo::all(db).await?)
}

pub async fn rule_packs(db: &SqlitePool) -> Result<Vec<RulePackSummary>, AppError> {
    Ok(rule_packs::active(db).await?)
}

/// Pending leave and overtime, and missing time-outs, inside the period starting on
/// `period_start`.
pub async fn checks(db: &SqlitePool, period_start: &str) -> Result<PeriodChecks, AppError> {
    let (start, _, end) =
        period_of(period_start).ok_or_else(|| AppError::Validation(vec![bad_start()]))?;
    let (from, to) = (text(start), text(end));
    let mut conn = db.acquire().await?;
    Ok(PeriodChecks {
        pending_leave: repo::pending_leave(&mut *conn, &from, &to).await?,
        pending_overtime: repo::pending_overtime(&mut *conn, &from, &to).await?,
        missing_time_outs: repo::missing_time_outs(&mut *conn, &from, &to).await?,
    })
}

pub async fn create(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: PayrollPeriodInput,
    now: DateTime<Utc>,
) -> Result<PayrollPeriod, AppError> {
    let mut tx = db.begin().await?;
    let mut errors: Vec<FieldError> = Vec::new();

    let period = period_of(&input.period_start);
    match period {
        None => errors.push(bad_start()),
        Some((start, _, end)) => {
            if repo::by_start(&mut *tx, &text(start)).await?.is_some() {
                errors.push(field(
                    "periodStart",
                    &format!("There is already a pay period for {}", span(start, end)),
                ));
            }
        }
    }

    if let Some((_, _, end)) = period {
        let latest = end + Days::new(MAX_PAY_DELAY_DAYS);
        let pay = parse_date(&input.pay_date);
        if !pay.is_some_and(|p| p >= end && p <= latest) {
            errors.push(field(
                "payDate",
                &format!("Pick a pay date from {} to {}", show(end), show(latest)),
            ));
        }
    } else if parse_date(&input.pay_date).is_none() {
        errors.push(field("payDate", "Enter the pay date"));
    }

    let pack = rule_packs::active_by_id(&mut *tx, input.rule_pack_id).await?;
    let covers = |start: NaiveDate, end: NaiveDate| {
        pack.as_ref().is_some_and(|p| {
            parse_date(&p.effective_from).is_some_and(|f| f <= start)
                && p.effective_to
                    .as_deref()
                    .is_none_or(|t| parse_date(t).is_some_and(|t| t >= end))
        })
    };
    let pack_ok = match period {
        Some((start, _, end)) => covers(start, end),
        None => pack.is_some(),
    };
    if !pack_ok {
        errors.push(field(
            "rulePackId",
            "Pick a rule pack in effect for this period",
        ));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let (start, cutoff_no, end) = period.expect("checked above");

    let at = time::to_db(now);
    let id = repo::insert(
        &mut tx,
        &NewPeriod {
            period_start: &text(start),
            period_end: &text(end),
            pay_date: input.pay_date.trim(),
            cutoff_no,
            rule_pack_id: input.rule_pack_id,
            created_by: actor.user_id,
            created_at: &at,
        },
    )
    .await?;
    let created = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    let entry = Entry {
        action: "payroll.period_create",
        entity: Some(("payroll_period", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

/// Deletes a draft period, such as one created with the wrong dates.
pub async fn delete(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    if before.status != "DRAFT" {
        return Err(AppError::Conflict("Only a draft pay period can be deleted"));
    }
    repo::delete(&mut tx, id).await?;
    let entry = Entry {
        action: "payroll.period_delete",
        entity: Some(("payroll_period", id)),
        before: Some(json!(before)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

fn bad_start() -> FieldError {
    field(
        "periodStart",
        "Pick the 1st or the 16th of a month from 2000 to 2100",
    )
}

/// First day, cutoff number and last day of the period starting on `s`.
fn period_of(s: &str) -> Option<(NaiveDate, i64, NaiveDate)> {
    let start = parse_date(s).filter(|d| YEARS.contains(&d.year()))?;
    let (no, end) = cutoff(start)?;
    Some((start, no, end))
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
}

fn text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn show(d: NaiveDate) -> String {
    d.format("%b %-d, %Y").to_string()
}

/// "Oct 16 to 31, 2026".
fn span(start: NaiveDate, end: NaiveDate) -> String {
    format!("{} to {}", start.format("%b %-d"), end.format("%-d, %Y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
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

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = pool.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        (dir, pool)
    }

    async fn ph_2026(db: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT id FROM rule_packs WHERE code = 'PH-2026'")
            .fetch_one(db)
            .await
            .unwrap()
    }

    async fn input(db: &SqlitePool, start: &str, pay: &str) -> PayrollPeriodInput {
        PayrollPeriodInput {
            period_start: start.into(),
            pay_date: pay.into(),
            rule_pack_id: ph_2026(db).await,
        }
    }

    fn fields(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    async fn sql(db: &SqlitePool, q: &str) {
        sqlx::query(q).execute(db).await.expect(q);
    }

    #[tokio::test]
    async fn a_new_period_is_a_draft_covering_its_cutoff() {
        let (_d, db) = db().await;
        let p = create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-10-31").await,
            now(),
        )
        .await
        .expect("create");
        assert_eq!(p.period_start, "2026-10-16");
        assert_eq!(p.period_end, "2026-10-31");
        assert_eq!(p.pay_date, "2026-10-31");
        assert_eq!(p.cutoff_no, 2);
        assert_eq!(p.status, "DRAFT");
        assert_eq!(p.rule_pack_code, "PH-2026");
        assert_eq!(p.created_by_name.as_deref(), Some("hr"));
        assert_eq!(p.created_at, "2026-10-07T01:00:00Z");
    }

    #[tokio::test]
    async fn lists_periods_newest_first() {
        let (_d, db) = db().await;
        for (start, pay) in [
            ("2026-10-01", "2026-10-15"),
            ("2026-11-01", "2026-11-15"),
            ("2026-10-16", "2026-10-31"),
        ] {
            create(&db, hr(), input(&db, start, pay).await, now())
                .await
                .expect(start);
        }
        let starts: Vec<String> = list(&db)
            .await
            .expect("list")
            .into_iter()
            .map(|p| p.period_start)
            .collect();
        assert_eq!(starts, ["2026-11-01", "2026-10-16", "2026-10-01"]);
    }

    #[tokio::test]
    async fn a_period_starts_on_the_1st_or_the_16th() {
        let (_d, db) = db().await;
        for start in ["2026-10-02", "2026-10-15", "October", "1999-10-01"] {
            let err = create(&db, hr(), input(&db, start, "2026-10-31").await, now())
                .await
                .unwrap_err();
            assert_eq!(
                fields(err),
                [(
                    "periodStart".to_string(),
                    "Pick the 1st or the 16th of a month from 2000 to 2100".to_string()
                )],
                "{start}"
            );
        }
    }

    #[tokio::test]
    async fn the_pay_date_falls_from_the_last_day_to_31_days_after() {
        let (_d, db) = db().await;
        for pay in ["2026-10-14", "2026-11-16", "soon"] {
            let err = create(&db, hr(), input(&db, "2026-10-01", pay).await, now())
                .await
                .unwrap_err();
            assert_eq!(
                fields(err),
                [(
                    "payDate".to_string(),
                    "Pick a pay date from Oct 15, 2026 to Nov 15, 2026".to_string()
                )],
                "{pay}"
            );
        }
        create(
            &db,
            hr(),
            input(&db, "2026-10-01", "2026-11-15").await,
            now(),
        )
        .await
        .expect("31 days after works");
    }

    #[tokio::test]
    async fn each_cutoff_has_one_period() {
        let (_d, db) = db().await;
        create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-10-31").await,
            now(),
        )
        .await
        .expect("first");
        let err = create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-11-05").await,
            now(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            fields(err),
            [(
                "periodStart".to_string(),
                "There is already a pay period for Oct 16 to 31, 2026".to_string()
            )]
        );
    }

    #[tokio::test]
    async fn the_rule_pack_must_be_active_and_cover_the_whole_period() {
        let (_d, db) = db().await;
        let message = |p: &str| {
            [(
                "rulePackId".to_string(),
                format!("Pick a rule pack in effect for this period{p}"),
            )]
        };
        let err = create(
            &db,
            hr(),
            input(&db, "2025-12-16", "2025-12-31").await,
            now(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), message(""), "PH-2026 starts Jan 1, 2026");

        let mut missing = input(&db, "2026-10-01", "2026-10-15").await;
        missing.rule_pack_id = 999;
        let err = create(&db, hr(), missing, now()).await.unwrap_err();
        assert_eq!(fields(err), message(""));

        sql(&db, "UPDATE rule_packs SET is_active = 0").await;
        let err = create(
            &db,
            hr(),
            input(&db, "2026-10-01", "2026-10-15").await,
            now(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), message(""));
        assert!(rule_packs(&db).await.expect("packs").is_empty());

        sql(
            &db,
            "UPDATE rule_packs SET is_active = 1, effective_to = '2026-10-20'",
        )
        .await;
        let err = create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-10-31").await,
            now(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), message(""), "ends partway through the period");
    }

    #[tokio::test]
    async fn checks_every_field_at_once() {
        let (_d, db) = db().await;
        let bad = PayrollPeriodInput {
            period_start: "2026-10-03".into(),
            pay_date: "".into(),
            rule_pack_id: 0,
        };
        let names: Vec<String> = fields(create(&db, hr(), bad, now()).await.unwrap_err())
            .into_iter()
            .map(|(f, _)| f)
            .collect();
        assert_eq!(names, ["periodStart", "payDate", "rulePackId"]);
    }

    #[tokio::test]
    async fn lists_the_active_rule_packs() {
        let (_d, db) = db().await;
        let packs = rule_packs(&db).await.expect("packs");
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].code, "PH-2026");
        assert_eq!(packs[0].effective_from, "2026-01-01");
        assert_eq!(packs[0].effective_to, None);
    }

    #[tokio::test]
    async fn audits_creating_and_deleting() {
        let (_d, db) = db().await;
        let p = create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-10-31").await,
            now(),
        )
        .await
        .expect("create");
        delete(&db, hr(), p.id, now()).await.expect("delete");
        assert!(list(&db).await.expect("list").is_empty());

        let rows = audit_rows::all(&db).await;
        let actions: Vec<&str> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(actions, ["payroll.period_create", "payroll.period_delete"]);
        assert!(rows.iter().all(|r| r.entity_id == Some(p.id)));
        let after: serde_json::Value =
            serde_json::from_str(rows[0].after_json.as_deref().unwrap()).unwrap();
        assert_eq!(after["periodStart"], "2026-10-16");
        let before: serde_json::Value =
            serde_json::from_str(rows[1].before_json.as_deref().unwrap()).unwrap();
        assert_eq!(before["periodStart"], "2026-10-16");
    }

    #[tokio::test]
    async fn only_a_draft_can_be_deleted() {
        let (_d, db) = db().await;
        let p = create(
            &db,
            hr(),
            input(&db, "2026-10-16", "2026-10-31").await,
            now(),
        )
        .await
        .expect("create");
        for status in ["COMPUTED", "APPROVED", "POSTED"] {
            sql(
                &db,
                &format!("UPDATE payroll_periods SET status = '{status}'"),
            )
            .await;
            let err = delete(&db, hr(), p.id, now()).await.unwrap_err();
            assert!(matches!(err, AppError::Conflict(_)), "{status}: {err:?}");
        }
        assert_eq!(list(&db).await.expect("list").len(), 1);
        let err = delete(&db, hr(), 999, now()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    async fn employee(db: &SqlitePool, id: i64, no: &str, last: &str) {
        sql(
            db,
            &format!(
                "INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
                 employment_status) VALUES ({id}, '{no}', 'Ana', '{last}', '2025-01-06', 'REGULAR')"
            ),
        )
        .await;
    }

    #[tokio::test]
    async fn checks_list_unfinished_work_inside_the_period_only() {
        let (_d, db) = db().await;
        employee(&db, 1, "EMP-0001", "Reyes").await;
        employee(&db, 2, "EMP-0002", "Cruz").await;
        // Leave: one pending across the start, one pending after, one approved inside.
        sql(
            &db,
            "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             halfdays, reason, status) VALUES \
             (1, 1, '2026-10-14', '2026-10-17', 8, 'trip', 'PENDING'), \
             (2, 1, '2026-11-02', '2026-11-02', 2, 'later', 'PENDING'), \
             (2, 1, '2026-10-20', '2026-10-20', 2, 'ok', 'APPROVED')",
        )
        .await;
        sql(
            &db,
            "INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, minutes, \
             reason, status) VALUES \
             (2, '2026-10-31', '2026-10-31T17:00:00', '2026-10-31T19:00:00', 120, 'x', 'PENDING'), \
             (1, '2026-10-15', '2026-10-15T17:00:00', '2026-10-15T19:00:00', 120, 'x', 'PENDING'), \
             (1, '2026-10-20', '2026-10-20T17:00:00', '2026-10-20T19:00:00', 120, 'x', 'REJECTED')",
        )
        .await;
        sql(
            &db,
            "INSERT INTO attendance_records (employee_id, work_date, time_in, time_out, status, \
             source) VALUES \
             (1, '2026-10-19', '2026-10-19T08:00:00', NULL, 'PRESENT', 'CLOCK'), \
             (2, '2026-10-19', '2026-10-19T08:00:00', '2026-10-19T17:00:00', 'PRESENT', 'CLOCK'), \
             (2, '2026-11-01', '2026-11-01T08:00:00', NULL, 'PRESENT', 'CLOCK'), \
             (2, '2026-10-21', NULL, NULL, 'ABSENT', 'MANUAL')",
        )
        .await;

        let c = checks(&db, "2026-10-16").await.expect("checks");
        let show = |items: &[crate::domain::payroll_period::PendingItem]| -> Vec<String> {
            items
                .iter()
                .map(|i| format!("{} {} {}", i.date, i.employee_no, i.employee_name))
                .collect()
        };
        assert_eq!(show(&c.pending_leave), ["2026-10-14 EMP-0001 Reyes, Ana"]);
        assert_eq!(show(&c.pending_overtime), ["2026-10-31 EMP-0002 Cruz, Ana"]);
        assert_eq!(
            show(&c.missing_time_outs),
            ["2026-10-19 EMP-0001 Reyes, Ana"]
        );
    }

    #[tokio::test]
    async fn checks_need_a_period_start() {
        let (_d, db) = db().await;
        let err = checks(&db, "2026-10-05").await.unwrap_err();
        assert_eq!(fields(err)[0].0, "periodStart");
    }
}
