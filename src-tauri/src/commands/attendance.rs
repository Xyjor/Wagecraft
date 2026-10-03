//! The kiosk and attendance screens. Kiosk punches need no session: the employee number
//! and PIN are checked instead, with a lockout (plan §3.3, §6.3).

use crate::auth::permissions::Permission;
use crate::domain::attendance::{AttendanceRecord, DateRange, KioskPunch, PunchKind};
use crate::error::AppError;
use crate::services::{accounts, attendance, kiosk};
use crate::state::AppState;
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
