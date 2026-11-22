# ALM Business Manager: PRD
## Goals
- Preserve the original ERP's look, layout, workflows, calculations, and outputs on Windows, macOS, and Android. The Android tablet app runs the full ERP, subject to the same account permissions as desktop.
- Support multiple staff in the existing single business, with hosted Supabase as the shared authority and local SQLite on each device.
- Carry forward existing records and invoice continuity, with evidence that the new system agrees with the legacy records before it takes over.

## Non-goals
- A redesign, an MVP, a companion app, a wrapper that retains JavaScript business logic, a shop server, or a new multi-entity model.
- Offline final sales, payments, stock reservations, or goods release; automatic finalization on reconnect.
- Unapproved changes to business rules, outputs, feature scope, source-of-record rules, or production data.

## Requirements
- R-1: Produce a traceable inventory of the entire deployed HTML and its dependencies. Every screen, action, setting, secondary workflow, calculation, integration, and output must have a source reference, expected behavior, and parity case; record unreachable or ambiguous behavior rather than omitting it.
- R-2: Ship installable Windows, macOS, and Android builds with a shared Rust business core. Preserve all inventoried screens and actions, including billing, estimates, GST, bill edits and cancellations, purchases and partial receipts, stock and FIFO costing, crates, godown, customer balances and udhaar, cash book, reports, exports, settings, and backup and restore. Any presentation-only reuse must not retain legacy JavaScript business logic.
- R-3: Preserve the existing bill image layout and content, and verify that the bill is handed off to WhatsApp as an image on each target platform. Preserve and separately verify existing print and PDF options, without requiring a printer for normal billing.
- R-4: Use the existing hosted Supabase project for production shared billing, with no shop server and a local SQLite database on each device. Verify that authorized online changes made on one device become visible on another and that interrupted or resumed sync does not duplicate or overwrite authoritative transactions.
- R-5: Give each owner, manager, and cashier an individual account. Enforce an owner-approved action and data permission matrix in Supabase; deny unapproved access by default. Staff devices must never receive owner-only costs, margins, or reports, including through nested fields, derived outputs, sync, exports, or backups. Verify allowed and denied cases at the service boundary and inspect device data, not just the UI.
- R-6: Allow staff to save, reopen, and edit clearly marked provisional orders in local SQLite across restart. A provisional order cannot collect or record payment, release goods, reserve or decrement stock, or issue a final invoice. Reconnect does not post it. Changed terms must be shown before an authorized user explicitly confirms online; stale terms must not be silently substituted.
- R-7: On explicit authorized online confirmation, Supabase validates current terms and atomically posts the sale, assigns its final invoice number under the verified legacy numbering rules, and records its related stock/FIFO, payment or udhaar, and ledger effects. A retry after a lost response returns the original outcome without posting again; conflicting reuse of a request identity is rejected. Concurrent confirmations cannot issue duplicate invoices, double-post a draft, or allocate the same available stock twice.
- R-8: Capture protected, attributable snapshots of Supabase, Google Sheets, and recurring JSON exports. Establish source precedence by record type from evidence and owner approval, retain provenance, and preserve history and identifiers without silently discarding or duplicating records. Reconcile record-level history and control totals for invoice numbering and status, GST, payments, balances, cash book, stock by item and location, and FIFO lots and valuation, including historical edits and cancellations.
- R-9: Verify every inventory case against the original on approved Windows, macOS, and Android targets, with expected versus actual evidence, role where applicable, and an explicit result for each platform. No untested case or unresolved parity failure passes the release gate.
- R-10: Require separate, recorded owner approvals for implementation and production cutover. Rehearse a migration and rollback, keep only one production billing path writable at a time, and preserve and reconcile new transactions before any rollback that follows new-system writes.

### Local delivery alongside discovery

The owner's D1 decision in `docs/OWNER_DELIVERY_REPLAN.md` is the recorded R-10 implementation approval for reversible local work using synthetic data. It extends the earlier slice-specific approvals without changing their historical results. The builder may implement, test, fix regressions, and sequence local milestones without a separate request for each case, milestone, demo replay, or feedback file. The owner may try any demo; absence of feedback is neither a blocker nor acceptance. A blocked dependency stops only work that needs it. These milestones are not phases and cannot satisfy a phase exit by themselves.

**Starting checkpoint.** Existing milestone A–D replays and later scoped results in `docs/RESULTS.md` are inputs, not a backlog to repeat. In particular, preserve the live and recovered estimate `Latest` checks at `250e083`, including `milestone_d_structured/signed_recovery.rs`, the signed half-rupee cases, the sum-before-rounding control, and the fresh-store regeneration from typed records. Reuse that implementation and its tests. Do not add a second rounding rule, rewrite earlier limitations, or report old observations as new runs.

