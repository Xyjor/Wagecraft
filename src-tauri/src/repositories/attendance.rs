//! SQL for attendance records and the kiosk's employee lookup. The kiosk and attendance
//! services decide the rules.

use super::schedules;
use crate::domain::attendance::AttendanceRecord;
use crate::domain::attendance_calc::Computed;
use sqlx::{FromRow, SqliteConnection, SqliteExecutor};

const RECORD: &str = "SELECT id, employee_id, work_date, time_in, time_out, status, late_minutes, \
    undertime_minutes, worked_minutes, night_minutes, source, needs_review, review_note, \
    locked_by_period_id IS NOT NULL AS locked \
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

/// The employee with this number (case-insensitive) and their work schedule on `date`.
pub async fn kiosk_employee(
    conn: &mut SqliteConnection,
    employee_no: &str,
    date: &str,
) -> sqlx::Result<Option<KioskEmployee>> {
    sqlx::query_as(&format!(
        "SELECT e.id, e.employee_no, e.first_name, e.kiosk_pin_hash, e.employment_status, \
         e.archived_at, s.start_time, s.end_time, s.break_minutes, s.grace_minutes \
         FROM employees e {} WHERE e.employee_no = ?",
        schedules::join_on("?")
    ))
    .bind(date)
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

pub async fn by_id(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<Option<AttendanceRecord>> {
    sqlx::query_as(&format!("{RECORD} WHERE id = ?"))
        .bind(id)
        .fetch_optional(conn)
        .await
}

/// Who belongs on the grid for `date`: not archived, hired by then, and not yet separated.
#[derive(Debug, FromRow)]
pub struct GridEmployee {
    pub id: i64,
    pub employee_no: String,
    pub name: String,
    pub department_name: Option<String>,
    pub work_days: Option<String>,
}

pub async fn grid_employees<'e>(
    db: impl SqliteExecutor<'e>,
    date: &str,
) -> sqlx::Result<Vec<GridEmployee>> {
    sqlx::query_as(&format!(
        "SELECT e.id, e.employee_no, e.last_name || ', ' || e.first_name AS name, \
         d.name AS department_name, s.work_days \
         FROM employees e \
         LEFT JOIN departments d ON d.id = e.department_id \
         {} \
         WHERE e.archived_at IS NULL AND e.hire_date <= ?1 \
         AND (e.separation_date IS NULL OR e.separation_date >= ?1) \
         ORDER BY e.last_name, e.first_name, e.employee_no",
        schedules::join_on("?1")
    ))
    .bind(date)
    .fetch_all(db)
    .await
}

pub async fn on_date_all<'e>(
    db: impl SqliteExecutor<'e>,
    work_date: &str,
) -> sqlx::Result<Vec<AttendanceRecord>> {
    sqlx::query_as(&format!("{RECORD} WHERE work_date = ?"))
        .bind(work_date)
        .fetch_all(db)
        .await
}

/// The holidays on `date` as (name, type). A double holiday is two rows.
pub async fn holidays_on<'e>(
    db: impl SqliteExecutor<'e>,
    date: &str,
) -> sqlx::Result<Vec<(String, String)>> {
    sqlx::query_as("SELECT name, type FROM holidays WHERE date = ? ORDER BY name")
        .bind(date)
        .fetch_all(db)
        .await
}

/// Every record flagged for HR, oldest first, with the employee's number and name. A time
/// in older than `stale_before` with no time out counts too: someone forgot to clock out.
pub async fn flagged<'e>(
    db: impl SqliteExecutor<'e>,
    stale_before: &str,
) -> sqlx::Result<Vec<(String, String, AttendanceRecord)>> {
    #[derive(FromRow)]
    struct Row {
        employee_no: String,
        employee_name: String,
        #[sqlx(flatten)]
        record: AttendanceRecord,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT e.employee_no, e.last_name || ', ' || e.first_name AS employee_name, \
         a.id, a.employee_id, a.work_date, a.time_in, a.time_out, a.status, a.late_minutes, \
         a.undertime_minutes, a.worked_minutes, a.night_minutes, a.source, a.needs_review, \
         CASE WHEN a.needs_review = 1 THEN a.review_note \
              ELSE 'No time out was recorded.' END AS review_note, \
         a.locked_by_period_id IS NOT NULL AS locked \
         FROM attendance_records a JOIN employees e ON e.id = a.employee_id \
         WHERE a.needs_review = 1 \
            OR (a.time_out IS NULL AND a.time_in IS NOT NULL AND a.time_in < ?) \
         ORDER BY a.work_date, e.last_name, e.first_name",
    )
    .bind(stale_before)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.employee_no, r.employee_name, r.record))
        .collect())
}

/// An employee's schedule and whether they can still get records, for HR corrections.
#[derive(Debug, FromRow)]
pub struct EmployeeShift {
    pub employee_no: String,
    pub archived_at: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub break_minutes: Option<i64>,
    pub grace_minutes: Option<i64>,
}

pub async fn employee_shift(
    conn: &mut SqliteConnection,
    employee_id: i64,
    date: &str,
) -> sqlx::Result<Option<EmployeeShift>> {
    sqlx::query_as(&format!(
        "SELECT e.employee_no, e.archived_at, s.start_time, s.end_time, s.break_minutes, \
         s.grace_minutes FROM employees e {} WHERE e.id = ?",
        schedules::join_on("?")
    ))
    .bind(date)
    .bind(employee_id)
    .fetch_optional(conn)
    .await
}

/// The times and minutes of an HR-entered day.
pub struct Manual<'a> {
    pub employee_id: i64,
    pub work_date: &'a str,
    pub time_in: &'a str,
    pub time_out: Option<&'a str>,
    pub computed: &'a Computed,
    pub reason: &'a str,
    pub edited_by: Option<i64>,
}

/// Adds or replaces the employee's record for the day as a MANUAL one. HR's correction
/// settles any review flag, so it is cleared.
pub async fn save_manual(conn: &mut SqliteConnection, m: &Manual<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO attendance_records \
         (employee_id, work_date, time_in, time_out, status, late_minutes, undertime_minutes, \
          worked_minutes, night_minutes, source, needs_review, review_note, edit_reason, edited_by) \
         VALUES (?1, ?2, ?3, ?4, 'PRESENT', ?5, ?6, ?7, ?8, 'MANUAL', 0, NULL, ?9, ?10) \
         ON CONFLICT (employee_id, work_date) DO UPDATE SET \
          time_in = excluded.time_in, time_out = excluded.time_out, status = excluded.status, \
          late_minutes = excluded.late_minutes, undertime_minutes = excluded.undertime_minutes, \
          worked_minutes = excluded.worked_minutes, night_minutes = excluded.night_minutes, \
          source = 'MANUAL', needs_review = 0, review_note = NULL, \
          edit_reason = excluded.edit_reason, edited_by = excluded.edited_by, \
          updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
         RETURNING id",
    )
    .bind(m.employee_id)
    .bind(m.work_date)
    .bind(m.time_in)
    .bind(m.time_out)
    .bind(m.computed.late_minutes)
    .bind(m.computed.undertime_minutes)
    .bind(m.computed.worked_minutes)
    .bind(m.computed.night_minutes)
    .bind(m.reason)
    .bind(m.edited_by)
    .fetch_one(conn)
    .await
}

pub async fn clear_review(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE attendance_records SET needs_review = 0, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}
