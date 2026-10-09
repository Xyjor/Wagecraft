# Wagecraft

A desktop HR and payroll app for small Philippine businesses, built with Tauri 2, Rust, SQLite
and React. One PC runs it; staff clock in on a shared time clock screen, HR keeps attendance,
leave and overtime, and payroll computes SSS, PhilHealth, Pag-IBIG and withholding tax under a
dated rule pack. Money is handled as integer centavos, every change is audited, and the database
backs itself up.

This is a training project. The full plan is in [docs/wagecraft-plan.md](docs/wagecraft-plan.md);
version 1.0.0 is due on Nov 1, 2026.

## Documentation

| Document                                            | What it is                                                              |
| --------------------------------------------------- | ----------------------------------------------------------------------- |
| [User manual](docs/user-manual.md)                  | Every screen, by role: Admin, HR, Staff and the time clock.             |
| [Manual test script](docs/manual-test-script.md)    | The seven flows to run on the installer before each release.            |
| [Release checklist](docs/release-checklist.md)      | The security and performance checks, and what proves each one.          |
| [Architecture decision records](docs/adr/README.md) | Why the app is built the way it is, one decision per file.              |
| [Changelog](CHANGELOG.md)                           | What each version changed.                                              |
| [Plan](docs/wagecraft-plan.md)                      | Scope, data model, module designs, security checklist and the timeline. |

## Set up (Windows 10/11)

1. Install Microsoft C++ Build Tools ("Desktop development with C++"), and WebView2 on Windows 10.
2. Install Rust with rustup (stable 1.80 or newer) and Node.js LTS, then run `corepack enable`.
3. In this folder: `pnpm install`.

On Linux, install `libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev` first, and run
the app with `WEBKIT_DISABLE_DMABUF_RENDERER=1` if the window stays blank.

## Everyday commands

| What            | Command                                                                              |
| --------------- | ------------------------------------------------------------------------------------ |
| Run the app     | `pnpm tauri dev`                                                                     |
| Demo company    | `pnpm seed` (200 employees; sign in as `admin` / `correct horse`, kiosk PIN `1234`)  |
| Frontend tests  | `pnpm test` (watch) or `pnpm test --run`                                             |
| Rust tests      | `cargo test` in `src-tauri/` (also regenerates `src/bindings/`)                      |
| Lint and format | `pnpm lint`, `pnpm format`, `cargo fmt`, `cargo clippy --all-targets -- -D warnings` |
| Installer       | `pnpm tauri build` (NSIS)                                                            |
| Set the version | `pnpm version:set 1.0.0`                                                             |

CI runs the same checks on every pull request: `pnpm lint`, `pnpm format:check`,
`pnpm test --run`, `pnpm build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
and `cargo test`, on Windows. Run them before you push.

The database lives at `%APPDATA%\io.github.xyjor.wagecraft\wagecraft.db`, with daily backups in a
`backups` folder next to it.

## How it is built

The frontend never touches the database. Every screen calls a Tauri command; the command checks
the session and the role's permission, then a service applies the business rules and writes
through a repository. Rust owns all the logic, so a command typed into the developer console gets
the same `Forbidden` as a hidden button would.

- **Rust** (`src-tauri/src/`): `commands/` one file per area, each command starting with
  `state.require(Permission)`; `services/` the rules (sign-in lockout, attendance, leave, payroll
  approval, backups); `repositories/` the SQL; `domain/` the types shared with the frontend and the
  pure payroll engine (`domain/payroll`); `auth/` sessions, passwords and permissions; `audit.rs`
  the append-only log written in the same transaction as each change.
- **Data**: SQLite via sqlx, in WAL mode. `src-tauri/migrations/` run on start; never edit one
  that has shipped, add a new one. Money is integer centavos everywhere; the payroll engine does
  its math with rust_decimal. Statutory rates are rule packs with an effective date, so old
  payslips keep the rates they were made with.
- **React** (`src/`): `app/` the router and shell, `features/` one folder per screen, `components/`
  shared form and layout pieces, `lib/` IPC, money, dates and theme helpers. Pages load data with
  a `call()` wrapper around Tauri's `invoke`, and forms validate with zod before the round trip.
  `src/bindings/` holds TypeScript types generated from the Rust types by ts-rs; don't edit them.
- **PDFs**: payslips and the payroll register are React documents rendered with
  `@react-pdf/renderer`; Rust only writes the file where the user chose.
- **Tests**: Rust unit tests per service against a temporary SQLite file, 20 hand-worked payroll
  cases in `src-tauri/tests/payroll_cases/`, Vitest with Testing Library for the pages, and a
  check that every command is registered and starts with a permission check.

The decisions behind this are written up in [docs/adr/](docs/adr/README.md).

## Layout

- `src-tauri/src/` Rust, as above; `src-tauri/src/bin/seed.rs` builds the demo company.
- `src-tauri/migrations/` SQL migrations, applied on start.
- `src-tauri/tests/payroll_cases/` golden payroll cases as JSON.
- `src/` React: `app/`, `components/`, `features/`, `lib/`, `bindings/`.
- `scripts/` `set-version.mjs`, used by `pnpm version:set`.
- `docs/` the plan, ADRs, user manual, test script and screenshots.
- `.github/workflows/` `ci.yml` (checks on every PR) and `release.yml` (installer on a tag).

## Working on it

Work happens on a branch, one pull request at a time, reviewed before it merges to `main`. Each
change starts with a failing test. Keep commands thin, rules in services, and SQL in repositories;
a new screen gets a feature folder with its page, its `api.ts` and its tests.

## Releases

1. `pnpm version:set 1.0.0` sets the version in `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/tauri.conf.json` and `Cargo.lock`. Date the version in `CHANGELOG.md`, then commit.
2. `git tag v1.0.0 && git push origin v1.0.0`. The `release` workflow builds the NSIS installer on
   Windows and attaches it to a draft GitHub Release named after the tag. It fails if the tag and
   the app version disagree.
3. Install it on a clean Windows PC and run the [manual test script](docs/manual-test-script.md)
   and the manual half of the [release checklist](docs/release-checklist.md), then publish the
   draft.

The installer is per-user, so it needs no admin rights, and it brings the WebView2 bootstrapper
for PCs without WebView2. It isn't code-signed, so Windows SmartScreen shows "Windows protected
your PC" the first time: click **More info**, then **Run anyway**. For an installer without a
tag, run the `release` workflow from the Actions tab and download the `wagecraft-installer`
artifact.

## License

MIT, see [LICENSE](LICENSE).
