//! Report files (plan §6.8). For now, the employee masterlist; the payroll register joins
//! it in Phase 6.

use crate::audit::{self, Actor, Entry};
use crate::domain::employee::EmployeeQuery;
use crate::domain::validation::{format_id, IdKind};
use crate::error::AppError;
use crate::export::csv;
use crate::repositories::employees::{self, MasterlistRow};
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
    format!("{}.{:02}", cents / 100, cents % 100)
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
}
