//! Report files (plan §6.8): the employee masterlist and the payroll register.

use crate::audit::{self, Actor, Entry};
use crate::domain::employee::EmployeeQuery;
use crate::domain::payroll_period::{RegisterReport, RegisterReportRow};
use crate::domain::validation::{format_id, IdKind};
use crate::error::AppError;
use crate::export::csv;
use crate::repositories::employees::{self, MasterlistRow};
use crate::repositories::{payroll_periods, payslips};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;

const MASTERLIST_HEADERS: &[&str] = &[
    "Employee no.",
    "Last name",
    "First name",
    "Middle name",
    "Suffix",
    "Department",
    "Position",
    "Status",
    "Hire date",
    "Regularization date",
    "Separation date",
    "Birth date",
    "Sex",
    "Civil status",
    "Email",
    "Mobile",
    "Address",
    "TIN",
    "SSS no.",
    "PhilHealth no.",
    "Pag-IBIG MID",
    "Bank",
    "Account no.",
    "Pay basis",
    "Rate (PHP)",
    "Rate since",
    "Archived",
];

/// The masterlist for everyone matching the Employees screen's filters, as CSV bytes.
/// Exporting personal data is audited, with the filters used and the row count.
pub async fn masterlist_csv(
    db: &SqlitePool,
    actor: Actor<'_>,
    query: &EmployeeQuery,
    now: DateTime<Utc>,
) -> Result<Vec<u8>, AppError> {
    let mut tx = db.begin().await?;
    let rows = employees::masterlist(&mut tx, query).await?;
    let count = rows.len();
    let bytes = csv::build(MASTERLIST_HEADERS, rows.into_iter().map(masterlist_row))?;
    let entry = Entry {
        action: "report.export",
        after: Some(json!({ "report": "employee_masterlist", "rows": count, "filters": query })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(bytes)
}

fn masterlist_row(r: MasterlistRow) -> Vec<String> {
    let id = |kind, v: Option<String>| v.map(|d| format_id(kind, &d)).unwrap_or_default();
    let text = |v: Option<String>| v.unwrap_or_default();
    vec![
        r.employee_no,
        r.last_name,
        r.first_name,
        text(r.middle_name),
        text(r.suffix),
        text(r.department_name),
        text(r.position_title),
        title_case(&r.employment_status),
        r.hire_date,
        text(r.regularization_date),
        text(r.separation_date),
        text(r.birth_date),
        r.sex.as_deref().map(title_case).unwrap_or_default(),
        r.civil_status
            .as_deref()
            .map(title_case)
            .unwrap_or_default(),
        text(r.email),
        r.mobile.as_deref().map(spaced_mobile).unwrap_or_default(),
        text(r.address),
        id(IdKind::Tin, r.tin),
        id(IdKind::Sss, r.sss_no),
        id(IdKind::Philhealth, r.philhealth_no),
        id(IdKind::Pagibig, r.pagibig_no),
        text(r.bank_name),
        text(r.bank_account_no),
        r.pay_basis.as_deref().map(title_case).unwrap_or_default(),
        r.rate_cents.map(plain_amount).unwrap_or_default(),
        text(r.rate_from),
        if r.archived { "Yes" } else { "No" }.to_string(),
    ]
}

/// One period's register with each deduction in its own column and a totals row, for the
/// PDF the UI draws and for the CSV below.
pub async fn register_report(db: &SqlitePool, period_id: i64) -> Result<RegisterReport, AppError> {
    let mut conn = db.acquire().await?;
    let period = payroll_periods::by_id(&mut *conn, period_id)
        .await?
        .ok_or(AppError::NotFound("Pay period"))?;
    let rows = payslips::register_report_rows(&mut *conn, period_id).await?;
    if rows.is_empty() {
        return Err(AppError::Conflict(
            "This period has no payslips yet. Compute the payroll first",
        ));
    }
    let totals = rows.iter().fold(
        RegisterReportRow {
            employee_name: "Total".into(),
            ..Default::default()
        },
        |mut t, r| {
            t.gross_cents += r.gross_cents;
            t.sss_cents += r.sss_cents;
            t.philhealth_cents += r.philhealth_cents;
            t.pagibig_cents += r.pagibig_cents;
            t.tax_cents += r.tax_cents;
            t.other_deductions_cents += r.other_deductions_cents;
            t.net_cents += r.net_cents;
            t
        },
    );
    Ok(RegisterReport {
        period,
        company: payslips::company(&mut *conn).await?,
        rows,
        totals,
    })
}

/// The register as a CSV file name and bytes, with a totals row. Exporting pay data is
/// audited.
pub async fn register_csv(
    db: &SqlitePool,
    actor: Actor<'_>,
    period_id: i64,
    now: DateTime<Utc>,
) -> Result<(String, Vec<u8>), AppError> {
    let report = register_report(db, period_id).await?;
    let row = |r: &RegisterReportRow| {
        let mut cells = vec![r.employee_no.clone(), r.employee_name.clone()];
        cells.extend(
            [
                r.gross_cents,
                r.sss_cents,
                r.philhealth_cents,
                r.pagibig_cents,
                r.tax_cents,
                r.other_deductions_cents,
                r.net_cents,
            ]
            .map(plain_amount),
        );
        cells
    };
    let lines = report.rows.iter().chain([&report.totals]).map(row);
    let bytes = csv::build(REGISTER_HEADERS, lines)?;

    let mut tx = db.begin().await?;
    let entry = Entry {
        action: "report.export",
        after: Some(json!({
            "report": "payroll_register",
            "period": report.period.period_start,
            "rows": report.rows.len(),
        })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    let name = format!("payroll-register-{}.csv", report.period.period_start);
    Ok((name, bytes))
}

const REGISTER_HEADERS: &[&str] = &[
    "Employee no.",
    "Employee",
    "Gross pay",
    "SSS",
    "PhilHealth",
    "Pag-IBIG",
    "Withholding tax",
    "Other deductions",
    "Net pay",
];

/// "09171234567" → "0917 123 4567". The spaces keep Excel from reading it as a number and
/// dropping the leading zero.
fn spaced_mobile(m: &str) -> String {
    if m.len() == 11 && m.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {} {}", &m[..4], &m[4..7], &m[7..])
    } else {
        m.to_string()
    }
}

/// "PROBATIONARY" → "Probationary"
fn title_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// 2500050 → "25000.50": a plain number Excel can add up, from integer math only.
fn plain_amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::compensation::CompensationInput;
    use crate::domain::employee::{Archived, EmployeeInput};
    use crate::services::{compensation, employees as employee_service};
    use chrono::{NaiveDate, TimeZone};

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
        drop(conn);
        (dir, pool)
    }

    async fn add(db: &SqlitePool, no: &str, first: &str, last: &str, tin: Option<&str>) -> i64 {
        let input = EmployeeInput {
            employee_no: no.into(),
            first_name: first.into(),
            last_name: last.into(),
            hire_date: "2025-01-06".into(),
            employment_status: "REGULAR".into(),
            tin: tin.map(Into::into),
            mobile: Some("09171234567".into()),
            ..Default::default()
        };
        let today = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        employee_service::create(db, hr(), input, today, now())
            .await
            .expect("employee")
            .id
    }

    fn lines(bytes: &[u8]) -> Vec<String> {
        std::str::from_utf8(&bytes[3..])
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[tokio::test]
    async fn exports_filtered_rows_with_formatted_ids_and_rates() {
        let (_dir, db) = db().await;
        let maria = add(&db, "EMP-0001", "María", "Peñaflor", Some("123456789")).await;
        add(&db, "EMP-0002", "Juan", "Santos", None).await;
        let rate = CompensationInput {
            pay_basis: "MONTHLY".into(),
            rate_cents: 2_500_050,
            effective_from: "2025-01-01".into(),
            reason: None,
            minimum_wage_earner: false,
        };
        compensation::add(&db, hr(), maria, rate, now())
            .await
            .unwrap();

        let query = EmployeeQuery {
            search: Some("peña".into()),
            archived: Archived::Include,
            ..Default::default()
        };
        let bytes = masterlist_csv(&db, hr(), &query, now()).await.unwrap();
        let lines = lines(&bytes);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with("Employee no.,Last name,First name"));
        assert!(lines[1].starts_with("EMP-0001,Peñaflor,María,,,,,Regular,2025-01-06"));
        assert!(lines[1].contains(",123-456-789,"));
        assert!(lines[1].contains(",0917 123 4567,"));
        assert!(lines[1].ends_with(",Monthly,25000.50,2025-01-01,No"));
    }

    #[tokio::test]
    async fn neutralizes_formulas_and_audits_the_export() {
        let (_dir, db) = db().await;
        add(&db, "EMP-0001", "Juan", "Santos", None).await;
        sqlx::query("UPDATE employees SET address = '=HYPERLINK(\"http://x\")'")
            .execute(&db)
            .await
            .unwrap();
        let bytes = masterlist_csv(&db, hr(), &EmployeeQuery::default(), now())
            .await
            .unwrap();
        assert!(lines(&bytes)[1].contains("\"'=HYPERLINK(\"\"http://x\"\")\""));

        let rows = audit_rows::all(&db).await;
        let last = rows.last().unwrap();
        assert_eq!(last.action, "report.export");
        assert!(last.after_json.as_deref().unwrap().contains("\"rows\":1"));
    }

    #[test]
    fn plain_amounts_keep_the_sign_on_the_whole_number() {
        assert_eq!(plain_amount(2_500_050), "25000.50");
        assert_eq!(plain_amount(-114_943), "-1149.43");
        assert_eq!(plain_amount(-5), "-0.05");
        assert_eq!(plain_amount(0), "0.00");
    }

    mod register {
        use super::super::*;
        use crate::audit::test_support as audit_rows;
        use crate::services::payroll_fixtures::*;
        use crate::services::payslips;

        /// Santos is plan §7.7's worked example. Reyes worked every day on ₱25,000 a month
        /// and repays a ₱2,000 loan each cutoff.
        async fn computed() -> (tempfile::TempDir, SqlitePool, i64) {
            let (d, db, period) = db().await;
            employee(&db, 1, "Santos", "2025-01-06", "MONTHLY", 2_500_000).await;
            full_days(&db, 1, &work_days_except(&["2026-10-20", "2026-10-21"])).await;
            sql(
                &db,
                "INSERT INTO attendance_records (employee_id, work_date, time_in, time_out, \
                 status, late_minutes, worked_minutes, source) VALUES (1, '2026-10-21', \
                 '2026-10-21T08:30:00', '2026-10-21T17:00:00', 'PRESENT', 30, 450, 'CLOCK')",
            )
            .await;
            sql(
                &db,
                "INSERT INTO overtime_requests (employee_id, work_date, start_at, end_at, \
                 minutes, reason, status) VALUES (1, '2026-10-22', '2026-10-22T17:00:00', \
                 '2026-10-22T20:00:00', 180, 'x', 'APPROVED')",
            )
            .await;
            employee(&db, 2, "Reyes", "2025-01-06", "MONTHLY", 2_500_000).await;
            full_days(&db, 2, &WORK_DAYS).await;
            item(&db, 2, "LOAN", "Company loan", 200_000, "").await;
            payslips::compute(&db, hr(), period, now())
                .await
                .expect("compute");
            (d, db, period)
        }

        #[tokio::test]
        async fn splits_each_deduction_into_its_own_column_with_totals() {
            let (_d, db, period) = computed().await;
            let r = register_report(&db, period).await.expect("report");
            assert_eq!(r.period.id, period);

            let names: Vec<_> = r.rows.iter().map(|x| x.employee_name.as_str()).collect();
            assert_eq!(names, ["Reyes, Ana", "Santos, Ana"]);
            let santos = &r.rows[1];
            assert_eq!(
                (
                    santos.employee_no.as_str(),
                    santos.gross_cents,
                    santos.sss_cents,
                    santos.philhealth_cents,
                    santos.pagibig_cents,
                    santos.tax_cents,
                    santos.other_deductions_cents,
                    santos.net_cents,
                ),
                ("EMP-0001", 1_181_752, 62_500, 31_250, 10_000, 5_445, 0, 1_072_557)
            );
            let reyes = &r.rows[0];
            assert_eq!(
                (reyes.gross_cents, reyes.sss_cents, reyes.philhealth_cents),
                (1_250_000, 62_500, 31_250)
            );
            assert_eq!(
                (reyes.pagibig_cents, reyes.other_deductions_cents),
                (10_000, 200_000)
            );
            // Gross less every deduction is net, for each row.
            for x in &r.rows {
                let deductions = x.sss_cents
                    + x.philhealth_cents
                    + x.pagibig_cents
                    + x.tax_cents
                    + x.other_deductions_cents;
                assert_eq!(
                    x.gross_cents - deductions,
                    x.net_cents,
                    "{}",
                    x.employee_name
                );
            }

            let t = &r.totals;
            assert_eq!(
                (t.employee_no.as_str(), t.employee_name.as_str()),
                ("", "Total")
            );
            assert_eq!(
                (
                    t.gross_cents,
                    t.sss_cents,
                    t.philhealth_cents,
                    t.pagibig_cents
                ),
                (2_431_752, 125_000, 62_500, 20_000)
            );
            assert_eq!(t.other_deductions_cents, 200_000);
            assert_eq!(t.tax_cents, 5_445 + reyes.tax_cents);
            assert_eq!(t.net_cents, 1_072_557 + reyes.net_cents);
        }

        #[tokio::test]
        async fn writes_the_csv_with_a_totals_row_and_audits_it() {
            let (_d, db, period) = computed().await;
            let (name, bytes) = register_csv(&db, hr(), period, now()).await.expect("csv");
            assert_eq!(name, "payroll-register-2026-10-16.csv");
            let text = std::str::from_utf8(&bytes[3..]).unwrap();
            let lines: Vec<_> = text.lines().collect();
            assert_eq!(
                lines[0],
                "Employee no.,Employee,Gross pay,SSS,PhilHealth,Pag-IBIG,Withholding tax,\
                 Other deductions,Net pay"
            );
            assert_eq!(lines.len(), 4);
            assert_eq!(
                lines[2],
                "EMP-0001,\"Santos, Ana\",11817.52,625.00,312.50,100.00,54.45,0.00,10725.57"
            );
            assert!(
                lines[3].starts_with(",Total,24317.52,1250.00,625.00,200.00,"),
                "{}",
                lines[3]
            );

            let rows = audit_rows::all(&db).await;
            let last = rows.last().unwrap();
            assert_eq!(last.action, "report.export");
            let after = last.after_json.as_deref().unwrap();
            assert!(after.contains("\"report\":\"payroll_register\""), "{after}");
            assert!(after.contains("\"rows\":2"), "{after}");
        }

        #[tokio::test]
        async fn a_period_with_no_payslips_has_nothing_to_export() {
            let (_d, db, period) = db().await;
            assert!(matches!(
                register_report(&db, period).await,
                Err(AppError::Conflict(_))
            ));
            assert!(matches!(
                register_csv(&db, hr(), period, now()).await,
                Err(AppError::Conflict(_))
            ));
            assert!(audit_rows::actions(&db)
                .await
                .iter()
                .all(|a| a != "report.export"));
        }

        #[tokio::test]
        async fn an_unknown_period_is_not_found() {
            let (_d, db, _) = db().await;
            assert!(matches!(
                register_report(&db, 999).await,
                Err(AppError::NotFound("Pay period"))
            ));
        }
    }
}
