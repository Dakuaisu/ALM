# ALM

A rebuild of a single-file HTML shop manager (billing, stock, customers, cash book, purchase
orders, GST bills) as a Rust core with a Tauri app for macOS, Windows and Android, and a
Postgres authority exposed through PostgREST and GoTrue. The original HTML is the behavioural
oracle; it is not in this repository. Everything here runs on synthetic data.

## Layout

| Path | What it is |
|---|---|
| `phase2-local/crates/alm-core` | Business rules: drafts, pricing, posting, stock and FIFO, books, bill images as display lists. No I/O. The seeded synthetic shop is behind the `demo-fixture` feature. |
| `phase2-local/crates/alm-store` | Device storage in SQLite: drafts per account, the outbox of queued requests, the pulled projection, preferences and archives. |
| `phase2-local/crates/alm-sync` | The device's view of the authority: intents, outcomes, the REST/GoTrue client and the incremental pull. `LocalAuthority` (feature `local-authority`) is an in-process simulation for development. |
| `phase2-local/crates/alm-app` | The Tauri app. Rust renders every screen's HTML; `ui/glue.js` only routes clicks and form fields. |
| `phase2-local/crates/alm-cli` | Development checks over an authority's stores, and a terminal draft demo. |
| `supabase/migrations` | The authority schema and functions, applied in order. |
| `supabase/tests` | pgTAP suites and the plain-Postgres bootstrap used by the server checks. |
| `docs/` | Requirements, decisions, open questions and test results. |

## Build and test

Rust 1.98 with a populated Cargo cache. From `phase2-local`:

```sh
cargo test --locked --offline --workspace
./scripts/local_ci.sh
```

`local_ci.sh` runs what the three GitHub workflows run and stops at the first failure: fmt,
clippy, workspace tests, the server checks (`scripts/server_checks.sh`, disposable Postgres,
PostgREST and GoTrue containers on 127.0.0.1, removed afterwards), the macOS app and disk image,
the Android release APK and a Windows cross-build. It needs Docker, the Tauri CLI 2.12.6, JDK
17, the Android SDK and NDK, and cargo-xwin; see `phase2-local/README.md`. The `milestone_d_replay`
tests compare with the original HTML and run only where that file is present.

## Run the app

```sh
cd phase2-local
cargo run --locked --offline -p alm-app
```

The app starts against the built-in simulation: a seeded synthetic shop held in the app's data
folder. To use a local stack instead, start Postgres, PostgREST and GoTrue on 127.0.0.1 the way
`scripts/server_checks.sh` does (bootstrap, then every migration, then accounts whose
`app_metadata.role` is `owner` or `manager`), then open Settings → Development authority (REST)
and enter the REST URL, the sign-in URL and the local anon key. Each person then signs in with
their own account; the role comes from the authority.

## Data policy

Use synthetic data only: the seeded fixture, the test fixtures and accounts under `@alm.test`.
Do not commit real records, customer details, business identifiers, hosted project URLs, keys,
tokens or passwords. The server checks generate their secrets per run and never write them to
the repository.

## Security

Please report vulnerabilities privately; see [SECURITY.md](SECURITY.md). Do not open a public
issue for a security problem.
