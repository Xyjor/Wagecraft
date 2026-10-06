//! Payroll periods and payslips (plan §6.5). Creating, deleting and computing periods
//! needs `payroll.compute`; reading any payslip needs `payslip.read_all`.

use crate::auth::permissions::Permission;
use crate::domain::payroll_period::{
    PayrollPeriod, PayrollPeriodInput, PayrollRegister, PayslipDetail, PeriodChecks,
    RulePackSummary,
};
use crate::error::AppError;
use crate::services::{payroll, payroll_approval, payslips};
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::State;

#[tauri::command]
pub async fn payroll_period_list(
    state: State<'_, AppState>,
) -> Result<Vec<PayrollPeriod>, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::list(&state.db).await
}

#[tauri::command]
pub async fn rule_pack_list(state: State<'_, AppState>) -> Result<Vec<RulePackSummary>, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::rule_packs(&state.db).await
}

/// Unfinished leave, overtime and time-outs inside a period, shown before creating it.
#[tauri::command]
pub async fn payroll_period_checks(
    state: State<'_, AppState>,
    period_start: String,
) -> Result<PeriodChecks, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::checks(&state.db, &period_start).await
}

#[tauri::command]
pub async fn payroll_period_create(
    state: State<'_, AppState>,
    input: PayrollPeriodInput,
) -> Result<PayrollPeriod, AppError> {
    let session = state.require(Permission::PayrollCompute).await?;
    payroll::create(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn payroll_period_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.require(Permission::PayrollCompute).await?;
    payroll::delete(&state.db, session.actor(), id, Utc::now()).await
}

/// Computes, or recomputes, every payslip in the period.
#[tauri::command]
pub async fn payroll_compute(
    state: State<'_, AppState>,
    period_id: i64,
) -> Result<PayrollRegister, AppError> {
    let session = state.require(Permission::PayrollCompute).await?;
    payslips::compute(&state.db, session.actor(), period_id, Utc::now()).await
}

#[tauri::command]
pub async fn payroll_approve(
    state: State<'_, AppState>,
    period_id: i64,
) -> Result<PayrollRegister, AppError> {
    let session = state.require(Permission::PayrollApprove).await?;
    payroll_approval::approve(&state.db, session.actor(), period_id, Utc::now()).await?;
    payslips::register(&state.db, period_id).await
}

#[tauri::command]
pub async fn payroll_send_back(
    state: State<'_, AppState>,
    period_id: i64,
    reason: String,
) -> Result<PayrollRegister, AppError> {
    let session = state.require(Permission::PayrollApprove).await?;
    payroll_approval::send_back(&state.db, session.actor(), period_id, &reason, Utc::now()).await?;
    payslips::register(&state.db, period_id).await
}

#[tauri::command]
pub async fn payroll_post(
    state: State<'_, AppState>,
    period_id: i64,
) -> Result<PayrollRegister, AppError> {
    let session = state.require(Permission::PayrollPost).await?;
    let local = Local::now();
    payroll_approval::post(
        &state.db,
        session.actor(),
        period_id,
        &state.backup_dir,
        local.naive_local(),
        local.date_naive(),
        Utc::now(),
    )
    .await?;
    payslips::register(&state.db, period_id).await
}

#[tauri::command]
pub async fn payroll_register(
    state: State<'_, AppState>,
    period_id: i64,
) -> Result<PayrollRegister, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payslips::register(&state.db, period_id).await
}

#[tauri::command]
pub async fn payslip_get(state: State<'_, AppState>, id: i64) -> Result<PayslipDetail, AppError> {
    state.require(Permission::PayslipReadAll).await?;
    payslips::payslip(&state.db, id).await
}
