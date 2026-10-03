fn main() {
    // sqlx::migrate! embeds the SQL files at compile time; rebuild when they change.
    println!("cargo:rerun-if-changed=migrations");
    tauri_build::build()
}
