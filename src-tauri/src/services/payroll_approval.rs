//! The last steps of a pay period (plan §6.5): approve freezes the numbers, send back
//! unfreezes them with a reason, and post makes them final. Posting takes a backup first,
//! then locks the period's attendance, takes each loan's scheduled amount off its balance,
//! and releases the payslips to staff. A posted period is read-only forever.

use crate::audit::{self, Actor, Entry};
use crate::domain::payroll_period::PayrollPeriod;
use crate::error::AppError;
use crate::repositories::payroll_periods as repo;
use crate::repositories::{payslips, recurring_items};
use crate::services::auth::field;
use crate::services::backups;
use crate::time;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use std::path::Path;

const NOT_COMPUTED: &str = "Compute this pay period before approving it.";
const NO_PAYSLIPS: &str =
    "There are no payslips to approve. Check that employees have a pay rate, then recompute.";
const NOT_APPROVED_BACK: &str = "Only an approved pay period can be sent back.";
const NOT_APPROVED_POST: &str = "Approve this pay period before posting it.";
const NOT_ENDED: &str =
    "This pay period hasn't ended yet. Post it after its last day, once all the work is in.";
const EARLIER_FIRST: &str = "Post the earlier pay periods first.";
const STALE: &str = "An earlier pay period was posted after this one was computed. Send it \
    back and recompute so it includes what carried over.";

