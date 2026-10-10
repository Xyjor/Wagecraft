//! Guards for the release checklist (docs/release-checklist.md): the parts of the Tauri
//! configuration that must stay locked down (plan §8.2), checked on every test run instead of
//! by eye before a release. Each test names the plan item it stands for.

use serde_json::{json, Value};

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("valid JSON")
}

/// Capabilities: the webview gets only core permissions. Files are read and written in Rust,
/// so the frontend needs no `fs`, `dialog`, `shell` or `http` permission.
#[test]
fn the_webview_gets_only_core_permissions() {
    let capability = parse(include_str!("../capabilities/default.json"));
    assert_eq!(capability["windows"], json!(["main"]));
    assert_eq!(capability["permissions"], json!(["core:default"]));
}

/// CSP: scripts only from the app itself, connections only to Tauri's IPC, no asset protocol.
/// `'wasm-unsafe-eval'` lets those scripts compile the WebAssembly they carry (the PDF
/// library's layout engine); it allows no `eval` and no outside script.
#[test]
fn the_window_has_a_content_security_policy() {
    let conf = parse(include_str!("../tauri.conf.json"));
    let security = &conf["app"]["security"];
    let csp = security["csp"].as_str().expect("app.security.csp is set");
    let directives: Vec<(&str, Vec<&str>)> = csp
        .split(';')
        .map(|d| {
            let mut words = d.split_whitespace();
            (words.next().unwrap_or(""), words.collect())
        })
        .collect();
    let sources = |name: &str| -> Vec<&str> {
        directives
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("csp has no {name}"))
            .1
            .clone()
    };
    assert_eq!(sources("default-src"), ["'self'"]);
    assert_eq!(sources("script-src"), ["'self'", "'wasm-unsafe-eval'"]);
    assert_eq!(sources("connect-src"), ["ipc:", "http://ipc.localhost"]);
    assert_eq!(sources("object-src"), ["'none'"]);
    for (name, values) in &directives {
        for value in values {
            assert!(
                !matches!(*value, "*" | "'unsafe-eval'" | "http:" | "https:"),
                "{name} allows {value}"
            );
        }
    }
    assert_ne!(
        security["assetProtocol"]["enable"],
        json!(true),
        "the asset protocol stays off"
    );
}

/// Devtools: debug builds have them, release builds must not compile them in.
#[test]
fn release_builds_leave_devtools_out() {
    let cargo_toml = include_str!("../Cargo.toml");
    let tauri = cargo_toml
        .lines()
        .find(|line| line.starts_with("tauri = "))
        .expect("Cargo.toml declares the tauri dependency on one line");
    assert!(!tauri.contains("devtools"), "{tauri}");
    assert!(
        !cargo_toml.contains("tauri/devtools"),
        "no feature turns devtools on"
    );
}
