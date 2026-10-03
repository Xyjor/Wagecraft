//! Employee records (plan §6.2, §6.11). The Rust checks here are the ones that count;
//! the form repeats them only for quick feedback.

use crate::audit::{self, Actor, Entry};
use crate::domain::employee::{
    Employee, EmployeeInput, EmployeePage, EmployeeQuery, CIVIL_STATUSES, EMPLOYMENT_STATUSES,
    SEXES,
};
use crate::domain::validation as rules;
use crate::error::{AppError, FieldError};
use crate::repositories::employees::{self as repo, EmployeeFields, Unique};
use crate::repositories::org;
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Months, NaiveDate, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

/// Plan §6.11: an employee is at least this old on the hire date.
const MIN_AGE_YEARS: u32 = 18;
const DEFAULT_PAGE_SIZE: i64 = 25;
const MAX_PAGE_SIZE: i64 = 100;
const SEPARATED: [&str; 2] = ["RESIGNED", "TERMINATED"];

pub async fn get(db: &SqlitePool, id: i64) -> Result<Employee, AppError> {
    repo::get(db, id)
        .await?
        .ok_or(AppError::NotFound("Employee"))
}

pub async fn list(db: &SqlitePool, query: EmployeeQuery) -> Result<EmployeePage, AppError> {
    let page_size = match query.page_size {
        n if n <= 0 => DEFAULT_PAGE_SIZE,
        n => n.min(MAX_PAGE_SIZE),
    };
    let page = query.page.max(1);
    let mut conn = db.acquire().await?;
    let (items, total) = repo::list(&mut conn, &query, page_size, (page - 1) * page_size).await?;
    Ok(EmployeePage {
        items,
        total,
        page,
        page_size,
    })
}