pub async fn approve(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    now: DateTime<Utc>,
) -> Result<PayrollPeriod, AppError> {
    let mut tx = db.begin().await?;
    let period = fetch(&mut tx, id).await?;
    if period.status != "COMPUTED" {
        return Err(AppError::Conflict(NOT_COMPUTED));
    }
    let slips = payslips::inputs_for_period(&mut *tx, id).await?.len();
    if slips == 0 {
        return Err(AppError::Conflict(NO_PAYSLIPS));
    }
    repo::mark_approved(&mut tx, id, actor.user_id, &time::to_db(now)).await?;
    let entry = Entry {
        action: "payroll.approve",
        entity: Some(("payroll_period", id)),
        before: Some(json!({ "status": period.status })),
        after: Some(json!({ "status": "APPROVED", "payslips": slips })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let approved = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(approved)
}

/// Back to computed, so HR can fix something at the source and recompute.
pub async fn send_back(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<PayrollPeriod, AppError> {
    let mut tx = db.begin().await?;
    let period = fetch(&mut tx, id).await?;
    if period.status != "APPROVED" {
        return Err(AppError::Conflict(NOT_APPROVED_BACK));
    }
    let reason = reason.trim();
    if !(3..=200).contains(&reason.chars().count()) {
        return Err(AppError::Validation(vec![field(
            "reason",
            "Say why, in 3 to 200 characters",
        )]));
    }
    repo::mark_sent_back(&mut tx, id).await?;
    let entry = Entry {
        action: "payroll.send_back",
        entity: Some(("payroll_period", id)),
        before: Some(json!({ "status": period.status })),
        after: Some(json!({ "status": "COMPUTED", "reason": reason })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let sent = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(sent)
}

/// `backup_dir` is where the pre-post backup goes, `local` names its file, and `today`
/// (the PC's date) must be after the period ends.
pub async fn post(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    backup_dir: &Path,
    local: NaiveDateTime,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<PayrollPeriod, AppError> {
    let mut conn = db.acquire().await?;
    let period = fetch(&mut conn, id).await?;
    ready_to_post(&mut conn, &period, today).await?;
    drop(conn);

    // The backup comes first and outside the transaction (VACUUM can't run inside one),
    // so a failed post leaves a copy of the database exactly as it was.
    let at = time::to_db(now);
    let backup = backups::create(db, backup_dir, "PRE_POST", actor.user_id, local, &at).await?;

    let mut tx = db.begin().await?;
    // Checked again inside the transaction, in case someone else got there first.
    let period = fetch(&mut tx, id).await?;
    ready_to_post(&mut tx, &period, today).await?;
    let locked =
        repo::lock_attendance(&mut tx, id, &period.period_start, &period.period_end).await?;
    let inputs = payslips::inputs_for_period(&mut *tx, id).await?;
    let mut loans = 0;
    for json in &inputs {
        for (item, amount) in scheduled_loans(json)? {
            recurring_items::reduce_balance(&mut tx, item, amount).await?;
            loans += 1;
        }
    }
    repo::mark_posted(&mut tx, id, actor.user_id, &at).await?;
    let file = backup
        .path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned());
    let entry = Entry {
        action: "payroll.post",
        entity: Some(("payroll_period", id)),
        before: Some(json!({ "status": period.status })),
        after: Some(json!({
            "status": "POSTED",
            "backup": file,
            "payslips": inputs.len(),
            "attendanceLocked": locked,
            "loanPayments": loans,
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    let posted = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(posted)
}

async fn fetch(conn: &mut SqliteConnection, id: i64) -> Result<PayrollPeriod, AppError> {
    repo::by_id(&mut *conn, id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))
}

/// Approved, ended, and every earlier period posted before this one was computed:
/// carryovers and loan balances come from posted periods, so posting goes in order.
async fn ready_to_post(
    conn: &mut SqliteConnection,
    period: &PayrollPeriod,
    today: NaiveDate,
) -> Result<(), AppError> {
    if period.status != "APPROVED" {
        return Err(AppError::Conflict(NOT_APPROVED_POST));
    }
    let end = NaiveDate::parse_from_str(&period.period_end, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("bad period end {}: {e}", period.period_end))?;
    if today <= end {
        return Err(AppError::Conflict(NOT_ENDED));
    }
    if repo::earlier_unposted(&mut *conn, &period.period_start).await? {
        return Err(AppError::Conflict(EARLIER_FIRST));
    }
    let computed = repo::computed_at(&mut *conn, period.id).await?;
    let last_posted = repo::last_posted_at_before(&mut *conn, &period.period_start).await?;
    if let (Some(computed), Some(posted)) = (computed, last_posted) {
        if computed < posted {
            return Err(AppError::Conflict(STALE));
        }
    }
    Ok(())
}

/// (loan, amount) for each loan a payslip scheduled itself, not counting carryovers:
/// their amount came off the balance when the payslip that scheduled them was posted.
fn scheduled_loans(inputs_json: &str) -> Result<Vec<(i64, i64)>, AppError> {
    let inputs: Value = serde_json::from_str(inputs_json).map_err(anyhow::Error::from)?;
    let deductions = inputs["deductions"].as_array().cloned().unwrap_or_default();
    Ok(deductions
        .iter()
        .filter(|d| d["kind"] == "Loan" && d["carryoverId"].is_null())
        .filter_map(|d| Some((d["recurringItemId"].as_i64()?, d["amountCents"].as_i64()?)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::attendance::AttendanceInput;
    use crate::services::payroll_fixtures::*;
    use crate::services::{attendance, payslips};
    use chrono::Duration;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
    }

    fn local() -> NaiveDateTime {
        today().and_hms_opt(9, 0, 0).unwrap()
    }

    fn fields(e: AppError) -> Vec<(String, String)> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| (f.field, f.message)).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn conflict(r: Result<PayrollPeriod, AppError>) -> &'static str {
        match r {
            Err(AppError::Conflict(m)) => m,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    /// October computed for one monthly employee with full attendance.
    async fn computed() -> (tempfile::TempDir, SqlitePool, i64) {
        let (dir, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        payslips::compute(&db, hr(), period, now()).await.unwrap();
        (dir, db, period)
    }

    async fn approved() -> (tempfile::TempDir, SqlitePool, i64) {
        let (dir, db, period) = computed().await;
        approve(&db, hr(), period, now()).await.unwrap();
        (dir, db, period)
    }

    async fn post_now(db: &SqlitePool, dir: &Path, period: i64) -> Result<PayrollPeriod, AppError> {
        post(
            db,
            hr(),
            period,
            &dir.join("backups"),
            local(),
            today(),
            now(),
        )
        .await
    }

    async fn status(db: &SqlitePool, period: i64) -> (String, Option<i64>, Option<String>) {
        sqlx::query_as("SELECT status, approved_by, approved_at FROM payroll_periods WHERE id = ?")
            .bind(period)
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn approving_freezes_the_numbers() {
        let (_d, db, period) = computed().await;
        let p = approve(&db, hr(), period, now()).await.unwrap();
        assert_eq!(p.status, "APPROVED");
        assert_eq!(
            status(&db, period).await,
            (
                "APPROVED".into(),
                Some(1),
                Some("2026-11-02T01:00:00Z".into())
            )
        );
        assert!(payslips::compute(&db, hr(), period, now()).await.is_err());
        let last = audit_rows::all(&db).await.pop().unwrap();
        assert_eq!(
            (last.action.as_str(), last.entity_id),
            ("payroll.approve", Some(period))
        );
    }

    #[tokio::test]
    async fn only_a_computed_period_with_payslips_can_be_approved() {
        let (_d, db, period) = db().await;
        assert_eq!(
            conflict(approve(&db, hr(), period, now()).await),
            "Compute this pay period before approving it."
        );
        // Computed, but nobody had a pay rate.
        payslips::compute(&db, hr(), period, now()).await.unwrap();
        assert_eq!(
            conflict(approve(&db, hr(), period, now()).await),
            "There are no payslips to approve. Check that employees have a pay rate, then \
             recompute."
        );
        assert!(matches!(
            approve(&db, hr(), 999, now()).await,
            Err(AppError::NotFound("Pay period"))
        ));
    }

    #[tokio::test]
    async fn sending_back_needs_a_reason_and_allows_a_recompute() {
        let (_d, db, period) = approved().await;
        assert_eq!(
            fields(
                send_back(&db, hr(), period, " no ", now())
                    .await
                    .unwrap_err()
            ),
            [("reason".into(), "Say why, in 3 to 200 characters".into())]
        );
        let p = send_back(&db, hr(), period, " Wrong OT for Santos ", now())
            .await
            .unwrap();
        assert_eq!(p.status, "COMPUTED");
        assert_eq!(status(&db, period).await, ("COMPUTED".into(), None, None));
        assert!(payslips::compute(&db, hr(), period, now()).await.is_ok());

        let rows = audit_rows::all(&db).await;
        let sent = rows
            .iter()
            .find(|r| r.action == "payroll.send_back")
            .unwrap();
        let after: serde_json::Value =
            serde_json::from_str(sent.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(after["reason"], "Wrong OT for Santos");

        assert_eq!(
            conflict(send_back(&db, hr(), period, "again", now()).await),
            "Only an approved pay period can be sent back."
        );
    }

    #[tokio::test]
    async fn posting_backs_up_locks_attendance_and_cuts_loan_balances() {
        let (dir, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        full_days(&db, 1, &["2026-11-02"]).await;
        // ₱20,000 a cutoff is more than net pay: part is taken, the rest carried over.
        // Either way the whole ₱20,000 comes off the balance; the carryover collects
        // the rest later.
        item(&db, 1, "LOAN", "Company loan", 2_000_000, "").await;
        item(&db, 1, "LOAN", "Small loan", 50_000, "").await;
        sql(
            &db,
            "UPDATE recurring_items SET remaining_balance_cents = 30000 WHERE label = 'Small loan'",
        )
        .await;
        payslips::compute(&db, hr(), period, now()).await.unwrap();
        approve(&db, hr(), period, now()).await.unwrap();

        let p = post_now(&db, dir.path(), period).await.expect("post");
        assert_eq!(p.status, "POSTED");
        let (by, at): (Option<i64>, Option<String>) =
            sqlx::query_as("SELECT posted_by, posted_at FROM payroll_periods WHERE id = ?")
                .bind(period)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!((by, at.as_deref()), (Some(1), Some("2026-11-02T01:00:00Z")));

        let backup = dir
            .path()
            .join("backups/wagecraft-backup-2026-11-02_0900.db");
        assert!(backup.exists());
        let kind: String = sqlx::query_scalar("SELECT kind FROM backup_history")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(kind, "PRE_POST");
        // The backup is from before posting.
        let copy = crate::db::open(&backup).await.unwrap();
        let before: String = sqlx::query_scalar("SELECT status FROM payroll_periods")
            .fetch_one(&copy)
            .await
            .unwrap();
        assert_eq!(before, "APPROVED");

        let locked: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT work_date, locked_by_period_id FROM attendance_records ORDER BY work_date",
        )
        .fetch_all(&db)
        .await
        .unwrap();
        assert_eq!(locked.len(), 12);
        assert!(locked[..11].iter().all(|(_, l)| *l == Some(period)));
        assert_eq!(
            locked[11],
            ("2026-11-02".into(), None),
            "outside the period"
        );

        let balances: Vec<(String, i64)> = sqlx::query_as(
            "SELECT label, remaining_balance_cents FROM recurring_items ORDER BY id",
        )
        .fetch_all(&db)
        .await
        .unwrap();
        assert_eq!(
            balances,
            [
                ("Company loan".into(), 10_000_000 - 2_000_000),
                ("Small loan".into(), 0),
            ]
        );

        let last = audit_rows::all(&db).await.pop().unwrap();
        assert_eq!(last.action, "payroll.post");
        let after: serde_json::Value =
            serde_json::from_str(last.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(after["backup"], "wagecraft-backup-2026-11-02_0900.db");
        assert_eq!(after["payslips"], 1);

        // Posted is final.
        assert!(payslips::compute(&db, hr(), period, now()).await.is_err());
        assert_eq!(
            conflict(send_back(&db, hr(), period, "too late", now()).await),
            "Only an approved pay period can be sent back."
        );
        assert_eq!(
            conflict(post_now(&db, dir.path(), period).await),
            "Approve this pay period before posting it."
        );
    }

    #[tokio::test]
    async fn a_carried_over_loan_does_not_cut_the_balance_twice() {
        let (dir, db, october) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        item(&db, 1, "LOAN", "Company loan", 2_000_000, "2026-10-31").await;
        payslips::compute(&db, hr(), october, now()).await.unwrap();
        approve(&db, hr(), october, now()).await.unwrap();
        post_now(&db, dir.path(), october).await.unwrap();

        // November takes October's carryover, which is already off the balance.
        let november = november(&db, 1).await;
        payslips::compute(&db, hr(), november, now()).await.unwrap();
        approve(&db, hr(), november, now()).await.unwrap();
        post(
            &db,
            hr(),
            november,
            &dir.path().join("backups"),
            local(),
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            now(),
        )
        .await
        .unwrap();
        let balance: i64 =
            sqlx::query_scalar("SELECT remaining_balance_cents FROM recurring_items")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(balance, 10_000_000 - 2_000_000);
    }

    #[tokio::test]
    async fn posting_waits_for_the_period_to_end_and_for_earlier_periods() {
        let (dir, db, october) = approved().await;
        let r = post(
            &db,
            hr(),
            october,
            &dir.path().join("backups"),
            local(),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
            now(),
        )
        .await;
        assert_eq!(conflict(r), NOT_ENDED);

        // November can't be posted while October isn't.
        sql(
            &db,
            "UPDATE payroll_periods SET status = 'COMPUTED' WHERE id = 1",
        )
        .await;
        let november = november(&db, 1).await;
        payslips::compute(&db, hr(), november, now()).await.unwrap();
        approve(&db, hr(), november, now()).await.unwrap();
        let late = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        let backups = dir.path().join("backups");
        let r = post(&db, hr(), november, &backups, local(), late, now()).await;
        assert_eq!(conflict(r), "Post the earlier pay periods first.");

        // October posted after November was computed: November must be recomputed to
        // pick up October's carryovers and loan balances.
        approve(&db, hr(), october, now()).await.unwrap();
        let later = now() + Duration::hours(1);
        post(&db, hr(), october, &backups, local(), late, later)
            .await
            .unwrap();
        let r = post(&db, hr(), november, &backups, local(), late, later).await;
        assert_eq!(
            conflict(r),
            "An earlier pay period was posted after this one was computed. Send it back and \
             recompute so it includes what carried over."
        );
        assert!(!std::fs::read_dir(&backups).unwrap().any(|f| f
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("-2.db")));
    }

    #[tokio::test]
    async fn attendance_in_a_posted_period_cannot_be_added() {
        let (dir, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        employee(&db, 2, "Reyes", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        payslips::compute(&db, hr(), period, now()).await.unwrap();
        approve(&db, hr(), period, now()).await.unwrap();
        post_now(&db, dir.path(), period).await.unwrap();

        // Reyes has no record on Oct 20 (an absence on the posted payslip).
        let input = |date: &str| AttendanceInput {
            employee_id: 2,
            work_date: date.into(),
            time_in: "08:00".into(),
            time_out: Some("17:00".into()),
            reason: "Forgot to clock in".into(),
        };
        let r = attendance::save(&db, hr(), input("2026-10-20"), today(), now()).await;
        assert!(
            matches!(r, Err(AppError::Conflict(m)) if m.contains("posted payroll period")),
            "{r:?}"
        );
        assert!(
            attendance::save(&db, hr(), input("2026-11-02"), today(), now())
                .await
                .is_ok()
        );
    }
}
