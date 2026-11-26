# Local synthetic Manager draft demo

Launch with an **existing absolute device root** and a trusted synthetic account ID:

```sh
cargo run --locked --offline --bin alm-draft-demo -- --device-root /absolute/device/root --account demo-a
```

Use the same root with another account ID to select its separate drafts. IDs are case-sensitive and accept only ASCII letters, digits, `-`, and `_` (1–64 characters). The application derives `manager-accounts/account-<lowercase-hex-id>.sqlite3` under the root, verifies the account binding on reopen, and does not discover, copy, or migrate an unscoped database. The Rust entry point is `LocalSales::open_manager_account(root, account)`, which returns a `ManagerDraftStore` with only draft save and reopen operations.

The explicit `--db /absolute/path/to/legacy.sqlite3` launch remains available for the **legacy single-account** demo and existing synthetic fixtures. `LocalSales::open_sqlite` remains the unscoped fixture constructor; it is not used by scoped launches. Do not point legacy mode at account-scoped files.

To change a draft's quantity, `open` the draft (or start one with `new`), enter `edit`, then enter the one-based ordinary line number and the new positive whole-number quantity on separate lines. Crate rows are not editable. A changed quantity marks the draft `UNSAVED`; entering the same quantity leaves its state unchanged. Use `review` to inspect the edit and `save` to persist it before `new`, `open`, or `quit`.

These are local provisional drafts, not authenticated accounts or a posting authority. Use a trusted device root; the scope checks do not defend against concurrent changes by someone who can write to that root.

## Local CI on macOS

Run `./scripts/local_ci.sh`. It stops at the first failure and reports elapsed seconds per stage and overall. In order it checks formatting, workspace clippy and tests, the disposable server stack, the macOS app and disk image, the arm64 Android release APK, then the Windows MSVC release binary and target clippy. The cross-build does not produce or exercise the Windows installer.

Use a populated Cargo cache, the frozen HTML for the replay tests, Docker, the Tauri CLI, JDK 17, the Android SDK and NDK, and the Android and Windows Rust targets used by CI. The Windows stage needs cargo-xwin and its compiler, linker and SDK. Set `ANDROID_HOME`, `NDK_HOME` and `JAVA_HOME` for nonstandard installations. Only the local workspace test stage omits elapsed-time assertions when the Mac's one-minute load exceeds 16; it prints the measured load and still runs the tests.
