# ADR-007: Sessions held in Rust memory with idle timeout

- **Status:** Accepted
- **Context:** Desktop app, one window, one signed-in user at a time.
- **Decision:** `Mutex<Option<Session>>` in `AppState`; every command checks it; idle timeout is enforced server-side.
- **Alternatives:** _JWT in localStorage_: unnecessary on desktop, and readable by any script in the webview.
- **Consequences:** ➕ nothing secret in the webview; trivial logout. ➖ only one user signed in per app instance (fine for this scope).
