//! SQL for computing and reading payslips: the facts compute gathers for each employee,
//! and the payslips it saves. No rules here: the payroll service decides.

use crate::domain::payroll_period::{PayslipLine, SkippedEmployee};
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const NAME: &str = "e.employee_no, e.last_name || ', ' || e.first_name AS employee_name";

/// Someone employed at some point from `from` to `to`, with the pay rate in effect on
/// `from` if there is one.
#[derive(Debug, FromRow)]
pub struct ScopeEmployee {
    pub id: i64,
    pub hire_date: String,
    pub separation_date: Option<String>,
    pub pay_basis: Option<String>,
    pub rate_cents: Option<i64>,
}

/// Archived employees are left out unless they have a separation date, since archiving
/// someone with no separation date means the record was a mistake.
pub async fn in_scope<'e>(
    db: impl SqliteExecutor<'e>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<ScopeEmployee>> {
    sqlx::query_as(
        "SELECT e.id, e.hire_date, e.separation_date, c.pay_basis, c.rate_cents \
         FROM employees e \
         LEFT JOIN compensations c ON c.employee_id = e.id AND c.effective_from <= ?1 \
              AND (c.effective_to IS NULL OR c.effective_to >= ?1) \
         WHERE e.hire_date <= ?2 AND (e.separation_date IS NULL OR e.separation_date >= ?1) \
           AND (e.archived_at IS NULL OR e.separation_date IS NOT NULL) \
         ORDER BY e.last_name, e.first_name, e.employee_no",
    )
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

/// (date, type) for every holiday from `from` to `to`.
pub async fn holidays<'e>(
    db: impl SqliteExecutor<'e>,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<(String, String)>> {
    sqlx::query_as("SELECT date, type FROM holidays WHERE date BETWEEN ? AND ?")
        .bind(from)
        .bind(to)
        .fetch_all(db)
        .await
}

#[derive(Debug, FromRow)]
pub struct DayRecordRow {
    pub work_date: String,
    pub time_in: Option<String>,
    pub time_out: Option<String>,
    pub late_minutes: i64,
    pub undertime_minutes: i64,
    pub worked_minutes: i64,
    pub night_minutes: i64,
}