| Delivery batch | Existing code and connected work to finish | Runnable integrated check and handoff |
|---|---|---|
| 1. Billing, receipts and books | Reuse `alm-draft-demo`, `owner_simulation`, `milestone_c_replay`, the L14 customer ledger/receipt replays, `l15_cashbook`, and the structured estimate archive. Connect the remaining steps so one named customer's provisional draft, simulated Owner transaction, estimate/GST output, bill payment, separately dated receipt, sales history, udhaar balance and Cash Book close all refer to the same records. Carry edits and cancellation through the affected views. | Provide one offline replay. Save and reopen the Manager draft without posting; in the separate Owner simulation require explicit review of changed terms, then exercise lost-response retry and conflicting request reuse. Follow the bill and independent receipt across a date boundary, edit and cancel the bill, and reconcile the effects by ID and date. Check exact paise separately from displayed amounts, confirm the standalone receipt survives cancellation, and verify historical bill values versus the estimate's `Latest` balance. Use the existing rounding regression, not a new oracle. Hand off one transaction timeline with expected and observed values and role denials. |
| 2. Supplier order through stock, dispatch and valuation | Reuse `milestone_b_replay`, the supplier-price and order-planner fixtures, `common/partial_po`, and the catalog, receipt-lot, crate and godown code. Connect remaining catalog or supplier changes, plan selection, PO creation, dated partial receipts and closure, crate component dispatch, stock adjustments, reorder and stock reports around the same product and lot IDs. This batch can start independently with synthetic opening stock. | Replay a quote change through a plan and PO; verify neither creates stock. Receive at differing dates/rates, close the unreceived remainder, then dispatch a crate in the Owner simulation and apply a count adjustment. Reconcile PO outstanding quantities, component movements, FIFO value, reorder visibility and report cutoffs against fixed expectations. Keep manual godown counts separate from derived movements and keep costs out of Manager data. Hand off one PO-to-lot movement and valuation reconciliation. OPEN-009 blocks a disputed paste-date decision, not valid-date purchasing or stock work. |
| 3. Documents, settings, export and recovery | Reuse `milestone_d_replay`, `milestone_d_structured`, the rate-list, preferences, folder-backup and clear/restore replays, and their inert sinks. Use representative sales and purchasing records from the first two batches or their existing synthetic fixtures. Connect remaining bill/estimate/GST, PO and rate-list outputs, settings and privacy, structured exports, backup, restore, and local-only sync/recovery cases. | Produce outputs, change an applicable setting, back up, drop the source handle, restore to a fresh path and regenerate from reopened records without reseeding or cached output. Compare regenerated contents to fixed expectations as well as to the originals. Check which settings are device-local, denied staff exports on the reopened store, and rejection of invalid restore without data loss. Exercise share, print, PDF, clipboard and integration substitutes only within their documented local scope. Hand off one artifact manifest distinguishing actual generated files, sink receipts, and unrun device or service actions. |

**One handoff and one consolidated Supervisor review per batch.** Start with the existing case matrix: link already-demonstrated behavior to its original evidence, implement the missing connections, and keep focused regressions and coherent commits within the batch. Map every remaining inventory action to its batch or a named external prerequisite; a missing fixture is not grounds to drop an action. Submit the runnable command, tested commits, run IDs, before/after controls, and case-by-case dispositions together in `docs/RESULTS.md`. Run the focused tests and locked offline regression suite. Report failures, ignored tests, blocked cases and cases not run individually; a batch verdict cannot convert any of them to a pass. Do not schedule separate planning, review, or evidence-formatting turns for each case.

Continue independent batch work and static discovery when another path is blocked. Keep the frozen HTML unserved until the isolation gate passes. Keep Manager provisional-only, Owner posting a local simulation, and all synthetic output labeled development. A replay is not hosted authorization, deployed-browser observation, real-device handoff, accounting sign-off, or phase completion.

The consolidated owner checklist is in `docs/OPEN.md`. A supplied prerequisite permits the named check only; it is not authorization for another environment, use of real data, or acceptance. The formal phase exits below still apply in full.

