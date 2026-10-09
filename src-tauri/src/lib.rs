mod audit;
mod auth;
mod commands;
mod db;
mod domain;
mod error;
mod export;
mod repositories;
pub mod seed;
mod services;
mod state;
mod time;

use tauri::Manager;

/// The daily backup at app start, into the folder and with the count set in Settings.
async fn daily_backup(
    db: &sqlx::SqlitePool,
    default_dir: &std::path::Path,
) -> Result<(), error::AppError> {
    let dir = services::settings::backup_dir(db, default_dir).await?;
    let keep = services::settings::backup_keep(db).await?;
    let local = chrono::Local::now().naive_local();
    services::backups::daily(db, &dir, keep, local, chrono::Utc::now()).await?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Info and up: sqlx logs every statement at Debug.
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let pool = tauri::async_runtime::block_on(db::open(&dir.join(db::DB_FILE)))?;
            // After a restore, record it in the restored database before anyone signs in.
            let finished = services::restore::finish(&pool, &dir, chrono::Utc::now());
            if let Err(e) = tauri::async_runtime::block_on(finished) {
                log::error!("couldn't record the restore: {e}");
            }
            let default_backup_dir = dir.join("backups");
            let idle_timeout =
                tauri::async_runtime::block_on(services::settings::idle_timeout(&pool))?;
            let (db, default_dir) = (pool.clone(), default_backup_dir.clone());
            tauri::async_runtime::spawn(async move {
                // A failed daily backup must not stop the app; it tries again next start.
                if let Err(e) = daily_backup(&db, &default_dir).await {
                    log::error!("daily backup failed: {e}");
                }
            });
            app.manage(state::AppState {
                db: pool,
                auth: auth::session::Auth::new(idle_timeout),
                kiosk: auth::kiosk_lock::KioskLock::default(),
                default_backup_dir,
                data_dir: dir,
                pending_restore: std::sync::Mutex::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::system_health,
            commands::auth::auth_setup_status,
            commands::auth::auth_setup_create_admin,
            commands::auth::auth_login,
            commands::auth::auth_logout,
            commands::auth::auth_me,
            commands::auth::auth_change_password,
            commands::users::user_list,
            commands::users::user_create,
            commands::users::user_update,
            commands::users::user_set_active,
            commands::users::user_reset_password,
            commands::org::department_list,
            commands::org::department_create,
            commands::org::department_update,
            commands::org::department_set_active,
            commands::org::position_list,
            commands::org::position_create,
            commands::org::position_update,
            commands::org::position_set_active,
            commands::org::schedule_list,
            commands::org::schedule_create,
            commands::org::schedule_update,
            commands::org::schedule_set_active,
            commands::org::holiday_list,
            commands::org::holiday_create,
            commands::org::holiday_update,
            commands::org::holiday_delete,
            commands::employees::employee_list,
            commands::employees::employee_get,
            commands::employees::employee_me,
            commands::employees::employee_create,
            commands::employees::employee_update,
            commands::employees::employee_archive,
            commands::employees::employee_compensation_history,
            commands::employees::employee_add_compensation,
            commands::employees::employee_recurring_items,
            commands::employees::employee_add_recurring_item,
            commands::employees::recurring_item_update,
            commands::employees::recurring_item_delete,
            commands::employees::employee_schedule_history,
            commands::employees::employee_change_schedule,
            commands::employees::employee_account,
            commands::employees::employee_linkable_accounts,
            commands::employees::employee_create_account,
            commands::employees::employee_link_user,
            commands::employees::employee_unlink_user,
            commands::employees::employee_set_kiosk_pin,
            commands::attendance::kiosk_clock_in,
            commands::attendance::kiosk_clock_out,
            commands::attendance::attendance_mine,
            commands::attendance::attendance_day,
            commands::attendance::attendance_review_queue,
            commands::attendance::attendance_for_employee,
            commands::attendance::attendance_save,
            commands::attendance::attendance_mark_reviewed,
            commands::overtime::overtime_request_create,
            commands::overtime::overtime_request_cancel,
            commands::overtime::overtime_mine,
            commands::overtime::overtime_list,
            commands::overtime::overtime_pending,
            commands::overtime::overtime_decide,
            commands::payroll::payroll_period_list,
            commands::payroll::payroll_period_checks,
            commands::payroll::payroll_period_create,
            commands::payroll::payroll_period_delete,
            commands::payroll::rule_pack_list,
            commands::payroll::rule_pack_get,
            commands::payroll::payroll_compute,
            commands::payroll::payroll_approve,
            commands::payroll::payroll_send_back,
            commands::payroll::payroll_post,
            commands::payroll::payroll_register,
            commands::payroll::payslip_get,
            commands::payroll::payslip_my_list,
            commands::payroll::payslip_my_get,
            commands::reports::export_save_pdf,
            commands::leave::leave_types_list,
            commands::leave::leave_type_create,
            commands::leave::leave_type_update,
            commands::leave::leave_type_set_active,
            commands::leave::leave_my_balances,
            commands::leave::leave_balances_for,
            commands::leave::leave_balance_grant,
            commands::leave::leave_balance_adjust,
            commands::leave::leave_request_create,
            commands::leave::leave_request_cancel,
            commands::leave::leave_my_requests,
            commands::leave::leave_request_list,
            commands::leave::leave_request_pending,
            commands::leave::leave_request_decide,
            commands::reports::report_masterlist_csv,
            commands::reports::report_payroll_register,
            commands::reports::report_payroll_register_csv,
            commands::backups::backup_list,
            commands::backups::backup_create,
            commands::backups::backup_restore_choose,
            commands::backups::backup_restore,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::settings_pick_backup_folder,
            commands::dashboard::dashboard_team,
            commands::dashboard::dashboard_mine,
            commands::audit::audit_list,
            commands::audit::audit_filters,
            commands::audit::audit_export_csv,
            commands::settings::settings_pick_logo,
            commands::settings::settings_clear_logo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wagecraft");
}
