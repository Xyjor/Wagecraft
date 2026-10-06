//! Test setup shared by the payroll services' tests: HR, an office schedule, the
//! Oct 16–31, 2026 period, and helpers for employees, attendance and pay items.

use crate::audit::Actor;
use crate::domain::payroll_period::PayrollPeriodInput;
use crate::services::payroll as periods;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::SqlitePool;

pub(crate) fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 2, 1, 0, 0).unwrap()
}

pub(crate) fn hr() -> Actor<'static> {
    Actor {
        user_id: Some(1),
        username: "hr",
    }
}

pub(crate) async fn sql(db: &SqlitePool, q: &str) {
    sqlx::query(q).execute(db).await.expect(q);
}

/// A database with HR, a Mon–Fri 08:00–17:00 schedule and the Oct 16–31, 2026 period.
pub(crate) async fn db() -> (tempfile::TempDir, SqlitePool, i64) {
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
pub(crate) async fn employee(
    db: &SqlitePool,
    id: i64,
    last: &str,
    hired: &str,
    basis: &str,
    rate: i64,
) {
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
pub(crate) async fn full_days(db: &SqlitePool, id: i64, dates: &[&str]) {
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
pub(crate) const WORK_DAYS: [&str; 11] = [
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

pub(crate) fn work_days_except(skip: &[&str]) -> Vec<&'static str> {
    WORK_DAYS
        .iter()
        .copied()
        .filter(|d| !skip.contains(d))
        .collect()
}

/// A recurring item from Jan 1, 2026; a loan owes ₱100,000.
pub(crate) async fn item(
    db: &SqlitePool,
    id: i64,
    kind: &str,
    label: &str,
    amount: i64,
    end: &str,
) {
    let balance = if kind == "LOAN" { "10000000" } else { "NULL" };
    let end = if end.is_empty() {
        "NULL".to_string()
    } else {
        format!("'{end}'")
    };
    sql(
        db,
        &format!(
            "INSERT INTO recurring_items (employee_id, kind, label, amount_cents, taxable, \
             schedule, start_date, end_date, remaining_balance_cents, created_at) VALUES \
             ({id}, '{kind}', '{label}', {amount}, 0, 'EVERY_CUTOFF', '2026-01-01', {end}, \
             {balance}, '2026-01-01T00:00:00Z')"
        ),
    )
    .await;
}

/// The Nov 1–15, 2026 period, with a full day on each of its 10 work days.
pub(crate) async fn november(db: &SqlitePool, id: i64) -> i64 {
    let pack: i64 = sqlx::query_scalar("SELECT id FROM rule_packs WHERE code = 'PH-2026'")
        .fetch_one(db)
        .await
        .unwrap();
    let p = periods::create(
        db,
        hr(),
        PayrollPeriodInput {
            period_start: "2026-11-01".into(),
            pay_date: "2026-11-15".into(),
            rule_pack_id: pack,
        },
        now(),
    )
    .await
    .expect("november");
    let days = [
        "2026-11-02",
        "2026-11-03",
        "2026-11-04",
        "2026-11-05",
        "2026-11-06",
        "2026-11-09",
        "2026-11-10",
        "2026-11-11",
        "2026-11-12",
        "2026-11-13",
    ];
    full_days(db, id, &days).await;
    p.id
}

/// A payroll period from `start` to `end` that is already posted, with no attendance in it.
pub(crate) async fn posted(db: &SqlitePool, start: &str, end: &str) {
    sqlx::query(
        "INSERT INTO payroll_periods (period_start, period_end, pay_date, cutoff_no, status, \
         rule_pack_id, created_at) VALUES (?, ?, ?, 1, 'POSTED', \
         (SELECT id FROM rule_packs LIMIT 1), '2026-01-01T00:00:00Z')",
    )
    .bind(start)
    .bind(end)
    .bind(end)
    .execute(db)
    .await
    .expect("posted period");
}
