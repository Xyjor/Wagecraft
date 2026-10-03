//! SQL for employees. No rules here: the employee service validates first.

use crate::domain::employee::{Archived, Employee, EmployeeListItem, EmployeeQuery, EmployeeSort};
use sqlx::{SqliteConnection, SqliteExecutor};

const PROFILE: &str = "SELECT e.id, e.employee_no, e.first_name, e.middle_name, e.last_name, \
    e.suffix, e.birth_date, e.sex, e.civil_status, e.email, e.mobile, e.address, e.hire_date, \
    e.regularization_date, e.separation_date, e.employment_status, e.department_id, \
    d.name AS department_name, e.position_id, p.title AS position_title, e.schedule_id, e.tin, \
    e.sss_no, e.philhealth_no, e.pagibig_no, e.bank_name, e.bank_account_no, \
    e.kiosk_pin_hash IS NOT NULL AS has_kiosk_pin, e.archived_at \
    FROM employees e \
    LEFT JOIN departments d ON d.id = e.department_id \
    LEFT JOIN positions p ON p.id = e.position_id";

pub async fn get<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<Employee>> {
    sqlx::query_as(&format!("{PROFILE} WHERE e.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// The list filters as SQL, using numbered parameters ?1 to ?5 (see `bind_filters`).
const FILTERS: &str = "FROM employees e \
    LEFT JOIN departments d ON d.id = e.department_id \
    LEFT JOIN positions p ON p.id = e.position_id \
    WHERE (?1 IS NULL \
        OR e.employee_no LIKE ?1 ESCAPE '\\' \
        OR e.first_name LIKE ?1 ESCAPE '\\' \
        OR e.last_name LIKE ?1 ESCAPE '\\' \
        OR (e.first_name || ' ' || e.last_name) LIKE ?1 ESCAPE '\\' \
        OR e.email LIKE ?1 ESCAPE '\\') \
    AND (?2 IS NULL OR e.department_id = ?2) \
    AND (?3 IS NULL OR e.position_id = ?3) \
    AND (?4 IS NULL OR e.employment_status = ?4) \
    AND (CASE ?5 WHEN 'exclude' THEN e.archived_at IS NULL \
                 WHEN 'only' THEN e.archived_at IS NOT NULL \
                 ELSE 1 END)";

/// Sort orders come from this fixed list, never from text the UI sent (plan §4.6).
fn order_by(sort: EmployeeSort, descending: bool) -> String {
    let dir = if descending { "DESC" } else { "ASC" };
    match sort {
        EmployeeSort::Name => format!(
            "e.last_name COLLATE NOCASE {dir}, e.first_name COLLATE NOCASE {dir}, e.id {dir}"
        ),
        EmployeeSort::EmployeeNo => format!("e.employee_no COLLATE NOCASE {dir}"),
        EmployeeSort::Department => {
            format!("d.name IS NULL, d.name COLLATE NOCASE {dir}, e.last_name COLLATE NOCASE, e.id")
        }
        EmployeeSort::HireDate => format!("e.hire_date {dir}, e.last_name COLLATE NOCASE, e.id"),
    }
}

/// `%term%`, with LIKE's own wildcards escaped so they match literally.
fn like_pattern(search: Option<&str>) -> Option<String> {
    let term = search.map(str::trim).filter(|s| !s.is_empty())?;
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

fn archived_filter(a: Archived) -> &'static str {
    match a {
        Archived::Exclude => "exclude",
        Archived::Only => "only",
        Archived::Include => "include",
    }
}

/// Returns one page of the list, plus the total count across all pages.
pub async fn list(
    conn: &mut SqliteConnection,
    q: &EmployeeQuery,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<EmployeeListItem>, i64)> {
    let pattern = like_pattern(q.search.as_deref());
    let status = q.employment_status.as_deref().filter(|s| !s.is_empty());
    let archived = archived_filter(q.archived);

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) {FILTERS}"))
        .bind(&pattern)
        .bind(q.department_id)
        .bind(q.position_id)
        .bind(status)
        .bind(archived)
        .fetch_one(&mut *conn)
        .await?;

    let sql = format!(
        "SELECT e.id, e.employee_no, e.last_name, e.first_name, e.middle_name, e.suffix, \
         d.name AS department_name, p.title AS position_title, e.employment_status, \
         e.hire_date, e.archived_at IS NOT NULL AS archived \
         {FILTERS} ORDER BY {} LIMIT ?6 OFFSET ?7",
        order_by(q.sort, q.descending)
    );
    let items = sqlx::query_as(&sql)
        .bind(&pattern)
        .bind(q.department_id)
        .bind(q.position_id)
        .bind(status)
        .bind(archived)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;
    Ok((items, total))
}

/// Columns that must be unique across employees. The SQL for each is fixed here.
#[derive(Debug, Clone, Copy)]
pub enum Unique {
    EmployeeNo,
    Email,
    Tin,
    Sss,
    Philhealth,
    Pagibig,
}

/// Whether another employee already has this value.
pub async fn taken(
    conn: &mut SqliteConnection,
    column: Unique,
    value: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    let condition = match column {
        Unique::EmployeeNo => "employee_no = ?",
        Unique::Email => "lower(email) = lower(?)",
        Unique::Tin => "tin = ?",
        Unique::Sss => "sss_no = ?",
        Unique::Philhealth => "philhealth_no = ?",
        Unique::Pagibig => "pagibig_no = ?",
    };
    sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM employees WHERE {condition} AND id IS NOT ?)"
    ))
    .bind(value)
    .bind(except_id)
    .fetch_one(conn)
    .await
}

