# ADR-002: All business logic and authorization in Rust; no SQL from the frontend

- **Status:** Accepted
- **Context:** Tauri offers `tauri-plugin-sql`, which lets the webview run SQL directly. That is quick to start with, but anything the UI can do, a user with devtools can do too.
- **Decision:** The webview calls only named Rust commands. Rust validates input, checks permissions, and runs SQL.
- **Alternatives:** _SQL plugin from the frontend_: less code, but RBAC can be bypassed and logic gets scattered across components.
- **Consequences:** ➕ one trusted place for rules, testable without the UI, real security. ➖ more boilerplate (command + service + repository per feature).
