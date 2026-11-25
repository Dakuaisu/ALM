# Open items

## Consolidated owner checklist

Use this checklist for one combined owner request when needed. The numbered entries below retain the evidence, scope, and status of each item. Do not ask again for the deployed HTML identity or for routine local implementation approval. Owner replay of the demo is optional and is not on this checklist.

- [ ] **Discovery prerequisites — OPEN-001, OPEN-008:** Supply the remaining deployment details and dependencies, and provide the contained browser environment. Provisioning it is not a passing isolation probe or a completed walkthrough.
- [ ] **Protected evidence and access — OPEN-002, OPEN-006:** Separately authorize each required environment and real-record use. Specify the permitted read-only metadata or protected copies for discovery and, separately, the isolated hosted test project and accounts for integration tests. Neither permits production writes. Do not put credentials or raw records in the repository.
- [ ] **Business and permission decisions — OPEN-003, OPEN-004, OPEN-009:** Decide source conflicts and precedence with accounting; approve the production role/data matrix and stale-term authority. Require a written decision before any material departure from the HTML. In particular, do not change `BillDate` or either paste-mismatch threshold to clear an unverified case.
- [ ] **Device prerequisites and parity sign-off — OPEN-005:** Confirm targets, provide access to actual devices and approved share recipients, and resolve disputed parity results against the original. Keep installed workflows, image handoff, print and PDF subject to their existing checks.
- [ ] **Migration, accounting, and cutover — OPEN-002, OPEN-003, OPEN-007:** Authorize migration rehearsal with protected copies separately. Obtain owner and accounting sign-off on reconciliation and invoice continuity, agree the write pause and rollback authority, and obtain a distinct production cutover authorization before switching. Approval to rehearse is not approval to switch.
- [ ] **Repeated-product crate valuation — [OPEN-010](#open-010-repeated-product-crate-valuation-l10-01):** After verified OPEN-008 containment and frozen-runtime evidence, decide the L10-01 release costing and report-rounding rule with the accounting reviewer. Neither candidate is approved.
- [ ] **Unattributed working-tree files — [OPEN-011](#open-011-unattributed-working-tree-files):** Decide for each listed file whether to commit, ignore or delete it. None is used as evidence.
- [ ] **App screen departures — [OPEN-013](#open-013-app-screen-departures-from-the-html):** Approve, reject or schedule a fix for each listed departure in the ported screens.

## OPEN-001 Deployed baseline and dependencies
- Status: OWNER-BLOCKED
The owner confirmed the repository HTML is the deployed copy; its local SHA-256 matches the pinned digest ([DISC-DEPLOY](RESULTS.md#phase-1-discovery)). Preserve it unchanged. Deployment origin, browser/OS, assets and integration configuration evidence without secrets remain outstanding; the confirmation does not establish complete dependency capture (DEC-012).
## OPEN-002 Live metadata and record access
- Status: OWNER-BLOCKED
The owner must provide protected Supabase metadata or explicitly authorize least-privilege read-only discovery access, plus approved Sheets and recurring JSON samples and a way to obtain protected snapshots. Do not contact live endpoints, infer deployed schema from client code, or assign source precedence (DEC-013).
## OPEN-003 Source precedence and invoice rules
- Status: OPEN
Continue the [static record crosswalk](PHASE1_DISCOVERY.md#per-record-source-crosswalk). Actual overlap, invoice series and periods, historical GST treatment, payment reconciliation, stock and godown boundaries, and source precedence are blocked pending protected evidence under OPEN-001/002. Keep OPEN-003 open; bring discrepancies and proposed precedence to the owner and accounting reviewer before migration.
## OPEN-004 Permission and draft conflict decisions
- Status: OWNER-BLOCKED
The owner must approve the role and data matrix, provisioned device rules, and who may resolve stale price, tax, stock, or credit terms. The proposals remain unapproved under DEC-015; unapproved operations remain denied.

Screen-access questions from the app ports (recorded at `66dfcfe`; not decided). The core currently denies the Manager on each:

- Counter: the HTML shows it to everyone, with no amounts.
- Customers balances and ledger.
- Cash Book.
- Dashboard.
- Reorder: the HTML shows it to everyone, costs included.
- Stock quantities without values.
- The manual Godown list.
- Sales and Receive Payment: unchanged, still Owner-only.

Added at `bc657e5` (batch 2 and Reports ports):

- GST Sales.
- Products: the HTML shows buy rates and margins to everyone.
- Supplier names without their rates.
- Order Planner.
- Purchase Orders and receiving goods.
- Crates.
- Rate List, which shows sell rates only.
- Reports.

Added at `e23a8aa` (Settings, backup, bill image ports):

- Settings, including this device's appearance.
- Backup & Restore.
- Sharing the bill image.
- The estimate and purchase-order print views.

Added at `9411965` (device data layer):

- What a Manager's device projection may hold (currently only the product list).
- Who may check or retry a confirmation whose outcome is unknown.
- Whether that check or retry may happen after an account switch.

Added at `f777be0`:

- Manager drafts start with a 0 loading charge because Settings (and its loading default) is Owner-only in core. Should the Manager receive the loading default?

Added at `d69804a` (unknown outcomes, revocation, REST); implemented conservatively, not decided:

- On downgrade or revocation, must Manager drafts, the account's device GST tables, shop settings and full backups also be purged? (Kept now.)
- May a downgraded Owner's queued or unknown confirmations be dropped (purged now) or handed to another Owner device?
- Who may resolve an unknown confirmation after the device switches authority?
- May a Manager look up the status of its own requests? (Owner only now.)
- May a Manager see the authority's per-screen rows beyond the product catalog?

Added at `2a6ff3e` (Owner book rows, crate sales, activity log); implemented conservatively, not decided:

- Who may read the activity log (Owner only now), and should entries name the account by its login identity?
- May a Manager sell crates? (Denied.)
- May a Manager see customer mobile and address? (Manager rows hold none.)
- May a Manager change the default reorder level? (Denied.)

Added at `dce16b0` (request identity, crate bills, legacy import rehearsal); implemented conservatively, not decided:

- May a Manager add a crate row on New Bill? (Denied, as crate sales.)
- Who may run a legacy import and see its manifest and quarantine? (Owner only; the rows include buy rates and costs.)
- Should the server parse the raw legacy file itself instead of trusting the device's mapped plan? (SQL checks only the plan's shape.)
- Who resolves quarantined legacy records, and on what evidence? (Ties to OPEN-003.)
- Should a device ID be bound to a provisioned device rather than drawn by the install?
## OPEN-005 Targets and output handoff
- Status: OWNER-BLOCKED
The owner must confirm Windows and macOS laptop and Android tablet models and OS versions, and approve WhatsApp test accounts or recipients. Local image, print, PDF and simulated share are development only; they do not establish actual handoff or platform acceptance (DEC-014).
## OPEN-006 Hosted test project and access authorization
- Status: OWNER-BLOCKED
The owner must authorize the isolated hosted project and test access before any hosted integration work. D1 supplies the implementation approval for reversible local work with synthetic data; OPEN-006 does not block it. Do not use production as a substitute.
## OPEN-007 Cutover and rollback authority
- Status: OWNER-BLOCKED
The owner must confirm the production write pause, identify the accounting signer and rollback authority, and separately authorize production cutover after rehearsal and parity sign-off. The rollback proposal remains unapproved under DEC-016. Confirm how post-switch sales will be retained and reconciled before any rollback.

## OPEN-008 Contained browser for Phase 1 walkthrough
- Status: OWNER-BLOCKED
The owner or a designated administrator must provide a disposable Linux VM with a supported Playwright/Chromium pair and permission to configure guest-only networking. Preinstall tooling before transferring the candidate. During the run, use a private lab network with no production routes, internet uplink, host-network sharing, or proxy. Run the walkthrough browser and its network subprocesses inside a dedicated Linux network namespace under an OS-level IPv4/IPv6 default-deny egress policy that permits only the required loopback fixture traffic. Use a temporary profile with no accounts, secrets, or production records. No changes to the Mac's firewall or production systems are needed.

On the private lab network, place an inert non-loopback marker listener outside the walkthrough namespace. Use a separate disposable browser with the same build that can reach the marker as the positive control. Then run the existing [inert browser gate](PHASE1_DISCOVERY.md#inert-browser-gate-not-run) in the actual contained browser: record the expected loopback response and matching listener receipt, plus an OS-level deny record for the direct non-loopback attempt and no matching receipt at the external listener. Preserve evidence of the effective policy and browser process placement. Do not substitute request interception, a timeout, or an unreachable control target for enforcement evidence.

Keep the candidate unserved until the full gate passes. If the VM is unavailable or containment cannot be demonstrated, keep OPEN-008 blocked and the candidate runtime cases NOT RUN. Preserve the original DISC-ISO failure and log any new attempt separately in `docs/RESULTS.md`. After a pass, keep the same verified containment in place and run synthetic L01 first as development evidence. That run does not resolve deployed-baseline verification under OPEN-001.

## OPEN-009 PO paste calendar date discrepancy (L07-04)
- Status: OWNER-BLOCKED
The [frozen parser](../ALM_Business_Manager.html#L6405) can produce a calendar-invalid date string. The Rust fixture instead retains the prior valid draft date when the paste has no valid date ([L07-04 development result](RESULTS.md#purchase-order-paste-formats-and-mismatch-boundaries)). Legacy runtime behavior is NOT RUN and remains unverified. Any permanent departure requires written owner approval. Recorded as [DEC-033](DECISIONS.md#dec-033-defer-po-paste-invalid-calendar-date-behavior-open-009). Keep `BillDate` validation and both mismatch thresholds unchanged.

## OPEN-010 Repeated-product crate valuation (L10-01)
- Status: OWNER-BLOCKED

The [development result recorded in `d6ac8d9`](RESULTS.md#pc-003-batch-2-addendum-crate-fifo-and-dead-stock-valuation) records a parity MISS. For component demands `[3,2]` and `[3,1]`, respectively, the source-derived saved report costs are 626 and 500 paise (displayed ₹6 and ₹5); Rust reports the depleted-lot costs of 925 and 650 paise. Neither the source-derived saved cost nor the cost of depleted lots is approved for release. The action-created Rust Dead Stock checks, private seeded history, and source-derived name-only helper are separate evidence; none settles crate costing.

Under [DEC-027](DECISIONS.md#dec-027-defer-repeated-product-crate-valuation-l10-01), first pass the [OPEN-008 frozen-browser prerequisite](#open-008-contained-browser-for-phase-1-walkthrough), run L01 first, then capture the frozen L10-01 allocation, saved cost and report display. The owner must resolve the costing and rounding rule in writing with the accounting reviewer after reviewing that evidence. Independent GST and backup write-failure work can continue.

## OPEN-011 Unattributed working-tree files
- Status: OWNER-BLOCKED

These paths are untracked or unattributed in the working tree. No builder session claims them. They were not opened or read (where listed as protected), are not used as evidence, and are not committed, ignored, moved or deleted. The owner decides whether to commit, ignore or delete each one. Anything that holds deployed schema, real records or configuration needs OPEN-002 authorization before use, and raw records must stay out of Git.

| Path | Type |
| --- | --- |
| `ALM_Business_Manager.html` | Untracked deployed baseline (SHA-256 re-verified as the pinned digest, DISC-DEPLOY-RECHECK); versioning it is the owner's call |
| `alm-shop-schema.sql` | Untracked SQL file, protected, not read |
| `Supabase Snippet Untitled query.csv` | Untracked CSV export, protected, not read |
| `supabase/config.toml`, `supabase/.gitignore` | Untracked Supabase CLI configuration. The owner claimed it on 2026-11-23 and approved its use for the local stack; it was read (default local settings, no secrets) and left uncommitted. The new `supabase/migrations/` and `supabase/tests/` are committed separately |
| `docs/OWNER_REVIEW.md` | Untracked document, not read |
| `package.json`, `package-lock.json`, `node_modules/` | Untracked Node tooling files and installed packages |
| `.kiro/agents/default-eed8665b.json` | Untracked agent-tool configuration |
| `.gitignore` | Untracked root ignore file |
| `.DS_Store`, `phase2-local/.DS_Store` | Untracked macOS metadata |
| `phase2-local/crates/alm-store/tests/milestone_d_replay/outputs/.verify.DawLfe/` | Untracked replay run artifacts (SVG, PNG, HTML, text); moved here from `phase2-local/tests/` with their directory in the D2 split (`2ca90b0`), committed by mistake in `7fbba84` and untracked again; the files stay in that commit's history |
| `phase2-local/crates/alm-store/tests/milestone_d_replay/recovery/harness.rs` | Untracked Rust test source, not compiled by any target; same move, mistaken commit and untracking as the row above |

## OPEN-012 HSN code assignment is weak in the legacy HTML
- Status: DEFERRED BY OWNER

Owner note: the HTML's HSN handling is known to be flimsy. The owner chose not to fix it in the HTML and will schedule a proper fix in the Rust core later. Until then, Rust mirrors the HTML's HSN behavior for parity. Do not redesign HSN handling, add HSN validation or change default HSN or rate mappings without a written owner decision. Known weak points in [`gstInfo`](../ALM_Business_Manager.html#L4872):

- HSN and rate come from case-insensitive substring keyword matching on the item name, checked in a fixed order. Short keywords (`sure`, `real`, `hell`, `slice`, `fc `, `ind-`, `milk`, `water`) can match unrelated product names. Anything that matches no keyword falls back to `22029090` at 40%.
- Plain soda (18%) and packaged water (5%) share HSN `22011010`.
- The product-level `hsn`, `gstRate` and `cess` overrides are found by exact item name (`prodByItem`), not product ID, so renaming a product can lose its override.
- On standalone GST bills, the line HSN is free text with no format check (4, 6 or 8 digits), and the invoice falls back to the keyword guess when it is blank.

## OPEN-013 App screen departures from the HTML
- Status: OWNER-BLOCKED

Recorded at `66dfcfe` from the batch 1 and partial batch 2 screen ports in `phase2-local/crates/alm-app`. None is approved; each needs a written owner decision to keep, or a fix to match the HTML. Every unported sub-action is listed per screen in the commits and in [RESULTS.md](RESULTS.md#review-fixes-batch-1-book-screens-batch-1-integrated-check-and-partial-batch-2).

- Modals (customer ledger, Cash Book entry, stock lots) are shown inline on the page.
- Dates show as ISO (2026-11-23), not the HTML's display format.
- The Cash Book difference updates when you press Check, not as you type.
- Customers "Only Due" uses the Counter's rule (balance due after rounding to whole rupees), not the HTML's raw balance > 0.
- Negative opening balances (advances) can't be entered, because the parser takes only non-negative amounts.
- Reorder Copy List shows the text in a box to copy by hand instead of writing to the clipboard.
- Dashboard deltas, charts, percentages and top products/customers are not ported.

Added at `bc657e5`:

- Editing a posted bill adds a line; the core can't change an existing line's quantity.
- On Products, a typed sell rate wins over the HTML's live "buy keeps margin" recalculation.
- Reports shows Misc and Salary as separate columns instead of a summed "Other Expenses".
- Autocompletes are replaced by native pickers or a "Fill from items" button.
- Rate List print/PDF and opening WhatsApp are not ported; the message text is shown instead.
- Purchase Orders "Close remaining" is covered by a Rust test only; it was not pressed in the macOS smoke.

Status at `e23a8aa`: closed (now match the HTML): posted bill edit in place, Reports "Other Expenses" column, Products "buy keeps margin", display dates, modals, live Cash Book difference, autocompletes, Reorder/Rate List/godown clipboard, Rate List print/PDF and WhatsApp. Still open or added:

- The bill image follows the HTML's canvas layout; the full-invoice HTML-to-PNG path isn't ported.
- Pack, MRP savings and customer contact are blank on the bill image; bold/italic are imitated from the one bundled font.
- Restore opens the backup read-only; it doesn't replace today's in-memory billing.
- Settings has only GSTIN, FSSAI, the show-on-bill switches and the reorder default.
- On the Android tablet, New Bill item suggestions open under the keyboard until it is dismissed.
- Carried over: "Only Due" rule, negative opening balances, Dashboard charts and deltas, GST customer autocomplete, "Close remaining" never pressed in a smoke.

Status at `f777be0`: closed since `e23a8aa`: Only Due rule, negative opening balances, full Settings, bill image pack/MRP/contact with real bold font, Dashboard charts and deltas, GST customer autocomplete, Android keyboard suggestions, "Close remaining" pressed on Android, default loading charge on New Bill. Added:

- Deleting a product that appears on a posted bill is refused; the HTML deletes it anyway.
- The server SQL rules are simplified and checked only by the conformance suite, not against the core's fixtures.

Status at `d69804a`: SQL now matches the core in a differential suite (16 differences fixed); crate sales are not yet an authority command and not compared. Still open: Owner screens outside the projected ones read the simulation's internal state, so with the REST switch they are empty and posting through REST from the app isn't possible yet; Customers list without mobile/address; crate picker lists directory products only; stock count form sits in the lots modal; hosted sync, activity log and Clear All are inert.

Status at `2a6ff3e`: closed: Owner screens read server rows (REST works end to end), crate sales through the authority, customer mobile/address, crate picker, stock Adjust dialog, reorder default through the authority, activity log. Still open:

- Crate sales are made from a Sell form on Crates, not as a crate row on New Bill.
- With a crate bill, Sales shows the "unsupported" notice, and crate bills can't be edited, cancelled or imaged.
- Activity times show in UTC (`dd-Mon-yyyy hh:mm`); the HTML uses device-local en-IN time.
- The HTML's restore, clear and sync log entries are not recorded.
- Manager New Bill drafting still reads the simulation's catalog, also with REST.
- Bill image canvas layout and slanted italic; Latest-balance-only archive regeneration; product delete refused on posted bills; Clear All and hosted sync / Google Sheets / folder cards inert.
- Status at `dce16b0`: closed: crate bills listed, viewed, edited, cancelled and restored in Sales; crate row on New Bill; activity times in local en-IN; restore log entry; Manager New Bill from its pulled catalog. Still open, added: a Manager's customer picker still comes from the device directory (Manager rows hold no customers); a legacy file must be placed in the app's `legacy-import` folder (no file picker).
- (Found at `fe5db4e`) A crate bill can hold only its crate. The HTML's New Bill mixes crate rows and ordinary lines on one bill (`draft.items` rendered as `crateRow` or `lineRow`, HTML L3640).