pub async fn create(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: EmployeeInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Employee, AppError> {
    let mut tx = db.begin().await?;
    let fields = validate(&mut tx, &input, None, today).await?;
    let id = repo::insert(&mut tx, &fields).await?;
    let created = repo::get(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    let entry = Entry {
        action: "employee.create",
        entity: Some(("employee", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: EmployeeInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Employee, AppError> {
    let mut tx = db.begin().await?;
    let before = repo::get(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if before.archived_at.is_some() {
        return Err(AppError::Conflict(
            "Unarchive this employee before editing their record",
        ));
    }
    let fields = validate(&mut tx, &input, Some(&before), today).await?;
    repo::update(&mut tx, id, &fields, &time::to_db(now)).await?;
    let after = repo::get(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    let entry = Entry {
        action: "employee.update",
        entity: Some(("employee", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

/// Archives a separated employee, or brings one back. Archived people leave the default
/// list but stay in reports and payroll history (plan §6.2).
pub async fn set_archived(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    archived: bool,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let e = repo::get(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if e.archived_at.is_some() == archived {
        return Ok(());
    }
    if archived && !SEPARATED.contains(&e.employment_status.as_str()) {
        return Err(AppError::Conflict(
            "Only resigned or terminated employees can be archived",
        ));
    }
    let at = time::to_db(now);
    repo::set_archived_at(&mut tx, id, archived.then_some(at.as_str()), &at).await?;
    let entry = Entry {
        action: if archived {
            "employee.archive"
        } else {
            "employee.unarchive"
        },
        entity: Some(("employee", id)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Trimmed text, with blanks turned into `None` (plan §6.11: blanks are stored as NULL).
fn opt(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Collects every field problem in form order, then either the cleaned fields or the errors.
async fn validate(
    conn: &mut SqliteConnection,
    input: &EmployeeInput,
    before: Option<&Employee>,
    today: NaiveDate,
) -> Result<EmployeeFields, AppError> {
    let except = before.map(|b| b.id);
    let mut errors: Vec<FieldError> = Vec::new();
    // Identity
    let mut f = EmployeeFields {
        employee_no: input.employee_no.trim().to_uppercase(),
        ..Default::default()
    };
    let no_ok = (1..=20).contains(&f.employee_no.len())
        && f.employee_no
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !no_ok {
        errors.push(field(
            "employeeNo",
            "Use up to 20 letters, numbers or dashes",
        ));
    } else if repo::taken(conn, Unique::EmployeeNo, &f.employee_no, except).await? {
        errors.push(field("employeeNo", "Another employee has this number"));
    }
    f.first_name = input.first_name.trim().to_string();
    if !rules::name(&f.first_name) {
        errors.push(field("firstName", NAME_HINT));
    }
    f.middle_name = opt(&input.middle_name);
    if f.middle_name.as_deref().is_some_and(|n| !rules::name(n)) {
        errors.push(field("middleName", NAME_HINT));
    }
    f.last_name = input.last_name.trim().to_string();
    if !rules::name(&f.last_name) {
        errors.push(field("lastName", NAME_HINT));
    }
    f.suffix = opt(&input.suffix);
    if f.suffix
        .as_deref()
        .is_some_and(|s| s.chars().count() > 10 || !rules::name(s))
    {
        errors.push(field("suffix", "Use a short suffix like Jr., Sr. or III"));
    }

    // Personal
    let birth = opt(&input.birth_date).map(|s| (date(&s), s));
    if let Some((None, _)) = birth {
        errors.push(field("birthDate", DATE_HINT));
    }
    let birth = birth.and_then(|(d, _)| d);
    f.birth_date = birth.map(fmt);
    f.sex = opt(&input.sex);
    if f.sex.as_deref().is_some_and(|s| !SEXES.contains(&s)) {
        errors.push(field("sex", "Pick Male or Female"));
    }
    f.civil_status = opt(&input.civil_status);
    if f.civil_status
        .as_deref()
        .is_some_and(|s| !CIVIL_STATUSES.contains(&s))
    {
        errors.push(field("civilStatus", "Pick a civil status from the list"));
    }
    f.email = opt(&input.email);
    if let Some(email) = &f.email {
        if !rules::email(email) {
            errors.push(field("email", "Enter an email like juan@company.ph"));
        } else if repo::taken(conn, Unique::Email, email, except).await? {
            errors.push(field("email", "Another employee has this email"));
        }
    }
    match opt(&input.mobile) {
        None => f.mobile = None,
        Some(m) => match rules::mobile(&m) {
            Some(clean) => f.mobile = Some(clean),
            None => errors.push(field("mobile", "Use 09XXXXXXXXX or +639XXXXXXXXX")),
        },
    }
    f.address = opt(&input.address);
    if f.address
        .as_deref()
        .is_some_and(|a| a.chars().count() > 300)
    {
        errors.push(field("address", "Keep the address under 300 characters"));
    }

    // Employment
    let hire = date(&input.hire_date);
    match hire {
        None => errors.push(field("hireDate", DATE_HINT)),
        Some(h) if h > today => {
            errors.push(field("hireDate", "The hire date can't be in the future"))
        }
        Some(h) => {
            f.hire_date = fmt(h);
            let adult_on =
                birth.and_then(|b| b.checked_add_months(Months::new(MIN_AGE_YEARS * 12)));
            if adult_on.is_some_and(|a| a > h) {
                errors.push(field(
                    "birthDate",
                    "The employee must be at least 18 on the hire date",
                ));
            }
        }
    }
    for (name, value, target) in [
        (
            "regularizationDate",
            &input.regularization_date,
            &mut f.regularization_date,
        ),
        (
            "separationDate",
            &input.separation_date,
            &mut f.separation_date,
        ),
    ] {
        let Some(text) = opt(value) else { continue };
        match date(&text) {
            None => errors.push(field(name, DATE_HINT)),
            Some(d) if hire.is_some_and(|h| d < h) => {
                errors.push(field(name, "This date can't be before the hire date"))
            }
            Some(d) => *target = Some(fmt(d)),
        }
    }
    f.employment_status = input.employment_status.trim().to_string();
    if !EMPLOYMENT_STATUSES.contains(&f.employment_status.as_str()) {
        errors.push(field("employmentStatus", "Pick a status from the list"));
    } else {
        let separated = SEPARATED.contains(&f.employment_status.as_str());
        let has_date = opt(&input.separation_date).is_some();
        if separated && !has_date {
            errors.push(field("separationDate", "Enter the last day of work"));
        } else if !separated && has_date {
            errors.push(field(
                "employmentStatus",
                "Set the status to Resigned or Terminated, or clear the separation date",
            ));
        }
    }

    // Organization: anything newly assigned must be active; what was already there may stay.
    f.department_id = input.department_id;
    if let Some(id) = f.department_id {
        let unchanged = before.is_some_and(|b| b.department_id == Some(id));
        let department = org::department(&mut *conn, id).await?;
        if !department.is_some_and(|d| d.is_active || unchanged) {
            errors.push(field("departmentId", "Pick an active department"));
        }
    }
    f.position_id = input.position_id;
    if let Some(id) = f.position_id {
        let unchanged = before.is_some_and(|b| b.position_id == Some(id));
        match org::position(&mut *conn, id).await? {
            Some(p) if !(p.is_active || unchanged) => {
                errors.push(field("positionId", "Pick an active position"))
            }
            Some(p) if Some(p.department_id) != f.department_id => errors.push(field(
                "positionId",
                "This position belongs to another department",
            )),
            Some(_) => {}
            None => errors.push(field("positionId", "Pick an active position")),
        }
    }
    f.schedule_id = input.schedule_id;
    if let Some(id) = f.schedule_id {
        let unchanged = before.is_some_and(|b| b.schedule_id == Some(id));
        if !repo::schedule_active(conn, id)
            .await?
            .is_some_and(|active| active || unchanged)
        {
            errors.push(field("scheduleId", "Pick an active work schedule"));
        }
    }

    // Government IDs: digits only, unique.
    type Normalize = fn(&str) -> Option<String>;
    let ids: [(&str, &Option<String>, Normalize, Unique, &str); 4] = [
        (
            "tin",
            &input.tin,
            rules::tin,
            Unique::Tin,
            "Use 000-000-000, with an optional branch code",
        ),
        (
            "sssNo",
            &input.sss_no,
            rules::sss,
            Unique::Sss,
            "Use 00-0000000-0 (10 digits)",
        ),
        (
            "philhealthNo",
            &input.philhealth_no,
            rules::philhealth,
            Unique::Philhealth,
            "Use 00-000000000-0 (12 digits)",
        ),
        (
            "pagibigNo",
            &input.pagibig_no,
            rules::pagibig,
            Unique::Pagibig,
            "Use 0000-0000-0000 (12 digits)",
        ),
    ];
    let mut clean_ids: [Option<String>; 4] = Default::default();
    for (i, (name, value, normalize, unique, hint)) in ids.into_iter().enumerate() {
        let Some(text) = opt(value) else { continue };
        match normalize(&text) {
            None => errors.push(field(name, hint)),
            Some(digits) if repo::taken(conn, unique, &digits, except).await? => {
                errors.push(field(name, "Another employee has this number"))
            }
            Some(digits) => clean_ids[i] = Some(digits),
        }
    }
    [f.tin, f.sss_no, f.philhealth_no, f.pagibig_no] = clean_ids;

    // Bank
    f.bank_name = opt(&input.bank_name);
    if f.bank_name
        .as_deref()
        .is_some_and(|b| b.chars().count() > 100)
    {
        errors.push(field("bankName", "Keep the bank name under 100 characters"));
    }
    f.bank_account_no = opt(&input.bank_account_no);
    let account_ok = |a: &str| {
        a.len() <= 30
            && a.chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    };
    if f.bank_account_no.as_deref().is_some_and(|a| !account_ok(a)) {
        errors.push(field(
            "bankAccountNo",
            "Use digits, spaces or dashes (up to 30)",
        ));
    }

    if errors.is_empty() {
        Ok(f)
    } else {
        Err(AppError::Validation(errors))
    }
}

const NAME_HINT: &str = "Use letters, spaces, hyphens, apostrophes or periods (up to 60)";
const DATE_HINT: &str = "Enter a date as YYYY-MM-DD";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::employee::{Archived, EmployeeSort};
    use crate::domain::org::{DepartmentInput, PositionInput};
    use crate::services::org;
    use chrono::TimeZone;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()
    }

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

    fn juan() -> EmployeeInput {
        EmployeeInput {
            employee_no: "emp-0001".into(),
            first_name: "Juan".into(),
            middle_name: Some("".into()),
            last_name: "Dela Cruz".into(),
            birth_date: Some("1995-03-14".into()),
            sex: Some("MALE".into()),
            email: Some("juan@rojyx.ph".into()),
            mobile: Some("+639171234567".into()),
            hire_date: "2024-06-03".into(),
            employment_status: "REGULAR".into(),
            tin: Some("123-456-789".into()),
            sss_no: Some("34-1234567-8".into()),
            philhealth_no: Some("12-345678901-2".into()),
            pagibig_no: Some("1234-5678-9012".into()),
            ..Default::default()
        }
    }

    /// A second, unrelated employee with no clashing IDs.
    fn maria() -> EmployeeInput {
        EmployeeInput {
            employee_no: "EMP-0002".into(),
            first_name: "Maria".into(),
            last_name: "Santos".into(),
            hire_date: "2025-01-06".into(),
            employment_status: "PROBATIONARY".into(),
            ..Default::default()
        }
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    async fn department(db: &SqlitePool, code: &str) -> i64 {
        let input = DepartmentInput {
            code: code.into(),
            name: format!("{code} Department"),
            description: None,
        };
        org::create_department(db, hr(), input, now())
            .await
            .expect("dept")
            .id
    }

    async fn position(db: &SqlitePool, department_id: i64, title: &str) -> i64 {
        let input = PositionInput {
            department_id,
            title: title.into(),
            min_rate_cents: None,
            max_rate_cents: None,
        };
        org::create_position(db, hr(), input, now())
            .await
            .expect("position")
            .id
    }

    fn query() -> EmployeeQuery {
        EmployeeQuery {
            page: 1,
            page_size: 25,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn create_stores_tidy_values() {
        let (_dir, db) = db().await;
        let e = create(&db, hr(), juan(), today(), now())
            .await
            .expect("create");
        assert_eq!(e.employee_no, "EMP-0001");
        assert_eq!(e.middle_name, None, "blank optional fields are NULL");
        assert_eq!(e.mobile.as_deref(), Some("09171234567"));
        assert_eq!(e.tin.as_deref(), Some("123456789"));
        assert_eq!(e.sss_no.as_deref(), Some("3412345678"));
        assert_eq!(e.philhealth_no.as_deref(), Some("123456789012"));
        assert_eq!(e.pagibig_no.as_deref(), Some("123456789012"));
        assert!(!e.has_kiosk_pin);
        assert_eq!(get(&db, e.id).await.expect("get").last_name, "Dela Cruz");
    }

    #[tokio::test]
    async fn required_fields_and_formats_are_checked() {
        let (_dir, db) = db().await;
        let bad = EmployeeInput {
            employee_no: " ".into(),
            first_name: "J0hn".into(),
            last_name: "".into(),
            email: Some("not-an-email".into()),
            mobile: Some("12345".into()),
            hire_date: "2024-13-01".into(),
            employment_status: "BOSS".into(),
            tin: Some("123".into()),
            sss_no: Some("1".into()),
            philhealth_no: Some("1".into()),
            pagibig_no: Some("1".into()),
            sex: Some("X".into()),
            ..Default::default()
        };
        let err = create(&db, hr(), bad, today(), now()).await.unwrap_err();
        assert_eq!(
            fields(err),
            [
                "employeeNo",
                "firstName",
                "lastName",
                "sex",
                "email",
                "mobile",
                "hireDate",
                "employmentStatus",
                "tin",
                "sssNo",
                "philhealthNo",
                "pagibigNo",
            ]
        );
    }

    #[tokio::test]
    async fn numbers_ids_and_email_must_be_unique() {
        let (_dir, db) = db().await;
        create(&db, hr(), juan(), today(), now())
            .await
            .expect("first");
        let twin = EmployeeInput {
            first_name: "Pedro".into(),
            email: Some("JUAN@rojyx.ph".into()),
            ..juan()
        };
        let err = create(&db, hr(), twin, today(), now()).await.unwrap_err();
        assert_eq!(
            fields(err),
            [
                "employeeNo",
                "email",
                "tin",
                "sssNo",
                "philhealthNo",
                "pagibigNo"
            ]
        );
    }

    #[tokio::test]
    async fn dates_follow_the_age_and_calendar_rules() {
        let (_dir, db) = db().await;
        let too_young = EmployeeInput {
            birth_date: Some("2008-01-01".into()),
            ..juan()
        };
        let err = create(&db, hr(), too_young, today(), now())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["birthDate"]);

        let future = EmployeeInput {
            hire_date: "2026-10-08".into(),
            ..juan()
        };
        let err = create(&db, hr(), future, today(), now()).await.unwrap_err();
        assert_eq!(fields(err), ["hireDate"]);

        let early = EmployeeInput {
            regularization_date: Some("2024-01-01".into()),
            ..juan()
        };
        let err = create(&db, hr(), early, today(), now()).await.unwrap_err();
        assert_eq!(fields(err), ["regularizationDate"]);
    }

    #[tokio::test]
    async fn separation_date_and_status_go_together() {
        let (_dir, db) = db().await;
        let resigned_no_date = EmployeeInput {
            employment_status: "RESIGNED".into(),
            ..juan()
        };
        let err = create(&db, hr(), resigned_no_date, today(), now())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["separationDate"]);

        let date_but_regular = EmployeeInput {
            separation_date: Some("2026-09-30".into()),
            ..juan()
        };
        let err = create(&db, hr(), date_but_regular, today(), now())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["employmentStatus"]);
    }

    #[tokio::test]
    async fn position_must_belong_to_the_department() {
        let (_dir, db) = db().await;
        let ops = department(&db, "OPS").await;
        let fin = department(&db, "FIN").await;
        let driver = position(&db, ops, "Driver").await;

        let mismatch = EmployeeInput {
            department_id: Some(fin),
            position_id: Some(driver),
            ..juan()
        };
        let err = create(&db, hr(), mismatch, today(), now())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["positionId"]);

        let ok = EmployeeInput {
            department_id: Some(ops),
            position_id: Some(driver),
            ..juan()
        };
        let e = create(&db, hr(), ok, today(), now()).await.expect("create");
        assert_eq!(e.department_name.as_deref(), Some("OPS Department"));
        assert_eq!(e.position_title.as_deref(), Some("Driver"));
    }

    #[tokio::test]
    async fn a_deactivated_department_stays_on_people_already_in_it() {
        let (_dir, db) = db().await;
        let ops = department(&db, "OPS").await;
        let e = create(
            &db,
            hr(),
            EmployeeInput {
                department_id: Some(ops),
                ..juan()
            },
            today(),
            now(),
        )
        .await
        .expect("create");
        org::set_department_active(&db, hr(), ops, false, now())
            .await
            .expect("deactivate");

        // Editing something else keeps the old department.
        let edit = EmployeeInput {
            department_id: Some(ops),
            mobile: Some("09181234567".into()),
            ..juan()
        };
        update(&db, hr(), e.id, edit, today(), now())
            .await
            .expect("unrelated edit");

        // A new hire can't be put into it.
        let new_hire = EmployeeInput {
            department_id: Some(ops),
            ..maria()
        };
        let err = create(&db, hr(), new_hire, today(), now())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["departmentId"]);
    }

    #[tokio::test]
    async fn the_list_searches_filters_sorts_and_pages() {
        let (_dir, db) = db().await;
        let ops = department(&db, "OPS").await;
        create(&db, hr(), juan(), today(), now())
            .await
            .expect("juan");
        let maria = EmployeeInput {
            department_id: Some(ops),
            email: Some("maria@rojyx.ph".into()),
            ..maria()
        };
        create(&db, hr(), maria, today(), now())
            .await
            .expect("maria");
        for i in 3..=12 {
            let extra = EmployeeInput {
                employee_no: format!("EMP-{i:04}"),
                first_name: "Extra".into(),
                last_name: format!("Person {}", char::from(b'A' + i as u8)),
                hire_date: "2025-02-03".into(),
                employment_status: "REGULAR".into(),
                ..Default::default()
            };
            create(&db, hr(), extra, today(), now())
                .await
                .expect("extra");
        }

        let names = |page: &EmployeePage| -> Vec<String> {
            page.items.iter().map(|i| i.last_name.clone()).collect()
        };

        let by_name = list(
            &db,
            EmployeeQuery {
                search: Some("santos".into()),
                ..query()
            },
        )
        .await
        .expect("search");
        assert_eq!(names(&by_name), ["Santos"]);
        let by_full_name = list(
            &db,
            EmployeeQuery {
                search: Some("Juan Dela".into()),
                ..query()
            },
        )
        .await
        .expect("search");
        assert_eq!(names(&by_full_name), ["Dela Cruz"]);
        let by_email = list(
            &db,
            EmployeeQuery {
                search: Some("maria@".into()),
                ..query()
            },
        )
        .await
        .expect("search");
        assert_eq!(by_email.total, 1);
        let wildcard = list(
            &db,
            EmployeeQuery {
                search: Some("%".into()),
                ..query()
            },
        )
        .await
        .expect("search");
        assert_eq!(wildcard.total, 0, "% is a plain character, not a wildcard");

        let in_ops = list(
            &db,
            EmployeeQuery {
                department_id: Some(ops),
                ..query()
            },
        )
        .await
        .expect("filter");
        assert_eq!(names(&in_ops), ["Santos"]);
        let probationary = list(
            &db,
            EmployeeQuery {
                employment_status: Some("PROBATIONARY".into()),
                ..query()
            },
        )
        .await
        .expect("filter");
        assert_eq!(probationary.total, 1);

        let page2 = list(
            &db,
            EmployeeQuery {
                page: 2,
                page_size: 5,
                sort: EmployeeSort::EmployeeNo,
                descending: true,
                ..query()
            },
        )
        .await
        .expect("page");
        assert_eq!(page2.total, 12);
        let numbers: Vec<_> = page2.items.iter().map(|i| i.employee_no.as_str()).collect();
        assert_eq!(
            numbers,
            ["EMP-0007", "EMP-0006", "EMP-0005", "EMP-0004", "EMP-0003"]
        );
    }

    #[tokio::test]
    async fn archiving_needs_a_separation_and_hides_the_employee() {
        let (_dir, db) = db().await;
        let e = create(&db, hr(), juan(), today(), now())
            .await
            .expect("create");
        let err = set_archived(&db, hr(), e.id, true, now())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)), "{err:?}");

        let resigned = EmployeeInput {
            employment_status: "RESIGNED".into(),
            separation_date: Some("2026-09-30".into()),
            ..juan()
        };
        update(&db, hr(), e.id, resigned, today(), now())
            .await
            .expect("resign");
        set_archived(&db, hr(), e.id, true, now())
            .await
            .expect("archive");

        assert_eq!(list(&db, query()).await.expect("list").total, 0);
        let archived = list(
            &db,
            EmployeeQuery {
                archived: Archived::Only,
                ..query()
            },
        )
        .await
        .expect("list");
        assert_eq!(archived.total, 1);
        assert!(archived.items[0].archived);

        let edit = update(&db, hr(), e.id, juan(), today(), now())
            .await
            .unwrap_err();
        assert!(
            matches!(edit, AppError::Conflict(_)),
            "archived records are read-only"
        );

        set_archived(&db, hr(), e.id, false, now())
            .await
            .expect("unarchive");
        assert_eq!(list(&db, query()).await.expect("list").total, 1);
    }

    #[tokio::test]
    async fn changes_are_audited_with_before_and_after() {
        let (_dir, db) = db().await;
        let e = create(&db, hr(), juan(), today(), now())
            .await
            .expect("create");
        let renamed = EmployeeInput {
            last_name: "Dela Cruz-Reyes".into(),
            ..juan()
        };
        update(&db, hr(), e.id, renamed, today(), now())
            .await
            .expect("update");

        let rows = audit_rows::all(&db).await;
        let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(actions, ["employee.create", "employee.update"]);
        assert!(rows[1]
            .before_json
            .as_deref()
            .unwrap()
            .contains("\"Dela Cruz\""));
        assert!(rows[1]
            .after_json
            .as_deref()
            .unwrap()
            .contains("\"Dela Cruz-Reyes\""));
        assert!(!rows[1].after_json.as_deref().unwrap().contains("kiosk"));
    }

    #[tokio::test]
    async fn unknown_ids_are_not_found() {
        let (_dir, db) = db().await;
        assert!(matches!(
            get(&db, 9).await,
            Err(AppError::NotFound("Employee"))
        ));
        assert!(matches!(
            update(&db, hr(), 9, juan(), today(), now()).await,
            Err(AppError::NotFound("Employee"))
        ));
    }
}
