//! SQL for departments, positions and work schedules. No rules here: the org service decides.

use crate::domain::org::{Department, Position, WorkSchedule};
use sqlx::{SqliteConnection, SqliteExecutor};

const DEPARTMENT: &str = "SELECT id, code, name, description, is_active FROM departments";

const POSITION: &str = "SELECT p.id, p.department_id, d.name AS department_name, p.title, \
    p.min_rate_cents, p.max_rate_cents, p.is_active \
    FROM positions p JOIN departments d ON d.id = p.department_id";

pub async fn departments<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<Department>> {
    sqlx::query_as(&format!("{DEPARTMENT} ORDER BY name"))
        .fetch_all(db)
        .await
}

pub async fn department<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<Department>> {
    sqlx::query_as(&format!("{DEPARTMENT} WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Another department already using `code` (case-insensitive), if any.
pub async fn department_code_taken(
    conn: &mut SqliteConnection,
    code: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM departments WHERE code = ? AND id IS NOT ?)")
        .bind(code)
        .bind(except_id)
        .fetch_one(conn)
        .await
}

pub async fn insert_department(
    conn: &mut SqliteConnection,
    code: &str,
    name: &str,
    description: Option<&str>,
) -> sqlx::Result<i64> {
    let res = sqlx::query("INSERT INTO departments (code, name, description) VALUES (?, ?, ?)")
        .bind(code)
        .bind(name)
        .bind(description)
        .execute(conn)
        .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_department(
    conn: &mut SqliteConnection,
    id: i64,
    code: &str,
    name: &str,
    description: Option<&str>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE departments SET code = ?, name = ?, description = ?, updated_at = ? WHERE id = ?",
    )
    .bind(code)
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_department_active(
    conn: &mut SqliteConnection,
    id: i64,
    active: bool,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE departments SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn positions<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<Position>> {
    sqlx::query_as(&format!("{POSITION} ORDER BY d.name, p.title"))
        .fetch_all(db)
        .await
}

pub async fn position<'e>(db: impl SqliteExecutor<'e>, id: i64) -> sqlx::Result<Option<Position>> {
    sqlx::query_as(&format!("{POSITION} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Whether the department already has a position with this title (case-insensitive).
pub async fn position_title_taken(
    conn: &mut SqliteConnection,
    department_id: i64,
    title: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM positions \
         WHERE department_id = ? AND title = ? AND id IS NOT ?)",
    )
    .bind(department_id)
    .bind(title)
    .bind(except_id)
    .fetch_one(conn)
    .await
}

pub struct PositionFields<'a> {
    pub department_id: i64,
    pub title: &'a str,
    pub min_rate_cents: Option<i64>,
    pub max_rate_cents: Option<i64>,
}

pub async fn insert_position(
    conn: &mut SqliteConnection,
    p: &PositionFields<'_>,
) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO positions (department_id, title, min_rate_cents, max_rate_cents) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(p.department_id)
    .bind(p.title)
    .bind(p.min_rate_cents)
    .bind(p.max_rate_cents)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_position(
    conn: &mut SqliteConnection,
    id: i64,
    p: &PositionFields<'_>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE positions SET department_id = ?, title = ?, min_rate_cents = ?, \
         max_rate_cents = ?, updated_at = ? WHERE id = ?",
    )
    .bind(p.department_id)
    .bind(p.title)
    .bind(p.min_rate_cents)
    .bind(p.max_rate_cents)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_position_active(
    conn: &mut SqliteConnection,
    id: i64,
    active: bool,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE positions SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

const SCHEDULE: &str = "SELECT s.id, s.name, s.start_time, s.end_time, s.break_minutes, \
    s.grace_minutes, s.work_days, s.is_active, \
    (SELECT COUNT(*) FROM employees e WHERE s.id = COALESCE( \
        (SELECT a.schedule_id FROM schedule_assignments a WHERE a.employee_id = e.id \
        AND a.effective_from <= date('now', 'localtime') ORDER BY a.effective_from DESC LIMIT 1), \
        (SELECT a.schedule_id FROM schedule_assignments a WHERE a.employee_id = e.id \
        ORDER BY a.effective_from LIMIT 1))) AS employee_count \
    FROM work_schedules s";

pub async fn schedules<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<WorkSchedule>> {
    sqlx::query_as(&format!("{SCHEDULE} ORDER BY s.name"))
        .fetch_all(db)
        .await
}

pub async fn schedule<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<WorkSchedule>> {
    sqlx::query_as(&format!("{SCHEDULE} WHERE s.id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Whether any employee has ever been assigned this schedule.
pub async fn schedule_ever_used(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM schedule_assignments WHERE schedule_id = ?)")
        .bind(id)
        .fetch_one(conn)
        .await
}

/// Another schedule already using `name` (case-insensitive), if any.
pub async fn schedule_name_taken(
    conn: &mut SqliteConnection,
    name: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM work_schedules WHERE name = ? AND id IS NOT ?)",
    )
    .bind(name)
    .bind(except_id)
    .fetch_one(conn)
    .await
}

pub struct ScheduleFields<'a> {
    pub name: &'a str,
    pub start_time: &'a str,
    pub end_time: &'a str,
    pub break_minutes: i64,
    pub grace_minutes: i64,
    pub work_days: &'a str,
}

pub async fn insert_schedule(
    conn: &mut SqliteConnection,
    s: &ScheduleFields<'_>,
) -> sqlx::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO work_schedules \
         (name, start_time, end_time, break_minutes, grace_minutes, work_days) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(s.name)
    .bind(s.start_time)
    .bind(s.end_time)
    .bind(s.break_minutes)
    .bind(s.grace_minutes)
    .bind(s.work_days)
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_schedule(
    conn: &mut SqliteConnection,
    id: i64,
    s: &ScheduleFields<'_>,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE work_schedules SET name = ?, start_time = ?, end_time = ?, break_minutes = ?, \
         grace_minutes = ?, work_days = ?, updated_at = ? WHERE id = ?",
    )
    .bind(s.name)
    .bind(s.start_time)
    .bind(s.end_time)
    .bind(s.break_minutes)
    .bind(s.grace_minutes)
    .bind(s.work_days)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_schedule_active(
    conn: &mut SqliteConnection,
    id: i64,
    active: bool,
    now: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE work_schedules SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