pub async fn attendance<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<DayRecordRow>> {
    sqlx::query_as(
        "SELECT work_date, time_in, time_out, late_minutes, undertime_minutes, \
         worked_minutes, night_minutes FROM attendance_records \
         WHERE employee_id = ? AND work_date BETWEEN ? AND ?",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

#[derive(Debug, FromRow)]
pub struct LeaveRow {
    pub start_date: String,
    pub end_date: String,
    pub half_day: bool,
    pub is_paid: bool,
}

/// Approved leave touching the days `from` to `to`.
pub async fn approved_leave<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<LeaveRow>> {
    sqlx::query_as(
        "SELECT r.start_date, r.end_date, r.half_day, t.is_paid FROM leave_requests r \
         JOIN leave_types t ON t.id = r.leave_type_id \
         WHERE r.employee_id = ? AND r.status = 'APPROVED' \
           AND r.start_date <= ? AND r.end_date >= ?",
    )
    .bind(employee_id)
    .bind(to)
    .bind(from)
    .fetch_all(db)
    .await
}

#[derive(Debug, FromRow)]
pub struct OvertimeRow {
    pub work_date: String,
    pub start_at: String,
    pub end_at: String,
    pub minutes: i64,
}

/// Approved overtime on the days `from` to `to`, in order.
pub async fn approved_overtime<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<OvertimeRow>> {
    sqlx::query_as(
        "SELECT work_date, start_at, end_at, minutes FROM overtime_requests \
         WHERE employee_id = ? AND status = 'APPROVED' AND work_date BETWEEN ? AND ? \
         ORDER BY start_at",
    )
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}

pub async fn delete_for_period(conn: &mut SqliteConnection, period_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM payslips WHERE payroll_period_id = ?")
        .bind(period_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub struct NewPayslip<'a> {
    pub period_id: i64,
    pub employee_id: i64,
    pub pay_basis: &'a str,
    pub rate_cents: i64,
    pub gross_cents: i64,
    pub taxable_cents: i64,
    pub statutory_ee_cents: i64,
    pub tax_cents: i64,
    pub other_deductions_cents: i64,
    pub net_cents: i64,
    pub warnings_json: &'a str,
    pub inputs_json: &'a str,
}

pub async fn insert(conn: &mut SqliteConnection, p: &NewPayslip<'_>) -> sqlx::Result<i64> {
    let id = sqlx::query(
        "INSERT INTO payslips (payroll_period_id, employee_id, pay_basis, rate_cents, \
         gross_cents, taxable_cents, statutory_ee_cents, tax_cents, other_deductions_cents, \
         net_cents, warnings_json, inputs_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(p.period_id)
    .bind(p.employee_id)
    .bind(p.pay_basis)
    .bind(p.rate_cents)
    .bind(p.gross_cents)
    .bind(p.taxable_cents)
    .bind(p.statutory_ee_cents)
    .bind(p.tax_cents)
    .bind(p.other_deductions_cents)
    .bind(p.net_cents)
    .bind(p.warnings_json)
    .bind(p.inputs_json)
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn insert_line(
    conn: &mut SqliteConnection,
    payslip_id: i64,
    sort_order: i64,
    l: &PayslipLine,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO payslip_lines (payslip_id, kind, code, label, quantity, unit, \
         amount_cents, taxable, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(payslip_id)
    .bind(&l.kind)
    .bind(&l.code)
    .bind(&l.label)
    .bind(&l.quantity)
    .bind(&l.unit)
    .bind(l.amount_cents)
    .bind(l.taxable)
    .bind(sort_order)
    .execute(conn)
    .await?;
    Ok(())
}

/// A saved payslip with its employee, as stored.
#[derive(Debug, FromRow)]
pub struct PayslipRow {
    pub id: i64,
    pub payroll_period_id: i64,
    pub employee_no: String,
    pub employee_name: String,
    pub pay_basis: String,
    pub rate_cents: i64,
    pub gross_cents: i64,
    pub taxable_cents: i64,
    pub statutory_ee_cents: i64,
    pub tax_cents: i64,
    pub other_deductions_cents: i64,
    pub net_cents: i64,
    pub warnings_json: String,
}

const PAYSLIP: &str = "SELECT p.id, p.payroll_period_id, e.employee_no, \
    e.last_name || ', ' || e.first_name AS employee_name, p.pay_basis, p.rate_cents, \
    p.gross_cents, p.taxable_cents, p.statutory_ee_cents, p.tax_cents, \
    p.other_deductions_cents, p.net_cents, p.warnings_json \
    FROM payslips p JOIN employees e ON e.id = p.employee_id";

/// The period's payslips, by employee name.
pub async fn for_period<'e>(
    db: impl SqliteExecutor<'e>,
    period_id: i64,
) -> sqlx::Result<Vec<PayslipRow>> {
    sqlx::query_as(&format!(
        "{PAYSLIP} WHERE p.payroll_period_id = ? \
         ORDER BY e.last_name, e.first_name, e.employee_no"
    ))
    .bind(period_id)
    .fetch_all(db)
    .await
}

pub async fn by_id<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<PayslipRow>> {
    sqlx::query_as(&format!("{PAYSLIP} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn lines<'e>(
    db: impl SqliteExecutor<'e>,
    payslip_id: i64,
) -> sqlx::Result<Vec<PayslipLine>> {
    sqlx::query_as(
        "SELECT kind, code, label, quantity, unit, amount_cents, taxable FROM payslip_lines \
         WHERE payslip_id = ? ORDER BY sort_order",
    )
    .bind(payslip_id)
    .fetch_all(db)
    .await
}

/// Employees in the period with no payslip, and why.
pub async fn skipped<'e>(
    db: impl SqliteExecutor<'e>,
    period_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<SkippedEmployee>> {
    sqlx::query_as(&format!(
        "SELECT {NAME}, CASE WHEN NOT EXISTS (SELECT 1 FROM compensations c \
              WHERE c.employee_id = e.id AND c.effective_from <= ?1 \
              AND (c.effective_to IS NULL OR c.effective_to >= ?1)) \
           THEN 'No pay rate in effect for this period' \
           ELSE 'Not computed yet. Recompute to include them' END AS reason \
         FROM employees e \
         WHERE e.hire_date <= ?2 AND (e.separation_date IS NULL OR e.separation_date >= ?1) \
           AND (e.archived_at IS NULL OR e.separation_date IS NOT NULL) \
           AND NOT EXISTS (SELECT 1 FROM payslips p \
               WHERE p.payroll_period_id = ?3 AND p.employee_id = e.id) \
         ORDER BY e.last_name, e.first_name, e.employee_no"
    ))
    .bind(from)
    .bind(to)
    .bind(period_id)
    .fetch_all(db)
    .await
}
