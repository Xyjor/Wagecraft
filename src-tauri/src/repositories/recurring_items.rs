//! SQL for recurring items and deduction carryovers. No rules here: the recurring items
//! and payslips services decide.

use crate::domain::recurring_item::{OpenCarryover, RecurringItem};
use sqlx::{SqliteConnection, SqliteExecutor};

const SELECT: &str = "SELECT id, employee_id, kind, label, amount_cents, taxable, schedule, \
    start_date, end_date, remaining_balance_cents FROM recurring_items";

/// Allowances first, then loans, then other deductions; oldest first within each.
pub async fn for_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
) -> sqlx::Result<Vec<RecurringItem>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE employee_id = ? \
         ORDER BY CASE kind WHEN 'ALLOWANCE' THEN 0 WHEN 'LOAN' THEN 1 ELSE 2 END, \
         start_date, id"
    ))
    .bind(employee_id)
    .fetch_all(db)
    .await
}

pub async fn get<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<RecurringItem>> {
    sqlx::query_as(&format!("{SELECT} WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Items in effect at any time from `from` to `to`, whatever their cutoff.
pub async fn in_effect<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<RecurringItem>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE employee_id = ? AND start_date <= ? \
         AND (end_date IS NULL OR end_date >= ?)"
    ))
    .bind(employee_id)
    .bind(to)
    .bind(from)
    .fetch_all(db)
    .await
}

pub struct ItemFields<'a> {
    pub kind: &'a str,
    pub label: &'a str,
    pub amount_cents: i64,
    pub taxable: bool,
    pub schedule: &'a str,
    pub start_date: &'a str,
    pub end_date: Option<&'a str>,
    pub remaining_balance_cents: Option<i64>,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    employee_id: i64,
    f: &ItemFields<'_>,
    created_by: Option<i64>,
    created_at: &str,
) -> sqlx::Result<i64> {
    let id = sqlx::query(
        "INSERT INTO recurring_items (employee_id, kind, label, amount_cents, taxable, \
         schedule, start_date, end_date, remaining_balance_cents, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(employee_id)
    .bind(f.kind)
    .bind(f.label)
    .bind(f.amount_cents)
    .bind(f.taxable)
    .bind(f.schedule)
    .bind(f.start_date)
    .bind(f.end_date)
    .bind(f.remaining_balance_cents)
    .bind(created_by)
    .bind(created_at)
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn update(conn: &mut SqliteConnection, id: i64, f: &ItemFields<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE recurring_items SET label = ?, amount_cents = ?, taxable = ?, schedule = ?, \
         start_date = ?, end_date = ?, remaining_balance_cents = ? WHERE id = ?",
    )
    .bind(f.label)
    .bind(f.amount_cents)
    .bind(f.taxable)
    .bind(f.schedule)
    .bind(f.start_date)
    .bind(f.end_date)
    .bind(f.remaining_balance_cents)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn delete(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM recurring_items WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Carryovers not yet taken, from payslips of posted periods that ended before `start`,
/// oldest first. Only a posted payslip's carryover is final: an earlier period can still
/// be recomputed until then.
pub async fn open_carryovers<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    start: &str,
) -> sqlx::Result<Vec<OpenCarryover>> {
    sqlx::query_as(
        "SELECT c.id, c.recurring_item_id, c.code, c.label, c.amount_cents \
         FROM deduction_carryovers c \
         JOIN payslips p ON p.id = c.source_payslip_id \
         JOIN payroll_periods pp ON pp.id = p.payroll_period_id \
         WHERE c.employee_id = ? AND c.applied_payslip_id IS NULL \
           AND pp.status = 'POSTED' AND pp.period_end < ? \
         ORDER BY pp.period_start, c.id",
    )
    .bind(employee_id)
    .bind(start)
    .fetch_all(db)
    .await
}

pub struct NewCarryover<'a> {
    pub employee_id: i64,
    pub source_payslip_id: i64,
    pub recurring_item_id: Option<i64>,
    pub code: &'a str,
    pub label: &'a str,
    pub amount_cents: i64,
}

pub async fn insert_carryover(
    conn: &mut SqliteConnection,
    c: &NewCarryover<'_>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO deduction_carryovers (employee_id, source_payslip_id, recurring_item_id, \
         code, label, amount_cents) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(c.employee_id)
    .bind(c.source_payslip_id)
    .bind(c.recurring_item_id)
    .bind(c.code)
    .bind(c.label)
    .bind(c.amount_cents)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn mark_applied(
    conn: &mut SqliteConnection,
    carryover_id: i64,
    payslip_id: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE deduction_carryovers SET applied_payslip_id = ? WHERE id = ?")
        .bind(payslip_id)
        .bind(carryover_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Takes `amount_cents` off a loan's balance, stopping at zero.
pub async fn reduce_balance(
    conn: &mut SqliteConnection,
    id: i64,
    amount_cents: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE recurring_items \
         SET remaining_balance_cents = MAX(0, remaining_balance_cents - ?) \
         WHERE id = ? AND kind = 'LOAN'",
    )
    .bind(amount_cents)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}
