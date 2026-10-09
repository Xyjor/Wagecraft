//! The dashboards (plan §6.9): Admin and HR see the team, staff see their own.

use crate::domain::dashboard::{
    CutoffSummary, DepartmentCount, MyDashboard, MyDay, PeriodCost, TeamDashboard, TodayCounts,
};
use crate::domain::payroll_period::cutoff;
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

/// One employee's own dashboard, as of `today` on the office clock.
pub async fn mine(
    db: &SqlitePool,
    employee_id: i64,
    today: NaiveDate,
) -> Result<MyDashboard, AppError> {
    let iso = today.to_string();
    // Today reads the same as on HR's grid, schedule, holidays and leave included.
    let row = attendance::day(db, &iso)
        .await?
        .into_iter()
        .find(|r| r.employee_id == employee_id);
    let today_row = match row {
        Some(r) => MyDay {
            date: iso.clone(),
            status: r.status,
            time_in: r.record.as_ref().and_then(|x| x.time_in.clone()),
            time_out: r.record.as_ref().and_then(|x| x.time_out.clone()),
            late_minutes: r.record.as_ref().map_or(0, |x| x.late_minutes),
            holiday: r.holiday,
            leave: r.leave,
        },
        None => MyDay {
            date: iso.clone(),
            ..MyDay::default()
        },
    };

    let start = today
        .with_day(if today.day() <= 15 { 1 } else { 16 })
        .unwrap_or(today);
    let end = cutoff(start).map_or(today, |(_, end)| end);
    let (start, end) = (start.to_string(), end.to_string());
    let (days_present, late_minutes, overtime_minutes, pending_leave, pending_overtime): (
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT \
           (SELECT COUNT(*) FROM attendance_records WHERE employee_id = ?1 \
              AND status = 'PRESENT' AND work_date BETWEEN ?2 AND ?3), \
           (SELECT COALESCE(SUM(late_minutes), 0) FROM attendance_records \
              WHERE employee_id = ?1 AND work_date BETWEEN ?2 AND ?3), \
           (SELECT COALESCE(SUM(minutes), 0) FROM overtime_requests WHERE employee_id = ?1 \
              AND status = 'APPROVED' AND work_date BETWEEN ?2 AND ?4), \
           (SELECT COUNT(*) FROM leave_requests WHERE employee_id = ?1 \
              AND status = 'PENDING'), \
           (SELECT COUNT(*) FROM overtime_requests WHERE employee_id = ?1 \
              AND status = 'PENDING')",
    )
    .bind(employee_id)
    .bind(&start)
    .bind(&iso)
    .bind(&end)
    .fetch_one(db)
    .await?;

    Ok(MyDashboard {
        today: today_row,
        cutoff: CutoffSummary {
            period_start: start,
            period_end: end,
            days_present,
            late_minutes,
            overtime_minutes,
        },
        pending_leave,
        pending_overtime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Reyes (2) on Wed, Oct 28, 2026, in the Oct 16–31 cutoff, with Santos (1) as
    /// someone else whose numbers must not leak in.
    async fn my_db() -> (tempfile::TempDir, SqlitePool) {
        let (dir, db, _) = db().await;
        employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
        employee(&db, 2, "Reyes", "2025-01-06", "MONTHLY", 2_500_000).await;
        sql(
            &db,
            "INSERT INTO attendance_records (employee_id, work_date, time_in, time_out, \
               status, late_minutes, source) VALUES \
               (2, '2026-10-15', '2026-10-15T08:30:00', '2026-10-15T17:00:00', 'PRESENT', \
                30, 'CLOCK'), \
               (2, '2026-10-16', '2026-10-16T08:10:00', '2026-10-16T17:00:00', 'PRESENT', \
                10, 'CLOCK'), \
               (2, '2026-10-19', '2026-10-19T08:00:00', '2026-10-19T17:00:00', 'PRESENT', \
                0, 'CLOCK'), \
               (2, '2026-10-20', NULL, NULL, 'ABSENT', 0, 'MANUAL'), \
               (2, '2026-10-28', '2026-10-28T08:05:00', NULL, 'PRESENT', 5, 'CLOCK'), \
               (1, '2026-10-19', '2026-10-19T09:00:00', '2026-10-19T17:00:00', 'PRESENT', \
                60, 'CLOCK'); \
             INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, \
               minutes, reason, status) VALUES \
               (2, '2026-10-15', '2026-10-15T17:00:00', '2026-10-15T19:00:00', 120, \
                'Last cutoff', 'APPROVED'), \
               (2, '2026-10-19', '2026-10-19T17:00:00', '2026-10-19T18:30:00', 90, \
                'Stocktake', 'APPROVED'), \
               (2, '2026-10-26', '2026-10-26T17:00:00', '2026-10-26T18:00:00', 60, \
                'Stocktake', 'PENDING'), \
               (2, '2026-10-27', '2026-10-27T17:00:00', '2026-10-27T18:00:00', 60, \
                'Stocktake', 'REJECTED'), \
               (1, '2026-10-19', '2026-10-19T17:00:00', '2026-10-19T19:00:00', 120, \
                'Stocktake', 'APPROVED'); \
             INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
               halfdays, reason, status) VALUES \
               (2, (SELECT id FROM leave_types WHERE code = 'VL'), '2026-11-03', \
                '2026-11-03', 2, 'Errand', 'PENDING'), \
               (2, (SELECT id FROM leave_types WHERE code = 'SL'), '2026-10-21', \
                '2026-10-21', 2, 'Checkup', 'APPROVED'), \
               (1, (SELECT id FROM leave_types WHERE code = 'VL'), '2026-11-04', \
                '2026-11-04', 2, 'Errand', 'PENDING')",
        )
        .await;
        (dir, db)
    }

    #[tokio::test]
    async fn shows_my_day_so_far() {
        let (_d, db) = my_db().await;

        let d = mine(&db, 2, oct_28()).await.expect("dashboard");

        assert_eq!(
            d.today,
            MyDay {
                date: "2026-10-28".into(),
                status: Some("PRESENT".into()),
                time_in: Some("2026-10-28T08:05:00".into()),
                time_out: None,
                late_minutes: 5,
                holiday: None,
                leave: None,
            }
        );
    }

    #[tokio::test]
    async fn a_work_day_without_a_clock_in_reads_as_not_in_yet() {
        let (_d, db) = my_db().await;

        let d = mine(&db, 1, oct_28()).await.expect("dashboard");

        assert_eq!(d.today.status.as_deref(), Some("ABSENT"));
        assert_eq!(d.today.time_in, None);
    }

    #[tokio::test]
    async fn sums_only_my_own_cutoff_so_far() {
        let (_d, db) = my_db().await;

        let d = mine(&db, 2, oct_28()).await.expect("dashboard");

        // Oct 16, 19 and 28 are in this cutoff; Oct 15 was the last one and the 20th
        // was an absence. Santos's late hour and overtime are his own.
        assert_eq!(
            d.cutoff,
            CutoffSummary {
                period_start: "2026-10-16".into(),
                period_end: "2026-10-31".into(),
                days_present: 3,
                late_minutes: 15,
                overtime_minutes: 90,
            }
        );
        assert_eq!((d.pending_leave, d.pending_overtime), (1, 1));
    }

    #[tokio::test]
    async fn the_first_cutoff_runs_from_the_1st_to_the_15th() {
        let (_d, db) = my_db().await;
        let oct_15 = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();

        let d = mine(&db, 2, oct_15).await.expect("dashboard");

        assert_eq!(
            d.cutoff,
            CutoffSummary {
                period_start: "2026-10-01".into(),
                period_end: "2026-10-15".into(),
                days_present: 1,
                late_minutes: 30,
                overtime_minutes: 120,
            }
        );
    }

    #[tokio::test]
    async fn a_holiday_and_approved_leave_show_on_my_day() {
        let (_d, db) = my_db().await;
        sql(
            &db,
            "INSERT INTO holidays (date, name, type) VALUES \
               ('2026-11-02', 'All Souls'' Day', 'SPECIAL_NON_WORKING'); \
             UPDATE leave_requests SET status = 'APPROVED' WHERE employee_id = 2 \
               AND start_date = '2026-11-03'",
        )
        .await;
        let nov = |day| NaiveDate::from_ymd_opt(2026, 11, day).unwrap();

        let all_souls = mine(&db, 2, nov(2)).await.expect("dashboard");
        let errand = mine(&db, 2, nov(3)).await.expect("dashboard");

        assert_eq!(all_souls.today.status.as_deref(), Some("HOLIDAY"));
        assert_eq!(all_souls.today.holiday.as_deref(), Some("All Souls' Day"));
        assert_eq!(errand.today.status.as_deref(), Some("ON_LEAVE"));
        assert_eq!(errand.today.leave.as_deref(), Some("Vacation Leave"));
        assert_eq!(errand.pending_leave, 0);
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
