//! The Admin and HR dashboard (plan §6.9).

use crate::domain::dashboard::{DepartmentCount, PeriodCost, TeamDashboard, TodayCounts};
use crate::error::AppError;
use crate::services::attendance;
use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Everything the team dashboard shows, as of `today` on the office clock.
pub async fn team(db: &SqlitePool, today: NaiveDate) -> Result<TeamDashboard, AppError> {
    let iso = today.to_string();
    // The attendance day already knows who is current and how each day stands.
    let day = attendance::day(db, &iso).await?;

    let mut counts = TodayCounts::default();
    let mut departments: BTreeMap<Option<String>, i64> = BTreeMap::new();
    for row in &day {
        let late = row.record.as_ref().is_some_and(|r| r.late_minutes > 0);
        match row.status.as_deref() {
            Some("PRESENT") if late => counts.late += 1,
            Some("PRESENT") => counts.present += 1,
            Some("ABSENT") => counts.not_in += 1,
            Some("ON_LEAVE") => counts.on_leave += 1,
            _ => {}
        }
        *departments.entry(row.department_name.clone()).or_default() += 1;
    }
    let mut by_department: Vec<DepartmentCount> = departments
        .into_iter()
        .map(|(department, count)| DepartmentCount { department, count })
        .collect();
    by_department.sort_by_key(|d| {
        (
            d.department.is_none(),
            Reverse(d.count),
            d.department.clone(),
        )
    });

    let month_start = today.with_day(1).unwrap_or(today).to_string();
    let new_hires_this_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees \
         WHERE archived_at IS NULL AND hire_date BETWEEN ? AND ?",
    )
    .bind(&month_start)
    .bind(&iso)
    .fetch_one(db)
    .await?;
    let (pending_leave, pending_overtime): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM leave_requests WHERE status = 'PENDING'), \
                (SELECT COUNT(*) FROM overtime_requests WHERE status = 'PENDING')",
    )
    .fetch_one(db)
    .await?;
    let mut payroll_cost: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT p.period_start, p.period_end, \
           COALESCE((SELECT SUM(s.gross_cents) FROM payslips s \
                     WHERE s.payroll_period_id = p.id), 0), \
           COALESCE((SELECT SUM(l.amount_cents) FROM payslip_lines l \
                     JOIN payslips s ON s.id = l.payslip_id \
                     WHERE s.payroll_period_id = p.id AND l.kind = 'EMPLOYER_SHARE'), 0) \
         FROM payroll_periods p WHERE p.status = 'POSTED' \
         ORDER BY p.period_start DESC LIMIT 6",
    )
    .fetch_all(db)
    .await?;
    payroll_cost.reverse();

    Ok(TeamDashboard {
        headcount: day.len() as i64,
        new_hires_this_month,
        today: counts,
        pending_leave,
        pending_overtime,
        by_department,
        payroll_cost: payroll_cost
            .into_iter()
            .map(|(period_start, period_end, gross, employer)| PeriodCost {
                period_start,
                period_end,
                gross_cents: gross,
                employer_cents: employer,
                cost_cents: gross + employer,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dashboard::{DepartmentCount, PeriodCost, TodayCounts};
    use crate::services::payroll_fixtures::{db, employee, posted, sql};

    fn oct_28() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
    }

    /// Five current employees on Wed, Oct 28, 2026, one already gone and one not yet
    /// started.
    async fn team_db() -> (tempfile::TempDir, SqlitePool) {
        let (dir, db, _) = db().await;
        for (id, last, hired) in [
            (1, "Santos", "2025-01-06"),
            (2, "Reyes", "2025-01-06"),
            (3, "Cruz", "2025-01-06"),
            (4, "Lim", "2025-01-06"),
            (5, "Tan", "2026-10-05"),
            (6, "Go", "2025-01-06"),
            (7, "Uy", "2026-10-30"),
            (8, "Sy", "2026-09-30"),
        ] {
            employee(&db, id, last, hired, "MONTHLY", 2_500_000).await;
        }
        sql(
            &db,
            "INSERT INTO departments (id, code, name) VALUES (5, 'OPS', 'Operations'), \
               (6, 'PUR', 'Purchasing'); \
             UPDATE employees SET department_id = 5 WHERE id IN (1, 2, 4, 6, 7); \
             UPDATE employees SET department_id = 6 WHERE id = 3; \
             UPDATE employees SET separation_date = '2026-10-27', \
               employment_status = 'RESIGNED' WHERE id = 6; \
             INSERT INTO attendance_records (employee_id, work_date, time_in, status, \
               late_minutes, source) VALUES \
               (1, '2026-10-28', '2026-10-28T07:55:00', 'PRESENT', 0, 'CLOCK'), \
               (2, '2026-10-28', '2026-10-28T08:15:00', 'PRESENT', 15, 'CLOCK'), \
               (5, '2026-10-28', '2026-10-28T08:00:00', 'PRESENT', 0, 'CLOCK'), \
               (8, '2026-10-28', '2026-10-28T08:00:00', 'PRESENT', 0, 'CLOCK'); \
             INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
               halfdays, reason, status) VALUES \
               (4, (SELECT id FROM leave_types WHERE code = 'VL'), '2026-10-27', \
                '2026-10-29', 6, 'Trip', 'APPROVED'), \
               (1, (SELECT id FROM leave_types WHERE code = 'VL'), '2026-11-03', \
                '2026-11-03', 2, 'Errand', 'PENDING'), \
               (2, (SELECT id FROM leave_types WHERE code = 'SL'), '2026-11-04', \
                '2026-11-04', 2, 'Checkup', 'PENDING'), \
               (3, (SELECT id FROM leave_types WHERE code = 'SL'), '2026-11-05', \
                '2026-11-05', 2, 'Checkup', 'REJECTED'); \
             INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, \
               minutes, reason, status) VALUES \
               (1, '2026-10-27', '2026-10-27T17:00:00', '2026-10-27T19:00:00', 120, \
                'Stocktake', 'PENDING'), \
               (2, '2026-10-27', '2026-10-27T17:00:00', '2026-10-27T18:00:00', 60, \
                'Stocktake', 'CANCELLED')",
        )
        .await;
        (dir, db)
    }

    #[tokio::test]
    async fn counts_today_s_team_and_who_is_in() {
        let (_d, db) = team_db().await;

        let d = team(&db, oct_28()).await.expect("dashboard");

        // Santos, Reyes, Cruz, Lim, Tan and Sy. Go left yesterday and Uy starts on the 30th.
        assert_eq!(d.headcount, 6);
        assert_eq!(
            d.new_hires_this_month, 1,
            "Tan; Sy was hired last month and Uy hasn't started yet"
        );
        assert_eq!(
            d.today,
            TodayCounts {
                present: 3,
                late: 1,
                not_in: 1,
                on_leave: 1,
            }
        );
        assert_eq!((d.pending_leave, d.pending_overtime), (2, 1));
    }

    #[tokio::test]
    async fn groups_the_team_by_department_with_no_department_last() {
        let (_d, db) = team_db().await;

        let d = team(&db, oct_28()).await.expect("dashboard");

        let dept = |name: Option<&str>, count| DepartmentCount {
            department: name.map(str::to_string),
            count,
        };
        assert_eq!(
            d.by_department,
            [
                dept(Some("Operations"), 3),
                dept(Some("Purchasing"), 1),
                dept(None, 2),
            ]
        );
    }

    #[tokio::test]
    async fn payroll_cost_covers_the_last_six_posted_periods() {
        let (_d, db) = team_db().await;
        for (start, end) in [
            ("2026-07-01", "2026-07-15"),
            ("2026-07-16", "2026-07-31"),
            ("2026-08-01", "2026-08-15"),
            ("2026-08-16", "2026-08-31"),
            ("2026-09-01", "2026-09-15"),
            ("2026-09-16", "2026-09-30"),
            ("2026-10-01", "2026-10-15"),
        ] {
            posted(&db, start, end).await;
        }
        // Two payslips in Oct 1–15 and one in Jul 16–31. The Oct 16–31 period from the
        // fixture is still a draft, so it isn't a cost yet.
        sql(
            &db,
            "INSERT INTO payslips (id, payroll_period_id, employee_id, pay_basis, rate_cents, \
               gross_cents, taxable_cents, statutory_ee_cents, tax_cents, \
               other_deductions_cents, net_cents, inputs_json) VALUES \
               (1, (SELECT id FROM payroll_periods WHERE period_start = '2026-10-01'), 1, \
                'MONTHLY', 2500000, 1250000, 0, 0, 0, 0, 1100000, '{}'), \
               (2, (SELECT id FROM payroll_periods WHERE period_start = '2026-10-01'), 2, \
                'MONTHLY', 2000000, 1000000, 0, 0, 0, 0, 900000, '{}'), \
               (3, (SELECT id FROM payroll_periods WHERE period_start = '2026-07-16'), 1, \
                'MONTHLY', 2500000, 1250000, 0, 0, 0, 0, 1100000, '{}'); \
             INSERT INTO payslip_lines (payslip_id, kind, code, label, quantity, amount_cents, \
               taxable, sort_order) VALUES \
               (1, 'EMPLOYER_SHARE', 'SSS_ER', 'SSS (employer)', '1', 125000, 0, 1), \
               (1, 'EMPLOYER_SHARE', 'PHILHEALTH_ER', 'PhilHealth (employer)', '1', 31250, 0, 2), \
               (1, 'DEDUCTION', 'SSS', 'SSS', '1', -62500, 0, 3), \
               (2, 'EMPLOYER_SHARE', 'SSS_ER', 'SSS (employer)', '1', 100000, 0, 1), \
               (3, 'EARNING', 'BASIC', 'Basic pay', '1', 1250000, 1, 1)",
        )
        .await;

        let d = team(&db, oct_28()).await.expect("dashboard");

        let cost = |start: &str, end: &str, gross, employer| PeriodCost {
            period_start: start.into(),
            period_end: end.into(),
            gross_cents: gross,
            employer_cents: employer,
            cost_cents: gross + employer,
        };
        assert_eq!(
            d.payroll_cost,
            [
                cost("2026-07-16", "2026-07-31", 1_250_000, 0),
                cost("2026-08-01", "2026-08-15", 0, 0),
                cost("2026-08-16", "2026-08-31", 0, 0),
                cost("2026-09-01", "2026-09-15", 0, 0),
                cost("2026-09-16", "2026-09-30", 0, 0),
                cost("2026-10-01", "2026-10-15", 2_250_000, 256_250),
            ]
        );
    }

    #[tokio::test]
    async fn an_empty_company_has_zeros_and_no_charts() {
        let (_d, db, _) = db().await;

        let d = team(&db, oct_28()).await.expect("dashboard");

        assert_eq!((d.headcount, d.new_hires_this_month), (0, 0));
        assert_eq!(d.today, TodayCounts::default());
        assert!(d.by_department.is_empty());
        assert!(d.payroll_cost.is_empty());
    }

    #[tokio::test]
    async fn runs_on_the_demo_company() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::open(&dir.path().join("demo.db")).await.unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        crate::seed::demo_company(&db, 200, today)
            .await
            .expect("seed");

        let started = std::time::Instant::now();
        let d = team(&db, today).await.expect("dashboard");
        eprintln!("dashboard on 200 employees: {:?}", started.elapsed());

        assert!(d.headcount > 150, "{}", d.headcount);
        let by_dept: i64 = d.by_department.iter().map(|x| x.count).sum();
        assert_eq!(
            by_dept, d.headcount,
            "every current employee is in one group"
        );
    }
}
