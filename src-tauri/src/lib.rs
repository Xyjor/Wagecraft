mod commands;
mod db;
mod error;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let pool = tauri::async_runtime::block_on(db::open(&dir.join("wagecraft.db")))?;
            app.manage(state::AppState { db: pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::system::system_health])
        .run(tauri::generate_context!())
        .expect("error while running Wagecraft");
}
