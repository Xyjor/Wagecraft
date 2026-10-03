mod auth;
mod commands;
mod db;
mod error;
mod repositories;
mod services;
mod state;
mod time;

use std::time::Duration;
use tauri::Manager;

/// Sign out after this long without activity (plan §6.1). Becomes a setting later.
const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Info and up: sqlx logs every statement at Debug.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let pool = tauri::async_runtime::block_on(db::open(&dir.join("wagecraft.db")))?;
            app.manage(state::AppState {
                db: pool,
                auth: auth::session::Auth::new(IDLE_TIMEOUT),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wagecraft");
}
