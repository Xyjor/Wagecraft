pub mod attendance;
pub mod auth;
pub mod employees;
pub mod leave;
pub mod org;
pub mod overtime;
pub mod payroll;
pub mod reports;
pub mod system;
pub mod users;

#[cfg(test)]
mod tests {
    //! Guards for plan §3.2: "the backend check is the real one". These read the source,
    //! so a new command file is covered the moment it exists.

    use std::{fs, path::Path};

    /// The only commands that may run without a signed-in session.
    const PUBLIC: &[&str] = &[
        "system_health",
        "auth_setup_status",
        "auth_setup_create_admin", // refuses once any user exists
        "auth_login",
        "auth_logout",     // ending no session is harmless
        "kiosk_clock_in",  // checks the employee number and PIN, with a lockout
        "kiosk_clock_out", // same
    ];

    /// Every `#[tauri::command]` as (name, body), from every file in src/commands.
    fn commands() -> Vec<(String, String)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let mut out = Vec::new();
        for file in fs::read_dir(dir).expect("read src/commands") {
            let path = file.expect("entry").path();
            if path.ends_with("mod.rs") {
                continue; // this file, whose strings mention the attribute
            }
            let src = fs::read_to_string(path).expect("read file");
            for chunk in src.split("#[tauri::command]").skip(1) {
                let after_fn = chunk.split_once("fn ").expect("fn after attribute").1;
                let name = after_fn.split('(').next().expect("fn name").trim();
                out.push((name.to_string(), chunk.to_string()));
            }
        }
        assert!(
            out.len() >= 10,
            "expected to find the commands, found {out:?}"
        );
        out
    }

    #[test]
    fn every_non_public_command_checks_a_permission() {
        let missing: Vec<_> = commands()
            .into_iter()
            .filter(|(name, body)| {
                !PUBLIC.contains(&name.as_str()) && !body.contains("state.require(")
            })
            .map(|(name, _)| name)
            .collect();
        assert!(
            missing.is_empty(),
            "these commands never call state.require(..): {missing:?}"
        );
    }

    #[test]
    fn every_command_is_registered() {
        let lib = include_str!("../lib.rs");
        let missing: Vec<_> = commands()
            .into_iter()
            .map(|(name, _)| name)
            .filter(|name| !lib.contains(&format!("::{name},")))
            .collect();
        assert!(missing.is_empty(), "not in generate_handler!: {missing:?}");
    }
}
