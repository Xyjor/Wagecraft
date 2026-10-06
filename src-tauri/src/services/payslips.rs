//! Computing a pay period (plan §6.5): for everyone employed in it, gather the pay rate,
//! schedule, attendance, approved leave and overtime, and holidays; run the engine; and
//! save the payslips. Recompute deletes and rewrites them, so the same inputs always give
//! the same payslips. Approving and posting arrive in a later PR.

use crate::audit::{self, Actor, Entry};
use crate::domain::attendance_calc::{parse_work_days, HolidayKind, Interval};
use crate::domain::payroll::contributions::Cutoff;
use crate::domain::payroll::engine::{DayRecord, DayStatus, Line, OvertimeBlock, PayslipInput};
use crate::domain::payroll::inputs::{day_record, DayFacts, LeaveFacts, RecordFacts};
use crate::domain::payroll::payslip::compute_payslip;
use crate::domain::payroll::rates::PayBasis;
use crate::domain::payroll_period::{
    PayrollPeriod, PayrollRegister, PayslipDetail, PayslipLine, RegisterRow,
};
use crate::domain::schedule::ScheduleHistory;
use crate::error::AppError;
use crate::repositories::payslips::{self as repo, NewPayslip, ScopeEmployee};
use crate::repositories::{payroll_periods, rule_packs, schedules};
use crate::time;
use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashMap;

/// Days looked at before the period, to find how the employee's last work day went
/// (daily-paid holiday pay on the first days of the period needs it).
const LOOKBACK_DAYS: u64 = 7;

const FROZEN: &str = "This pay period is approved or posted, so it can't be recomputed.";

