# Wagecraft

A desktop HR and payroll app for small Philippine businesses, built with Tauri 2, Rust, SQLite and React.
This is a training project; see [docs/wagecraft-plan.md](docs/wagecraft-plan.md) for the full plan.

## Set up (Windows 10/11)

1. Install Microsoft C++ Build Tools ("Desktop development with C++"), and WebView2 on Windows 10.
2. Install Rust with rustup (stable 1.80 or newer) and Node.js LTS, then run `corepack enable`.
3. In this folder: `pnpm install`.

## Everyday commands

| What            | Command                                                                              |
| --------------- | ------------------------------------------------------------------------------------ |
| Run the app     | `pnpm tauri dev`                                                                     |
| Frontend tests  | `pnpm test` (watch) or `pnpm test --run`                                             |
| Rust tests      | `cargo test` in `src-tauri/`                                                         |
| Lint and format | `pnpm lint`, `pnpm format`, `cargo fmt`, `cargo clippy --all-targets -- -D warnings` |
| Installer       | `pnpm tauri build` (NSIS)                                                            |

The database lives at `%APPDATA%\io.github.focalstack-lex.wagecraft\wagecraft.db`.

## Layout

- `src-tauri/src/` Rust: `db.rs` (open + migrate), `error.rs` (`AppError`), `commands/` (one file per area).
- `src-tauri/migrations/` SQL migrations, applied on start. Never edit one that has shipped; add a new one.
- `src/` React: `app/` (router, shell), `components/`, `features/` (one folder per screen), `lib/`.
- `src/bindings/` TypeScript types generated from Rust by ts-rs when you run `cargo test`. Don't edit them.
