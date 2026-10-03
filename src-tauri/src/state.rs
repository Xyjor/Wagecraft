/// Everything the commands share. Tauri hands it to each command as `State<AppState>`.
pub struct AppState {
    pub db: sqlx::SqlitePool,
}
