//! The kiosk and attendance screens. Kiosk punches need no session: the employee number
//! and PIN are checked instead, with a lockout (plan §3.3, §6.3).

use crate::auth::permissions::Permission;
use crate::domain::attendance::{
    AttendanceInput, AttendanceRecord, DateRange, DayRow, KioskPunch, PunchKind, ReviewItem,
};
use crate::error::AppError;
use crate::services::{accounts, attendance, kiosk};
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::State;

/// Takes no time from the UI: the backend reads the PC clock itself.
#[tauri::command]
pub async fn kiosk_clock_in(
    state: State<'_, AppState>,
    employee_no: String,
    pin: String,
) -> Result<KioskPunch, AppError> {
    let clock = kiosk::Clock::now();
    kiosk::punch(
        &state.db,
        &state.kiosk,
        &employee_no,
        &pin,
        PunchKind::In,
        clock,
    )
    .await
}

#[tauri::command]
pub async fn kiosk_clock_out(
    state: State<'_, AppState>,
    employee_no: String,
    pin: String,
) -> Result<KioskPunch, AppError> {
    let clock = kiosk::Clock::now();
    kiosk::punch(
        &state.db,
        &state.kiosk,
        &employee_no,
        &pin,
        PunchKind::Out,
        clock,
    )
    .await
}

/// The signed-in person's own attendance. The employee comes from the session, never the UI.
#[tauri::command]
pub async fn attendance_mine(
    state: State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<AttendanceRecord>, AppError> {
    let session = state.require(Permission::SelfAttendance).await?;
    let id = accounts::employee_of(&state.db, session.user_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    attendance::for_employee(&state.db, id, &range).await
}

/// HR's grid: every current employee's day on `date` (`YYYY-MM-DD`).
#[tauri::command]
pub async fn attendance_day(
    state: State<'_, AppState>,
    date: String,
) -> Result<Vec<DayRow>, AppError> {
    state.require(Permission::AttendanceReadAll).await?;
    attendance::day(&state.db, &date).await
}

/// Records flagged for HR to look at.
#[tauri::command]
pub async fn attendance_review_queue(
    state: State<'_, AppState>,
) -> Result<Vec<ReviewItem>, AppError> {
    state.require(Permission::AttendanceReadAll).await?;
    attendance::review_queue(&state.db).await
}

/// Any employee's records over a range, for their profile.
#[tauri::command]
pub async fn attendance_for_employee(
    state: State<'_, AppState>,
    id: i64,
    range: DateRange,
) -> Result<Vec<AttendanceRecord>, AppError> {
    state.require(Permission::AttendanceReadAll).await?;
    attendance::for_employee(&state.db, id, &range).await
}

/// Adds or corrects a day. A reason is required, and the change is audited.
#[tauri::command]
pub async fn attendance_save(
    state: State<'_, AppState>,
    input: AttendanceInput,
) -> Result<AttendanceRecord, AppError> {
    let session = state.require(Permission::AttendanceEdit).await?;
    // "Today" is the PC's calendar day, which is the company's day (plan §5.1).
    let today = Local::now().date_naive();
    attendance::save(&state.db, session.actor(), input, today, Utc::now()).await
}

#[tauri::command]
pub async fn attendance_mark_reviewed(
    state: State<'_, AppState>,
    id: i64,
) -> Result<AttendanceRecord, AppError> {
    let session = state.require(Permission::AttendanceEdit).await?;
    attendance::mark_reviewed(&state.db, session.actor(), id, Utc::now()).await
}