## Phase 1 - Discovery and baseline
Scope: R-1, R-2, R-3, R-4, R-5, R-6, R-7, R-8, R-9, R-10
Exit criteria:
- The deployed laptop baseline and dependencies are identified and versioned by digest. A source walk and isolated legacy walkthrough account for the entire HTML, with a parity matrix covering screens, settings, secondary actions, calculations, outputs, and integration behavior. Record source and observed evidence separately.
- A read-only schema and configuration capture identifies relevant live tables, constraints, views, functions, triggers, grants, policies, and integrations. Compare it with the HTML's observed data access and document gaps; do not infer the deployed schema from the HTML alone.
- The source map identifies how Supabase, Sheets, and JSON are read or written, where records overlap, and how protected snapshots, conflicts, and reconciliation will be handled. No unverified source is declared authoritative.
- Discovery delivers the proposed Rust and Supabase split, permission matrix, offline confirmation rules, device and sharing matrix, and migration and rollback plan. Resolve implementation-critical decisions with the owner; record unavailable evidence as blocked, not passed. Local implementation may proceed under D1 while discovery remains open. Do not declare discovery complete without its evidence or begin gated work without its separate authorization.
Owner-only:
- See the consolidated checklist in `docs/OPEN.md` for outstanding baseline evidence, protected data and access, disputed rules, permissions, and device prerequisites. The deployed copy is confirmed; D1 authorizes routine local implementation only.

## Phase 2 - Shared data, authorization, and sales posting
Scope: R-4, R-5, R-6, R-7
Exit criteria:
- After implementation approval, run hosted integration tests in a separate Supabase test project. Exercise individual accounts, allowed and denied operations, approved data projections, cross-device sync, account switching, and staff payloads and caches. Restricted data must not reach staff devices.
- Exercise offline draft persistence through a restart, prohibited offline actions, reconnect without posting, stale terms and explicit confirmation, simultaneous confirmations, lost responses, identical retry and conflicting request reuse. Verify database effects after success and rejection, including that rejected or failed operations do not leave partial postings.
Owner-only:
- Provide or authorize an isolated hosted test project and test accounts; approve any required account or device handling decisions before those paths are enabled.

## Phase 3 - Full ERP port and platform parity
Scope: R-1, R-2, R-3, R-4, R-5, R-6, R-7, R-9
Exit criteria:
- Install and exercise the full release candidate on all approved platform targets, including Android tablet touch workflows. Every inventory case has passing evidence against the frozen original or passes against an explicitly owner-approved departure recorded in the contract because of the new role, offline, or shared-posting rules.
- Compare calculation and output fixtures against the original, including bill images, GST, stock/FIFO, reports, and backup/restore. Verify actual WhatsApp image handoff and separately verify print and PDF on each platform. Rerun shared sales and data isolation checks through the platform apps.
Owner-only:
- Provide access to approved laptops and tablets and WhatsApp test recipients or accounts; judge any disputed visual or workflow difference. Do not accept an untested platform as equivalent.

## Phase 4 - Migration and rollback rehearsal
Scope: R-8, R-10
Exit criteria:
- Rehearse with approved protected copies, retaining raw snapshots, an import manifest, record provenance, and a conflict log. Rerunning the import does not create duplicates. Reconciliation shows no unexplained differences or missing in-scope records and verifies the next invoice number without issuing a live one.
- Rehearse pausing legacy writes, taking a final snapshot, restoring the pre-switch state, and preserving and reconciling sales posted after the switch before an old billing path could reopen. Record the decision points, accountable people, and evidence required to proceed or roll back.
Owner-only:
- Authorize rehearsal data and access, settle record conflicts with the person responsible for accounts, and agree the production write pause and rollback authority.

## Phase 5 - Production cutover
Scope: R-4, R-5, R-7, R-8, R-9, R-10
Exit criteria:
- Separate production cutover approval is recorded after rehearsal and parity sign-off. Pause all legacy write paths, capture the final protected snapshots, and reconcile them with the migrated records before opening the new billing path for writes.
- Obtain owner and accounting sign-off on reconciliation and invoice continuity. Verify production access and shared billing using the approved smoke procedure and reconcile any actual posting it makes. If a gate fails, stop and follow the approved rollback procedure; do not reopen legacy billing against a stale snapshot.
Owner-only:
- Schedule and authorize the production change, coordinate the pause and reopening, sign off with the person responsible for accounts, and authorize rollback if needed.

## Results
A real result is an observed run against a named acceptance case using the frozen baseline, identified build, approved target device or environment, approved data, and criteria fixed before the run. Acceptance and rehearsal evidence may be real results for their own gates, but neither is a production outcome. Label synthetic exercises, mocks, exploratory runs, and non-acceptance builds as development. Every reported number gets a row in `docs/RESULTS.md` with its phase, requirement or case, environment, data and build references, expected and observed outcome, evidence location, and development or real label. Record failed, blocked, and not-run cases distinctly. Never fabricate a number or move a threshold to make a run pass.
