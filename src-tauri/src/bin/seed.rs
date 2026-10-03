//! Adds a demo company (3 work schedules, 6 departments, 200 employees with salary
//! history) to an empty Wagecraft database. Usage, from the repository root:
//!
//!     pnpm seed                 # the app's own database
//!     pnpm seed path/to.db      # any other database file
//!
//! Close the app first. Sign in afterwards with your own Admin account as usual.

use std::path::PathBuf;

const APP_ID: &str = "io.github.xyjor.wagecraft";

fn main() {
    if let Err(e) = seed() {
        eprintln!("Seeding failed: {e:#}");
        std::process::exit(1);
    }
}

fn seed() -> anyhow::Result<()> {
    let path = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => app_data_dir()?.join("wagecraft.db"),
    };
    println!("Seeding {}", path.display());
    let s = wagecraft_lib::seed::run(&path, 200)?;
    println!(
        "Added {} work schedules, {} departments, {} positions, {} employees and {} pay rates.",
        s.schedules, s.departments, s.positions, s.employees, s.rates
    );
    Ok(())
}

/// Where Tauri keeps the app's data on each OS (the same folder `app_data_dir()` returns).
fn app_data_dir() -> anyhow::Result<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    let base = base.ok_or_else(|| anyhow::anyhow!("can't find the app data folder"))?;
    let dir = base.join(APP_ID);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
