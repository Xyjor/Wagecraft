# ADR-001: Tauri 2 as the desktop framework

- **Status:** Accepted (trainer requirement)
- **Context:** A Windows desktop app is needed with a modern UI and access to local files.
- **Decision:** Tauri 2 with the system webview (WebView2 on Windows) and a Rust backend.
- **Alternatives:** _Electron_ is the most popular, but installers are 80 MB+ and memory use is high. _.NET WPF/WinUI_ is Windows-only, with a different skill set.
- **Consequences:** ➕ small installers, low memory use, strong security model, valuable Rust experience. ➖ Rust learning curve; the UI can render slightly differently across OS webviews.
