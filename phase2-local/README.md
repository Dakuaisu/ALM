# Local synthetic Manager draft demo

Launch with an **existing absolute device root** and a trusted synthetic account ID:

```sh
cargo run --locked --offline --bin alm-draft-demo -- --device-root /absolute/device/root --account demo-a
```

Use the same root with another account ID to select its separate drafts. IDs are case-sensitive and accept only ASCII letters, digits, `-`, and `_` (1–64 characters). The application derives `manager-accounts/account-<lowercase-hex-id>.sqlite3` under the root, verifies the account binding on reopen, and does not discover, copy, or migrate an unscoped database. The Rust entry point is `LocalSales::open_manager_account(root, account)`, which returns a `ManagerDraftStore` with only draft save and reopen operations.

The explicit `--db /absolute/path/to/legacy.sqlite3` launch remains available for the **legacy single-account** demo and existing synthetic fixtures. `LocalSales::open_sqlite` remains the unscoped fixture constructor; it is not used by scoped launches. Do not point legacy mode at account-scoped files.

To change a draft's quantity, `open` the draft (or start one with `new`), enter `edit`, then enter the one-based ordinary line number and the new positive whole-number quantity on separate lines. Crate rows are not editable. A changed quantity marks the draft `UNSAVED`; entering the same quantity leaves its state unchanged. Use `review` to inspect the edit and `save` to persist it before `new`, `open`, or `quit`.

These are local provisional drafts, not authenticated accounts or a posting authority. Use a trusted device root; the scope checks do not defend against concurrent changes by someone who can write to that root.