/// Cleaned values, ready to store. Built by the employee service.
#[derive(Debug, Default)]
pub struct EmployeeFields {
    pub employee_no: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub suffix: Option<String>,
    pub birth_date: Option<String>,
    pub sex: Option<String>,
    pub civil_status: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub address: Option<String>,
    pub hire_date: String,
    pub regularization_date: Option<String>,
    pub separation_date: Option<String>,
    pub employment_status: String,
    pub department_id: Option<i64>,
    pub position_id: Option<i64>,
    pub schedule_id: Option<i64>,
    pub tin: Option<String>,
    pub sss_no: Option<String>,
    pub philhealth_no: Option<String>,
    pub pagibig_no: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account_no: Option<String>,
}

const COLUMNS: [&str; 24] = [
    "employee_no",
    "first_name",
    "middle_name",
    "last_name",
    "suffix",
    "birth_date",
    "sex",
    "civil_status",
    "email",
    "mobile",
    "address",
    "hire_date",
    "regularization_date",
    "separation_date",
    "employment_status",
    "department_id",
    "position_id",
    "schedule_id",
    "tin",
    "sss_no",
    "philhealth_no",
    "pagibig_no",
    "bank_name",
    "bank_account_no",
];

/// Binds the fields in `COLUMNS` order.
fn bind_fields<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    f: &'q EmployeeFields,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    q.bind(&f.employee_no)
        .bind(&f.first_name)
        .bind(&f.middle_name)
        .bind(&f.last_name)
        .bind(&f.suffix)
        .bind(&f.birth_date)
        .bind(&f.sex)
        .bind(&f.civil_status)
        .bind(&f.email)
        .bind(&f.mobile)
        .bind(&f.address)
        .bind(&f.hire_date)
        .bind(&f.regularization_date)
        .bind(&f.separation_date)
        .bind(&f.employment_status)
        .bind(f.department_id)
        .bind(f.position_id)
        .bind(f.schedule_id)
        .bind(&f.tin)
        .bind(&f.sss_no)
        .bind(&f.philhealth_no)
        .bind(&f.pagibig_no)
        .bind(&f.bank_name)
        .bind(&f.bank_account_no)
}

pub async fn insert(conn: &mut SqliteConnection, f: &EmployeeFields) -> sqlx::Result<i64> {
    let sql = format!(
        "INSERT INTO employees ({}) VALUES ({})",
        COLUMNS.join(", "),
        vec!["?"; COLUMNS.len()].join(", ")
    );
    let res = bind_fields(sqlx::query(&sql), f).execute(conn).await?;
    Ok(res.last_insert_rowid())
}

pub async fn update(
    conn: &mut SqliteConnection,
    id: i64,
    f: &EmployeeFields,
    now: &str,
) -> sqlx::Result<()> {
    let sets: Vec<String> = COLUMNS.iter().map(|c| format!("{c} = ?")).collect();
    let sql = format!(
        "UPDATE employees SET {}, updated_at = ? WHERE id = ?",
        sets.join(", ")
    );
    bind_fields(sqlx::query(&sql), f)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_archived_at(
    conn: &mut SqliteConnection,
    id: i64,
    archived_at: Option<&str>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE employees SET archived_at = ?, updated_at = ? WHERE id = ?")
        .bind(archived_at)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// The schedule's id if it exists, with whether it is active.
pub async fn schedule_active(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<Option<bool>> {
    sqlx::query_scalar("SELECT is_active FROM work_schedules WHERE id = ?")
        .bind(id)
        .fetch_optional(conn)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_escapes_like_wildcards() {
        assert_eq!(
            like_pattern(Some(" 50%_off ")).as_deref(),
            Some("%50\\%\\_off%")
        );
        assert_eq!(like_pattern(Some("  ")), None);
        assert_eq!(like_pattern(None), None);
    }
}