pub async fn compute(
    db: &SqlitePool,
    actor: Actor<'_>,
    period_id: i64,
    now: DateTime<Utc>,
) -> Result<PayrollRegister, AppError> {
    let mut tx = db.begin().await?;
    let period = payroll_periods::by_id(&mut *tx, period_id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    if !matches!(period.status.as_str(), "DRAFT" | "COMPUTED") {
        return Err(AppError::Conflict(FROZEN));
    }
    let rules = rule_packs::by_code(&mut tx, &period.rule_pack_code)
        .await?
        .ok_or_else(|| anyhow::anyhow!("rule pack {} is missing", period.rule_pack_code))?;
    let start = date(&period.period_start)?;
    let end = date(&period.period_end)?;
    let from = start - Days::new(LOOKBACK_DAYS);

    let holidays = holidays_by_date(&repo::holidays(&mut *tx, &text(from), &text(end)).await?);
    let employees = repo::in_scope(&mut *tx, &text(start), &text(end)).await?;
    repo::delete_for_period(&mut tx, period_id).await?;

    let mut count = 0;
    let mut net_cents = 0;
    for e in &employees {
        let (Some(basis), Some(rate_cents)) = (e.pay_basis.as_deref(), e.rate_cents) else {
            continue;
        };
        let gathered = gather(&mut tx, e, &period, from, &holidays).await?;
        let input = PayslipInput {
            basis: if basis == "DAILY" {
                PayBasis::Daily
            } else {
                PayBasis::Monthly
            },
            rate_cents,
            days_per_week: gathered.days_per_week,
            days: gathered.days,
            status_before_period: gathered.status_before_period,
            overtime: gathered.overtime,
            allowances: vec![],
            cutoff: if period.cutoff_no == 1 {
                Cutoff::First
            } else {
                Cutoff::Second
            },
            minimum_wage_earner: false,
            deductions: vec![],
            minimum_net_cents: 0,
        };
        let result = compute_payslip(&input, &rules);
        let mut warnings = gathered.warnings;
        warnings.extend(result.warnings.iter().cloned());
        let inputs = snapshot(&input, &period);
        let id = repo::insert(
            &mut tx,
            &NewPayslip {
                period_id,
                employee_id: e.id,
                pay_basis: basis,
                rate_cents,
                gross_cents: cents(result.gross),
                taxable_cents: cents(result.taxable),
                statutory_ee_cents: cents(result.statutory_ee),
                tax_cents: cents(result.withholding_tax),
                other_deductions_cents: cents(result.deductions),
                net_cents: cents(result.net),
                warnings_json: &serde_json::to_string(&warnings).map_err(anyhow::Error::from)?,
                inputs_json: &inputs.to_string(),
            },
        )
        .await?;
        let lines = result
            .lines
            .iter()
            .map(|l| line(l, false, input.basis))
            .chain(result.employer.iter().map(|l| line(l, true, input.basis)));
        for (order, l) in lines.enumerate() {
            repo::insert_line(&mut tx, id, order as i64, &l).await?;
        }
        count += 1;
        net_cents += cents(result.net);
    }

    let at = time::to_db(now);
    payroll_periods::mark_computed(&mut tx, period_id, actor.user_id, &at).await?;
    let entry = Entry {
        action: "payroll.compute",
        entity: Some(("payroll_period", period_id)),
        before: Some(json!({ "status": period.status })),
        after: Some(json!({
            "periodStart": period.period_start,
            "payslips": count,
            "netCents": net_cents,
        })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    register(db, period_id).await
}

pub async fn register(db: &SqlitePool, period_id: i64) -> Result<PayrollRegister, AppError> {
    let mut conn = db.acquire().await?;
    let period = payroll_periods::by_id(&mut *conn, period_id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    let rows = repo::for_period(&mut *conn, period_id)
        .await?
        .into_iter()
        .map(|p| RegisterRow {
            payslip_id: p.id,
            employee_no: p.employee_no,
            employee_name: p.employee_name,
            gross_cents: p.gross_cents,
            statutory_ee_cents: p.statutory_ee_cents,
            tax_cents: p.tax_cents,
            other_deductions_cents: p.other_deductions_cents,
            net_cents: p.net_cents,
            warnings: warnings(&p.warnings_json),
        })
        .collect();
    let skipped = if period.status == "DRAFT" {
        vec![]
    } else {
        repo::skipped(
            &mut *conn,
            period_id,
            &period.period_start,
            &period.period_end,
        )
        .await?
    };
    Ok(PayrollRegister {
        period,
        rows,
        skipped,
    })
}

pub async fn payslip(db: &SqlitePool, id: i64) -> Result<PayslipDetail, AppError> {
    let mut conn = db.acquire().await?;
    let p = repo::by_id(&mut *conn, id)
        .await?
        .ok_or(AppError::NotFound("Payslip"))?;
    let period = payroll_periods::by_id(&mut *conn, p.payroll_period_id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    Ok(PayslipDetail {
        id: p.id,
        period_start: period.period_start,
        period_end: period.period_end,
        employee_no: p.employee_no,
        employee_name: p.employee_name,
        pay_basis: p.pay_basis,
        rate_cents: p.rate_cents,
        lines: repo::lines(&mut *conn, id).await?,
        gross_cents: p.gross_cents,
        taxable_cents: p.taxable_cents,
        statutory_ee_cents: p.statutory_ee_cents,
        tax_cents: p.tax_cents,
        other_deductions_cents: p.other_deductions_cents,
        net_cents: p.net_cents,
        warnings: warnings(&p.warnings_json),
    })
}

/// What compute gathers for one employee.
struct Gathered {
    days_per_week: u8,
    /// The period's days only.
    days: Vec<DayRecord>,
    status_before_period: Option<DayStatus>,
    overtime: Vec<OvertimeBlock>,
    warnings: Vec<String>,
}

async fn gather(
    conn: &mut SqliteConnection,
    e: &ScopeEmployee,
    period: &PayrollPeriod,
    from: NaiveDate,
    holidays: &HashMap<NaiveDate, Vec<HolidayKind>>,
) -> Result<Gathered, AppError> {
    let start = date(&period.period_start)?;
    let end = date(&period.period_end)?;
    let (from_text, end_text) = (text(from), text(end));
    let schedule: ScheduleHistory<String> = schedules::work_days_history(conn, e.id).await?;
    let records: HashMap<NaiveDate, repo::DayRecordRow> =
        repo::attendance(&mut *conn, e.id, &from_text, &end_text)
            .await?
            .into_iter()
            .filter_map(|r| Some((date(&r.work_date).ok()?, r)))
            .collect();
    let leave = repo::approved_leave(&mut *conn, e.id, &from_text, &end_text).await?;
    let hired = date(&e.hire_date)?;
    let left = e.separation_date.as_deref().map(date).transpose()?;

    let mut warnings = Vec::new();
    let mut before: Vec<DayRecord> = Vec::new();
    let mut days: Vec<DayRecord> = Vec::new();
    for day in from.iter_days().take_while(|d| *d <= end) {
        let record = records.get(&day);
        let leave_on = leave
            .iter()
            .find(|l| l.start_date.as_str() <= text(day).as_str() && l.end_date >= text(day));
        // Outside employment, the schedule they started or ended on says which days
        // would have been work days.
        let schedule_day = match left {
            _ if day < hired => hired,
            Some(l) if day > l => l,
            _ => day,
        };
        let facts = DayFacts {
            date: day,
            scheduled: schedule
                .on(schedule_day)
                .is_some_and(|w| parse_work_days(w).contains(&day.weekday())),
            holidays: holidays.get(&day).cloned().unwrap_or_default(),
            employed: day >= hired && left.is_none_or(|l| day <= l),
            record: record.map(|r| RecordFacts {
                clocked_in: r.time_in.is_some(),
                late_minutes: r.late_minutes,
                undertime_minutes: r.undertime_minutes,
                worked_minutes: r.worked_minutes,
                night_minutes: r.night_minutes,
            }),
            leave: leave_on.map(|l| LeaveFacts {
                paid: l.is_paid,
                half_day: l.half_day,
            }),
        };
        let r = day_record(&facts);
        if day < start {
            before.push(r);
            continue;
        }
        if record.is_some_and(|r| r.time_in.is_some() && r.time_out.is_none()) {
            warnings.push(format!(
                "No time-out on {}, so no hours were counted that day.",
                day.format("%b %-d, %Y")
            ));
        }
        days.push(r);
    }

    let status_before_period = before
        .iter()
        .rev()
        .find(|d| d.scheduled && d.status != DayStatus::Off)
        .map(|d| d.status);
    let days_per_week = schedule
        .on(start)
        .or_else(|| schedule.on(end))
        .map(|w| parse_work_days(w).len() as u8)
        .unwrap_or(5);

    let mut overtime = Vec::new();
    for ot in repo::approved_overtime(&mut *conn, e.id, &period.period_start, &end_text).await? {
        let Some(day) = days.iter().find(|d| text(d.date) == ot.work_date) else {
            continue;
        };
        let night = match (wall(&ot.start_at), wall(&ot.end_at)) {
            (Some(a), Some(b)) => Interval::new(a, b).night_minutes().min(ot.minutes),
            _ => 0,
        };
        overtime.push(OvertimeBlock {
            day_type: day.day_type,
            minutes: ot.minutes,
            night_minutes: night,
        });
    }

    Ok(Gathered {
        days_per_week,
        days,
        status_before_period,
        overtime,
        warnings,
    })
}

fn holidays_by_date(rows: &[(String, String)]) -> HashMap<NaiveDate, Vec<HolidayKind>> {
    let mut out: HashMap<NaiveDate, Vec<HolidayKind>> = HashMap::new();
    for (day, kind) in rows {
        if let (Ok(day), Some(kind)) = (date(day), HolidayKind::from_db(kind)) {
            out.entry(day).or_default().push(kind);
        }
    }
    out
}

/// A payslip line as stored.
fn line(l: &Line, employer: bool, basis: PayBasis) -> PayslipLine {
    let kind = if employer {
        "EMPLOYER_SHARE"
    } else if matches!(
        l.code,
        "SSS" | "PHILHEALTH" | "PAGIBIG" | "TAX" | "LOAN" | "DEDUCTION"
    ) {
        "DEDUCTION"
    } else {
        "EARNING"
    };
    let unit = match l.code {
        "BASIC" if basis == PayBasis::Daily => Some("DAYS"),
        "ABSENT" | "HOLIDAY_PAY" => Some("DAYS"),
        "LATE" | "UNDERTIME" => Some("MINUTES"),
        "HOLIDAY" | "REST_DAY" | "OT" | "NIGHT_DIFF" => Some("HOURS"),
        _ => None,
    };
    PayslipLine {
        kind: kind.into(),
        code: l.code.into(),
        label: l.label.clone(),
        quantity: l.quantity.normalize().to_string(),
        unit: unit.map(Into::into),
        amount_cents: cents(l.amount),
        taxable: l.taxable,
    }
}

/// What the engine was given, for explaining a payslip later.
fn snapshot(input: &PayslipInput, period: &PayrollPeriod) -> Value {
    json!({
        "rulePack": period.rule_pack_code,
        "basis": format!("{:?}", input.basis),
        "rateCents": input.rate_cents,
        "daysPerWeek": input.days_per_week,
        "cutoff": period.cutoff_no,
        "statusBeforePeriod": input.status_before_period.map(|s| format!("{s:?}")),
        "days": input.days.iter().map(|d| json!({
            "date": text(d.date),
            "dayType": format!("{:?}", d.day_type),
            "scheduled": d.scheduled,
            "status": format!("{:?}", d.status),
            "late": d.late_minutes,
            "undertime": d.undertime_minutes,
            "worked": d.worked_minutes,
            "night": d.night_minutes,
        })).collect::<Vec<_>>(),
        "overtime": input.overtime.iter().map(|o| json!({
            "dayType": format!("{:?}", o.day_type),
            "minutes": o.minutes,
            "night": o.night_minutes,
        })).collect::<Vec<_>>(),
    })
}

/// Pesos with two decimals as centavos.
fn cents(amount: Decimal) -> i64 {
    let mut a = amount.round_dp(2);
    a.rescale(2);
    a.mantissa() as i64
}

fn warnings(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

fn date(s: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::Internal(anyhow::anyhow!("bad date {s}: {e}")))
}

fn wall(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
}

fn text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::payroll_period::PayrollPeriodInput;
    use crate::services::payroll as periods;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, 1, 0, 0).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    async fn sql(db: &SqlitePool, q: &str) {
        sqlx::query(q).execute(db).await.expect(q);
    }

    /// A database with HR, a Mon–Fri 08:00–17:00 schedule and the Oct 16–31, 2026 period.
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
        sql(
            &pool,
            "INSERT INTO work_schedules (id, name, start_time, end_time, work_days) \
             VALUES (90, 'Test office', '08:00', '17:00', 'MON,TUE,WED,THU,FRI')",
        )
        .await;
        let pack: i64 = sqlx::query_scalar("SELECT id FROM rule_packs WHERE code = 'PH-2026'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let p = periods::create(
            &pool,
            hr(),
            PayrollPeriodInput {
                period_start: "2026-10-16".into(),
                pay_date: "2026-10-31".into(),
                rule_pack_id: pack,
            },
            now(),
        )
        .await
        .expect("period");
        (dir, pool, p.id)
    }

    /// An employee on the office schedule with a pay rate from Jan 1, 2025.
    async fn employee(db: &SqlitePool, id: i64, last: &str, hired: &str, basis: &str, rate: i64) {
        sql(
            db,
            &format!(
                "INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
                 employment_status) VALUES ({id}, 'EMP-{id:04}', 'Ana', '{last}', '{hired}', \
                 'REGULAR')"
            ),
        )
        .await;
        sql(
            db,
            &format!(
                "INSERT INTO schedule_assignments (employee_id, schedule_id, effective_from) \
                 VALUES ({id}, 90, '{hired}')"
            ),
        )
        .await;
        sql(
            db,
            &format!(
                "INSERT INTO compensations (employee_id, pay_basis, rate_cents, effective_from) \
                 VALUES ({id}, '{basis}', {rate}, '2025-01-01')"
            ),
        )
        .await;
    }

    /// A full 08:00–17:00 day on each listed date.
    async fn full_days(db: &SqlitePool, id: i64, dates: &[&str]) {
        for date in dates {
            sql(
                db,
                &format!(
                    "INSERT INTO attendance_records (employee_id, work_date, time_in, time_out, \
                     status, worked_minutes, source) VALUES ({id}, '{date}', \
                     '{date}T08:00:00', '{date}T17:00:00', 'PRESENT', 480, 'CLOCK')"
                ),
            )
            .await;
        }
    }

    /// Every Mon–Fri date of Oct 16–31, 2026.
    const WORK_DAYS: [&str; 11] = [
        "2026-10-16",
        "2026-10-19",
        "2026-10-20",
        "2026-10-21",
        "2026-10-22",
        "2026-10-23",
        "2026-10-26",
        "2026-10-27",
        "2026-10-28",
        "2026-10-29",
        "2026-10-30",
    ];

    fn work_days_except(skip: &[&str]) -> Vec<&'static str> {
        WORK_DAYS
            .iter()
            .copied()
            .filter(|d| !skip.contains(d))
            .collect()
    }

    fn amounts(p: &PayslipDetail) -> Vec<(String, i64)> {
        p.lines
            .iter()
            .map(|l| (l.code.clone(), l.amount_cents))
            .collect()
    }

    /// Plan §7.7: monthly ₱25,000, one absence, 30 minutes late, 3 hours of approved
    /// overtime. The amounts are golden case 01's, worked out by hand.
    #[tokio::test]
    async fn computes_the_worked_example_from_recorded_attendance() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&["2026-10-20", "2026-10-21"])).await;
        sql(
            &db,
            "INSERT INTO attendance_records (employee_id, work_date, time_in, time_out, status, \
             late_minutes, worked_minutes, source) VALUES (1, '2026-10-21', \
             '2026-10-21T08:30:00', '2026-10-21T17:00:00', 'PRESENT', 30, 450, 'CLOCK')",
        )
        .await;
        sql(
            &db,
            "INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, minutes, \
             reason, status) VALUES \
             (1, '2026-10-22', '2026-10-22T17:00:00', '2026-10-22T20:00:00', 180, 'x', 'APPROVED'), \
             (1, '2026-10-23', '2026-10-23T17:00:00', '2026-10-23T20:00:00', 180, 'x', 'PENDING')",
        )
        .await;

        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        assert_eq!(reg.period.status, "COMPUTED");
        assert_eq!(reg.rows.len(), 1);
        let row = &reg.rows[0];
        assert_eq!(row.employee_no, "EMP-0001");
        assert_eq!(row.gross_cents, 1_181_752);
        assert_eq!(row.statutory_ee_cents, 103_750);
        assert_eq!(row.tax_cents, 5_445);
        assert_eq!(row.net_cents, 1_072_557);
        assert!(row.warnings.is_empty(), "{:?}", row.warnings);

        let slip = payslip(&db, row.payslip_id).await.expect("payslip");
        assert_eq!(
            amounts(&slip),
            [
                ("BASIC".to_string(), 1_250_000),
                ("ABSENT".to_string(), -114_943),
                ("LATE".to_string(), -7_184),
                ("OT".to_string(), 53_879),
                ("SSS".to_string(), -62_500),
                ("PHILHEALTH".to_string(), -31_250),
                ("PAGIBIG".to_string(), -10_000),
                ("TAX".to_string(), -5_445),
                ("SSS_ER".to_string(), 125_000),
                ("SSS_EC".to_string(), 1_500),
                ("PHILHEALTH_ER".to_string(), 31_250),
                ("PAGIBIG_ER".to_string(), 10_000),
            ]
        );
        let line = |code: &str| slip.lines.iter().find(|l| l.code == code).unwrap();
        assert_eq!(
            (
                line("ABSENT").quantity.as_str(),
                line("ABSENT").unit.as_deref()
            ),
            ("1", Some("DAYS"))
        );
        assert_eq!(line("LATE").unit.as_deref(), Some("MINUTES"));
        assert_eq!(line("OT").quantity, "3");
        assert_eq!(line("OT").unit.as_deref(), Some("HOURS"));
        assert_eq!(line("BASIC").kind, "EARNING");
        assert_eq!(line("TAX").kind, "DEDUCTION");
        assert_eq!(line("SSS_ER").kind, "EMPLOYER_SHARE");
    }

    #[tokio::test]
    async fn recompute_replaces_the_payslips_with_the_same_result() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        let first = compute(&db, hr(), period, now()).await.expect("first");
        let again = compute(&db, hr(), period, now()).await.expect("again");
        assert_eq!(again.rows.len(), 1);
        assert_eq!(again.rows[0].net_cents, first.rows[0].net_cents);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payslips")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 1);
        let lines: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payslip_lines")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(
            lines as usize,
            payslip(&db, again.rows[0].payslip_id)
                .await
                .unwrap()
                .lines
                .len()
        );

        // A correction at the source shows after recompute.
        sql(
            &db,
            "DELETE FROM attendance_records WHERE work_date = '2026-10-19'",
        )
        .await;
        let corrected = compute(&db, hr(), period, now()).await.expect("corrected");
        assert!(corrected.rows[0].net_cents < first.rows[0].net_cents);
    }

    #[tokio::test]
    async fn holidays_off_and_paid_leave_are_not_absences() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&["2026-10-23", "2026-10-30"])).await;
        sql(
            &db,
            "INSERT INTO holidays (date, name, type) VALUES ('2026-10-30', 'Town fiesta', \
             'SPECIAL_NON_WORKING')",
        )
        .await;
        sql(
            &db,
            "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             halfdays, reason, status) SELECT 1, id, '2026-10-23', '2026-10-23', 2, 'trip', \
             'APPROVED' FROM leave_types WHERE is_paid = 1 LIMIT 1",
        )
        .await;
        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        let slip = payslip(&db, reg.rows[0].payslip_id).await.unwrap();
        assert!(
            !slip.lines.iter().any(|l| l.code == "ABSENT"),
            "{:?}",
            amounts(&slip)
        );
    }

    #[tokio::test]
    async fn monthly_staff_hired_mid_period_lose_the_days_before() {
        let (_d, db, period) = db().await;
        // Hired Wednesday Oct 21: Fri 16, Mon 19 and Tue 20 are deducted.
        employee(&db, 1, "Santos", "2026-10-21", "MONTHLY", 2_500_000).await;
        full_days(
            &db,
            1,
            &work_days_except(&["2026-10-16", "2026-10-19", "2026-10-20"]),
        )
        .await;
        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        let slip = payslip(&db, reg.rows[0].payslip_id).await.unwrap();
        let absent = slip
            .lines
            .iter()
            .find(|l| l.code == "ABSENT")
            .expect("absent");
        assert_eq!(absent.quantity, "3");
        assert_eq!(absent.amount_cents, -344_828);
    }

    #[tokio::test]
    async fn daily_staff_are_paid_for_the_days_they_worked() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Cruz", "2025-01-06", "DAILY", 64_500).await;
        full_days(&db, 1, &work_days_except(&["2026-10-20"])).await;
        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        let slip = payslip(&db, reg.rows[0].payslip_id).await.unwrap();
        let basic = slip.lines.iter().find(|l| l.code == "BASIC").unwrap();
        assert_eq!(
            (basic.quantity.as_str(), basic.amount_cents),
            ("10", 645_000)
        );
        assert_eq!(basic.unit.as_deref(), Some("DAYS"));
    }

    #[tokio::test]
    async fn lists_who_was_left_out_and_why() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        employee(&db, 2, "Reyes", "2025-01-06", "MONTHLY", 2_000_000).await;
        sql(&db, "DELETE FROM compensations WHERE employee_id = 2").await;
        employee(&db, 3, "Lim", "2025-01-06", "MONTHLY", 2_000_000).await;
        sql(
            &db,
            "UPDATE employees SET separation_date = '2026-10-15' WHERE id = 3",
        )
        .await;
        employee(&db, 4, "Tan", "2026-11-02", "MONTHLY", 2_000_000).await;

        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        let names: Vec<&str> = reg.rows.iter().map(|r| r.employee_name.as_str()).collect();
        assert_eq!(names, ["Santos, Ana"]);
        assert_eq!(reg.skipped.len(), 1);
        assert_eq!(reg.skipped[0].employee_name, "Reyes, Ana");
        assert_eq!(
            reg.skipped[0].reason,
            "No pay rate in effect for this period"
        );
    }

    #[tokio::test]
    async fn a_missing_time_out_is_flagged_on_the_payslip() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&["2026-10-19"])).await;
        sql(
            &db,
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source) \
             VALUES (1, '2026-10-19', '2026-10-19T08:00:00', 'PRESENT', 'CLOCK')",
        )
        .await;
        let reg = compute(&db, hr(), period, now()).await.expect("compute");
        assert_eq!(
            reg.rows[0].warnings,
            ["No time-out on Oct 19, 2026, so no hours were counted that day."]
        );
    }

    #[tokio::test]
    async fn approved_or_posted_periods_cannot_be_recomputed() {
        let (_d, db, period) = db().await;
        for status in ["APPROVED", "POSTED"] {
            sql(
                &db,
                &format!("UPDATE payroll_periods SET status = '{status}'"),
            )
            .await;
            let err = compute(&db, hr(), period, now()).await.unwrap_err();
            assert!(matches!(err, AppError::Conflict(_)), "{status}: {err:?}");
        }
        let err = compute(&db, hr(), 999, now()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[tokio::test]
    async fn a_draft_has_no_payslips_and_nobody_skipped() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        let reg = register(&db, period).await.expect("register");
        assert_eq!(reg.period.status, "DRAFT");
        assert!(reg.rows.is_empty() && reg.skipped.is_empty());
    }

    #[tokio::test]
    async fn computing_is_audited_and_marks_who_computed() {
        let (_d, db, period) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        full_days(&db, 1, &work_days_except(&[])).await;
        compute(&db, hr(), period, now()).await.expect("compute");
        let (by, at): (Option<i64>, Option<String>) =
            sqlx::query_as("SELECT computed_by, computed_at FROM payroll_periods")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!((by, at.as_deref()), (Some(1), Some("2026-11-02T01:00:00Z")));
        let rows = audit_rows::all(&db).await;
        let last = rows.last().unwrap();
        assert_eq!(last.action, "payroll.compute");
        assert_eq!(last.entity_id, Some(period));
        let after: serde_json::Value =
            serde_json::from_str(last.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(after["payslips"], 1);
    }
}
