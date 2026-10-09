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

The database lives at `%APPDATA%\io.github.xyjor.wagecraft\wagecraft.db`.

## Releases

1. `pnpm version:set 1.0.0` sets the version in `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/tauri.conf.json` and `Cargo.lock`. Date the version in `CHANGELOG.md`, then commit.
2. `git tag v1.0.0 && git push origin v1.0.0`. The `release` workflow builds the NSIS installer on
   Windows and attaches it to a draft GitHub Release named after the tag. It fails if the tag and
   the app version disagree.
3. Install it on a clean Windows PC and run the smoke test (first run, sign in, one payroll), then
   publish the draft.

The installer is per-user, so it needs no admin rights, and it brings the WebView2 bootstrapper
for PCs without WebView2. It isn't code-signed, so Windows SmartScreen shows "Windows protected
your PC" the first time: click **More info**, then **Run anyway**. For an installer without a
tag, run the `release` workflow from the Actions tab and download the `wagecraft-installer`
artifact.

## Layout

- `src-tauri/src/` Rust: `db.rs` (open + migrate), `error.rs` (`AppError`), `commands/` (one file per area).
- `src-tauri/migrations/` SQL migrations, applied on start. Never edit one that has shipped; add a new one.
- `src/` React: `app/` (router, shell), `components/`, `features/` (one folder per screen), `lib/`.
- `src/bindings/` TypeScript types generated from Rust by ts-rs when you run `cargo test`. Don't edit them.
