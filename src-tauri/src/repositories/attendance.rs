//! SQL for attendance records and the kiosk's employee lookup. The kiosk and attendance
//! services decide the rules.

use crate::domain::attendance::AttendanceRecord;
use crate::domain::attendance_calc::Computed;
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const RECORD: &str = "SELECT id, employee_id, work_date, time_in, time_out, status, late_minutes, \
    undertime_minutes, worked_minutes, night_minutes, source, needs_review, review_note \
    FROM attendance_records";

/// What the kiosk needs to check a PIN and do the time math. Never sent to the UI.
#[derive(Debug, FromRow)]
pub struct KioskEmployee {
    pub id: i64,
    pub employee_no: String,
    pub first_name: String,
    pub kiosk_pin_hash: Option<String>,
    pub employment_status: String,
    pub archived_at: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub break_minutes: Option<i64>,
    pub grace_minutes: Option<i64>,
}

/// The employee with this number (case-insensitive) and their work schedule, if any.
pub async fn kiosk_employee(
    conn: &mut SqliteConnection,
    employee_no: &str,
) -> sqlx::Result<Option<KioskEmployee>> {
    sqlx::query_as(
        "SELECT e.id, e.employee_no, e.first_name, e.kiosk_pin_hash, e.employment_status, \
         e.archived_at, s.start_time, s.end_time, s.break_minutes, s.grace_minutes \
         FROM employees e LEFT JOIN work_schedules s ON s.id = e.schedule_id \
         WHERE e.employee_no = ?",
    )
    .bind(employee_no)
    .fetch_optional(conn)
    .await
}

pub async fn on_date(
    conn: &mut SqliteConnection,
    employee_id: i64,
    work_date: &str,
) -> sqlx::Result<Option<AttendanceRecord>> {
    sqlx::query_as(&format!("{RECORD} WHERE employee_id = ? AND work_date = ?"))
        .bind(employee_id)
        .bind(work_date)
        .fetch_optional(conn)
        .await
}

/// The latest record with a time in and no time out yet.
pub async fn latest_open(
    conn: &mut SqliteConnection,
    employee_id: i64,
) -> sqlx::Result<Option<AttendanceRecord>> {
    sqlx::query_as(&format!(
        "{RECORD} WHERE employee_id = ? AND time_in IS NOT NULL AND time_out IS NULL \
         ORDER BY work_date DESC LIMIT 1"
    ))
    .bind(employee_id)
    .fetch_optional(conn)
    .await
}

/// The employee's most recent punch, in or out, on any day.
pub async fn last_punch(
    conn: &mut SqliteConnection,
    employee_id: i64,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT MAX(t) FROM (\
         SELECT time_in AS t FROM attendance_records WHERE employee_id = ?1 \
         UNION ALL SELECT time_out FROM attendance_records WHERE employee_id = ?1)",
    )
    .bind(employee_id)
    .fetch_one(conn)
    .await
}

/// A review flag and the reason for it.
pub struct Review<'a> {
    pub needed: bool,
    pub note: Option<&'a str>,
}

pub async fn insert_clock_in(
    conn: &mut SqliteConnection,
    employee_id: i64,
    work_date: &str,
    time_in: &str,
    late_minutes: i64,
    review: &Review<'_>,
) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO attendance_records \
         (employee_id, work_date, time_in, status, late_minutes, source, needs_review, review_note) \
         VALUES (?, ?, ?, 'PRESENT', ?, 'CLOCK', ?, ?)",
    )
    .bind(employee_id)
    .bind(work_date)
    .bind(time_in)
    .bind(late_minutes)
    .bind(review.needed)
    .bind(review.note)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

/// Saves the time out and the recomputed minutes. A new review flag never clears an
/// earlier one, and its note is added after the earlier note.
pub async fn set_clock_out(
    conn: &mut SqliteConnection,
    id: i64,
    time_out: &str,
    c: &Computed,
    review: &Review<'_>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE attendance_records SET time_out = ?1, late_minutes = ?2, undertime_minutes = ?3, \
         worked_minutes = ?4, night_minutes = ?5, needs_review = needs_review OR ?6, \
         review_note = CASE WHEN ?7 IS NULL THEN review_note \
                            WHEN review_note IS NULL THEN ?7 \
                            ELSE review_note || ' ' || ?7 END, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
         WHERE id = ?8",
    )
    .bind(time_out)
    .bind(c.late_minutes)
    .bind(c.undertime_minutes)
    .bind(c.worked_minutes)
    .bind(c.night_minutes)
    .bind(review.needed)
    .bind(review.note)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// One employee's records from `from` to `to` (inclusive), newest first.
pub async fn for_employee<'e>(
    db: impl SqliteExecutor<'e>,
    employee_id: i64,
    from: &str,
    to: &str,
) -> sqlx::Result<Vec<AttendanceRecord>> {
    sqlx::query_as(&format!(
        "{RECORD} WHERE employee_id = ? AND work_date BETWEEN ? AND ? ORDER BY work_date DESC"
    ))
    .bind(employee_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}
