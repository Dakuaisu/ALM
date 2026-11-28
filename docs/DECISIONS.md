# Decisions
## DEC-001 Single deployed baseline
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The owner uses one laptop and one deployed version today; the port is for expansion to more staff.
Options: Verify and freeze the single deployed copy, or assume multiple counter variants.
Decision: Confirm the laptop copy matches the supplied HTML and freeze it with its dependencies.
Why: The deployed version defines parity without inventing variants.

## DEC-002 Existing record sources
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The owner reports that records are in Supabase and Google Sheets, and that JSON is exported regularly.
Options: Pick one source now, or compare all sources by record type.
Decision: Inventory and snapshot all three; establish precedence and resolve conflicts from evidence before migration. Verify whether Sheets is a live integration, a copy, or both.
Why: An export or hosted table cannot be assumed to include the complete history.

## DEC-003 Recover the live schema
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The owner has Supabase credentials but no longer has the SQL schema.
Options: Recover deployed metadata through authorized read-only discovery, or reconstruct the schema from the HTML.
Decision: Recover deployed metadata without changing production; use HTML observations to cross-check it.
Why: Client code does not establish deployed constraints, policies, functions, or triggers.

## DEC-004 Single business
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The owner says this is a single entity with no formal counters.
Options: Introduce a counter or multi-entity model, or serve one business across devices.
Decision: Serve one business across devices. Preserve verified GST, invoice series, stock, and location boundaries rather than deriving them from device count.
Why: More staff does not imply new accounting or inventory boundaries.

## DEC-005 Laptops and tablets
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The owner uses laptops and tablets for billing, not dedicated counter hardware.
Options: Target dedicated terminals, or verify the required apps on representative laptops and Android tablets.
Decision: Target Windows and macOS laptops and Android tablets with full ERP parity. Confirm actual devices, OS versions, and WhatsApp handoff in discovery.
Why: Packaging, touch behavior, and sharing must match the equipment staff will use.

## DEC-006 Rust core and shared authority
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The port must not retain JavaScript business logic or require a shop server.
Options: Wrap the legacy app, add a shop server, or share a Rust core with hosted Supabase authority and local SQLite.
Decision: Use a shared Rust domain core, Supabase for authoritative shared operations, and SQLite per device. Select the presentation stack after discovery demonstrates a route to full parity on all targets.
Why: It preserves the ERP while keeping shared transactional rules out of JavaScript and device caches.

## DEC-007 Account permissions
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: Individual accounts are required; exact manager and cashier permissions were not supplied.
Options: Copy legacy access or enforce an explicit matrix.
Decision: Enforce least privilege in Supabase, default deny for unapproved actions, and no owner-only data delivery to staff devices. The detailed matrix awaits owner approval.
Why: UI hiding cannot protect data already sent to a device.

## DEC-008 Draft and confirmation
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: Offline staff may draft orders but may not transact; the owner has not specified who handles changed terms.
Options: Replay on reconnect or retain drafts for authorized online review.
Decision: Use durable SQLite drafts, show changed terms and require fresh explicit confirmation online, then post atomically and idempotently. Who may resolve which conflicts awaits owner approval.
Why: Reconnect and retries must not cause unauthorized, duplicate, or stale sales.

## DEC-009 Isolated implementation and acceptance
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: The existing hosted project is required in production, but test access was not confirmed.
Options: Test on production or use a separate hosted project.
Decision: Require a separate hosted test project for implementation and rehearsals; production discovery remains read-only.
Why: Testing permission rules, failure handling, and concurrent posting must not affect live billing.

## DEC-010 Separate migration and cutover gates
- Decided by: planner
- Status: APPROVED by the owner 2026-11-19

Context: Only discovery is approved. The owner has not confirmed downtime or rollback arrangements.
Options: Keep both systems writable or use a reconciled switch with one writable path.
Decision: Require separate implementation and production approvals, rehearse migration and rollback, and hold the switch until the write pause and sign-off are arranged.
Why: A controlled handover prevents split invoice history, duplicates, and loss of post-switch sales.

## DEC-011 Plan approved by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge approve)

Recorded by agent-bridge at 2026-11-19T01:56:05+05:30.

- docs/PRD.md: sha256 7b7bdb41a005321019d2c5098f673f5c6b30235ab2dc15a1206065edeec9ccca
- CLAUDE.md: sha256 407bbc6e4b6d788c713f6ea4f7ef693e659a36687e0750d88fcc36411af0acb1
- bridge.toml: sha256 3886f7e9ba67ffba1fa0e6ba8eb2b5f572281c8fe8c04187d4a98274b04c5803

## DEC-012 Deployed baseline and dependencies (OPEN-001)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options: Treat the supplied HTML as deployed, or keep it as an unchanged candidate pending the laptop copy or an independently computed digest and deployment details.
Decision: Keep deployed-copy confirmation and dependency capture OWNER-BLOCKED. Use the candidate only for static discovery and a verified isolated walkthrough.
Why: The repository digest fingerprints only the supplied bytes. It cannot attest to the laptop file, origin, dependency versions, or runtime configuration.

## DEC-013 Read-only data access (OPEN-002)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options: Treat client calls as the deployed schema or probe configured services, or continue static mapping and request protected metadata or authorized least-privilege read-only access plus approved Sheets and recurring JSON samples.
Decision: Keep discovery access OWNER-BLOCKED. Continue the source map and checklist without contacting any live endpoint or assigning source precedence.
Why: Client code cannot establish deployed schema, grants, RPC behavior, retention, or overlaps. Synthetic payloads cannot replace authorized source evidence.

## DEC-014 Platform and sharing evidence (OPEN-005)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options: Substitute local previews, simulated sharing or emulator output for acceptance, or keep actual handoff blocked pending confirmed devices and approved test accounts or recipients.
Decision: Keep platform and actual WhatsApp handoff OWNER-BLOCKED. Prepare the matrix; local image, print, PDF and simulated share checks are development evidence only.
Why: Neither a local image nor an intercepted share call shows that WhatsApp receives the image on a supported target. The blank owner review provides no approval.

## DEC-015 Defer permission and offline conflict approval (OPEN-004)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options: Treat the draft role, device and conflict rules as approved, or retain them as proposals and deny unapproved actions pending explicit owner approval.
Decision: Defer approval. The proposed role/data matrix, device rules and price, tax, stock and credit conflict authority grant no access or override. Implementation still requires separate explicit owner approval (OPEN-006).
Why: The proposals describe defaults and alternatives, not the owner's delegation of access or authority over stale terms. Assuming approval could expose restricted data or let someone confirm terms they are not authorized to accept.

## DEC-016 Defer rollback authority and cutover approval (OPEN-007)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options: Treat the rollback proposal as delegated authority to pause, recover or reopen billing, or retain it as a proposal pending named authority, rehearsal and explicit owner decisions.
Decision: Defer approval. Keep rollback authority, write pause, accounting sign-off and post-switch reconciliation unapproved. Implementation and production cutover require separate explicit owner approvals; neither this deferral nor a rehearsal grants either.
Why: A documented procedure does not identify who can order a switch or prove that committed post-switch records can be retained before reopening legacy billing.

## DEC-017 PC-001 Provision a contained browser for the Phase 1 walkthrough
- Decided by: planner (re-plan requested by the supervisor at exchange 123)
- Status: APPLIED (non-material)
- Change: PC-001 (.bridge/plan/changes/PC-001.diff)

Reason: docs/RESULTS.md records that Docker's local daemon was unavailable, the macOS sandbox smoke check did not demonstrate browser network confinement, and the Playwright MCP session showed only about:blank. No qualifying probe or application walkthrough ran. docs/PHASE1_DISCOVERY.md already defines the required inert gate. The missing prerequisite is an environment that can enforce and demonstrate that gate, not a different acceptance criterion.

Context: The current macOS run has no verified containment for the browser that would execute the legacy HTML. Options: retry the unchanged setup, rely on browser-context isolation, or provision a disposable Linux guest with OS-level egress controls and a controlled marker listener. Decision: record OPEN-008 and use the Linux guest route once provisioned; keep the existing inert gate and PRD exit criteria unchanged. Why: this provides a way to verify containment in the actual walkthrough browser without contacting live services or treating source findings as runtime evidence.

## DEC-018 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: supervisor: opencode openai/gpt-6.2-sol -> opencode openai/gpt-6.2-astra.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 7b7bdb41a005321019d2c5098f673f5c6b30235ab2dc15a1206065edeec9ccca
- CLAUDE.md: sha256 407bbc6e4b6d788c713f6ea4f7ef693e659a36687e0750d88fcc36411af0acb1
- bridge.toml: sha256 42a70f0acb70c91f5a20010723e7bd5a6f38f389bb10fa56b90ad1543742f13c

## DEC-019 Local synthetic Phase 2 slice
- Decided by: owner
- Status: APPROVED (local synthetic implementation only)

The owner authorized a small Rust slice for provisional sales drafts and Owner/Manager access. Owner can use all operations in the slice; Manager must not receive total profit. This does not close Phase 1 checks, approve the full production permission matrix, authorize hosted or production access, or authorize Phase 3.

## DEC-020 Contract re-approved by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge approve)

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 7b7bdb41a005321019d2c5098f673f5c6b30235ab2dc15a1206065edeec9ccca
- CLAUDE.md: sha256 407bbc6e4b6d788c713f6ea4f7ef693e659a36687e0750d88fcc36411af0acb1
- bridge.toml: sha256 fb25e99c09fc1485b0ea0ec2927cf21c6ddb0b05c8882ca54800e8d65b862f04

## DEC-021 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: planner: opencode openai/gpt-6.2-sol -> opencode openai/gpt-6.2-astra.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 7b7bdb41a005321019d2c5098f673f5c6b30235ab2dc15a1206065edeec9ccca
- CLAUDE.md: sha256 407bbc6e4b6d788c713f6ea4f7ef693e659a36687e0750d88fcc36411af0acb1
- bridge.toml: sha256 b5008eafdde5cb2c0da40e9b3198c418938f0061043282b8bedcb8a6de802d70

## DEC-022 PC-002 Authorize parallel local development and deliver the next billing workflow as a runnable batch
- Decided by: planner (the owner's decisions in docs/OWNER_DELIVERY_REPLAN.md)
- Status: OWNER DECISION (docs/OWNER_DELIVERY_REPLAN.md)
- Change: PC-002 (.bridge/plan/changes/PC-002.diff)

Reason: docs/RESULTS.md records DEV-DRAFT-DEMO at 3292b20: the synthetic Manager can save a provisional draft, quit, reopen it in a second process, and review it without posting. It does not record owner acceptance. The Phase 1 stop in docs/PRD.md, the corresponding rule in CLAUDE.md, and the wording of OPEN-006 would still stop the builder before extending that workflow. D1–D3 in docs/OWNER_DELIVERY_REPLAN.md grant that local authority now while reserving access, business-rule decisions, and acceptance for the owner.

Context: DEC-019 and the recorded demo result document earlier narrow local approvals. The owner's `docs/OWNER_DELIVERY_REPLAN.md` now expands local delivery authority. Preserve those historical entries and results. Options: continue requesting approval per slice; remove all gates; or preauthorize reversible local work and retain access and acceptance gates. Owner decisions: D1 selects standing local authorization, optional demo feedback, and continued independent work. D2 selects integrated runnable milestones, beginning with the existing draft-to-bill workflow, with case-level evidence and milestone-level review. D3 retains separate access, real-data, material-departure, migration and cutover decisions, plus actual device/parity and accounting sign-offs. Why: the owner can try useful workflows sooner without confusing a synthetic demonstration with a verified release. No owner replay, acceptance, phase completion, or new access is recorded.

## DEC-023 Use the standard Rust JSON serializer for local recovery
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Pin `serde` with `derive` to `1.0.229` and `serde_json` to `1.0.154`, the versions already tested in `phase2-local/Cargo.lock`. Retain the locked offline build. Serde derives decode the versioned archive into typed Rust records; the JSON library handles grammar, escaping, and malformed input without a parallel hand-written parser to maintain. This decision covers the local JSON workflow only; it grants no hosted access and does not approve a legacy migration format or change posting authority.

## DEC-024 Contract re-approved by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge approve)

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 c282440c309b210614d485bfb9ee238ab4e77b9b92dff988083b7c3a7581178b
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 41d6a4d1c5c5142172ca08ad8d17468331523f89a73846b0e87a714f1cbb16e8

## DEC-025 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: planner: opencode openai/gpt-6.2-sol -> opencode openai/gpt-6.2-astra; builder: opencode openai/gpt-6.2-sol -> opencode openai/gpt-6.2-astra.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 c282440c309b210614d485bfb9ee238ab4e77b9b92dff988083b7c3a7581178b
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 e2a53f33d79e120f6676d6db11e8df95d6fb7d24407149bc4070d47850625b4b

## DEC-026 PC-003 Deliver remaining local work in three connected workflow batches
- Decided by: planner (the owner's decisions in docs/OWNER_BATCH_WORKFLOWS.md)
- Status: OWNER DECISION (docs/OWNER_BATCH_WORKFLOWS.md)
- Change: PC-003 (.bridge/plan/changes/PC-003.diff)

Reason: The local delivery table in docs/PRD.md still starts by assigning the draft preview and separates billing from its accounting effects. The repository is past that checkpoint: the draft demo, Owner simulation, purchasing replay, account replay, and structured recovery replay already exist. docs/RESULTS.md also records archive-backed estimate rounding at 250e083: the restored estimate was regenerated from typed records and checked against independent expected values. The next useful deliverable is to make related workflows runnable against the same records, not to rebuild those controls or review another isolated rounding case.

Context: The former A–D milestones produced reusable code and scoped development evidence, including archive-backed rounding. Options: continue case-by-case handoffs, restart the earlier milestones, or batch the remaining work by shared transaction, inventory, and recovered records. D1 — OWNER DECISION: use the three dependency-based batches in the PRD, with focused regressions and coherent commits inside each batch and one consolidated evidence handoff and Supervisor review at its boundary. Preserve completed results and use them as controls. D2 — OWNER DECISION: change cadence only. Keep permissions, local synthetic authorization, case traceability, phase exits, and all owner gates. No changes to models, effort, or Bridge configuration.

## DEC-027 Defer repeated-product crate valuation (L10-01)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Decision: Defer the release valuation and report-rounding rule to [OPEN-010](OPEN.md#open-010-repeated-product-crate-valuation-l10-01). Neither reproducing the source-derived saved cost nor reporting the cost of depleted lots is approved. Preserve the parity MISS and both expectations; do not change the baseline or acceptance criteria to clear the case.

Reason: The [development result recorded in `d6ac8d9`](RESULTS.md#pc-003-batch-2-addendum-crate-fifo-and-dead-stock-valuation) compared `[3,2]` and `[3,1]`: source-derived saved report costs 626/500 paise (display ₹6/₹5), versus Rust depleted-lot/report costs 925/650 paise. That difference involves allocation and rounding; the source calculation is not a frozen-runtime observation. Dead Stock action-created Rust results, private seeded-history checks, and source-derived name-only helper results remain distinct and do not validate either crate cost rule.

Resolution: After [OPEN-008 verified containment](OPEN.md#open-008-contained-browser-for-phase-1-walkthrough), run L01 first and collect frozen L10-01 runtime evidence. The owner, with the accounting reviewer, must then record the release rule for allocation and report rounding in writing, including approval for any material departure. This deferral is not owner acceptance and does not block independent GST or backup write-failure work.

## DEC-028 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: supervisor: effort max -> xhigh; builder: opencode openai/gpt-6.2-astra -> opencode openai/gpt-6.2-sol, effort max -> xhigh.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 721457621301cd3994f4409ea03788b3f82a4de889f0442a070268c65547121a
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 eea693e1c694dbbd0e3e93a7a57afaf365ebae990741c42ff4f7efdb9b3ecf40

## DEC-029 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: builder: opencode openai/gpt-6.2-sol -> opencode claude/sonnet-5.5, effort xhigh -> default.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 721457621301cd3994f4409ea03788b3f82a4de889f0442a070268c65547121a
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 36e94f19969862cfac0a965926ccc20c1c1c195285534713b543cd5d10a2ee0e

## DEC-030 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: builder: opencode claude/sonnet-5.5 -> opencode Anthropic/sonnet-5.5.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 721457621301cd3994f4409ea03788b3f82a4de889f0442a070268c65547121a
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 7a3f87d146a557cab5c3a2fdf5666ae246e11c32ee9bab3dd214fb2695170bdf

## DEC-031 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: builder: opencode Anthropic/sonnet-5.5 -> opencode anthropic/claude-sonnet-5-5.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 721457621301cd3994f4409ea03788b3f82a4de889f0442a070268c65547121a
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 4071fc4b2e2ee011e2977b2bdc94b5694dc841f4c5b90e7919d2c1e8cf7d7121

## DEC-032 Engines changed by the owner
- Decided by: owner
- Status: OWNER DECISION (agent-bridge engines)

Engines: supervisor: opencode openai/gpt-6.2-astra -> opencode anthropic/claude-opus-5-5, effort xhigh -> default; builder: effort default -> xhigh.

Changed since the last approval: bridge.toml.

- docs/PRD.md: sha256 721457621301cd3994f4409ea03788b3f82a4de889f0442a070268c65547121a
- CLAUDE.md: sha256 fd8248df5c3fafaf5cb4221c92f7a78709b7d5b811c6bdcc183a7b3cbdf97083
- bridge.toml: sha256 760fecbf36bfb597e96e9243244bfba0dacd46dda0524e303194501cb1350a2b

## DEC-033 Defer PO paste invalid-calendar-date behavior (OPEN-009)
- Decided by: supervisor
- Status: AUTONOMOUS DECISION - owner to review

Options considered: (a) adopt the frozen parser's calendar-invalid date string; (b) adopt the Rust fixture's retention of the prior valid draft date; (c) defer.

Choice: (c). Neither (a) nor (b) is approved for release. `BillDate` validation and both mismatch thresholds stay unchanged. Legacy runtime behavior stays NOT RUN until [OPEN-008](OPEN.md#open-008-contained-browser-for-phase-1-walkthrough) passes. Any permanent departure requires written owner approval.

Reason: This is the most conservative option and mirrors [DEC-027](#dec-027-defer-repeated-product-crate-valuation-l10-01). See [OPEN-009](OPEN.md#open-009-po-paste-calendar-date-discrepancy-l07-04).

## DEC-034 Owner and Manager permission matrix (OPEN-004)
- Decided by: owner (2026-11-27), technical defaults proposed by the reviewer and accepted with the matrix
- Status: OWNER DECISION

Two roles only: **Owner** and **Manager** (no Cashier). Anything not listed as allowed stays denied. Enforce every rule on the server (SQL role checks and role-filtered pulls), not only in the app.

**Manager may:**
- Make bills and **finish (post) them when online**: payment, final invoice number and stock effects are applied by the server after it re-checks current prices, stock and terms. Drafts made offline still wait until the device is online; reconnecting never posts by itself. This replaces "Manager provisional-only".
- Add crate rows to bills and see crate and item **selling** rates. Manager drafts get the shop's default loading charge.
- **Edit and cancel posted bills.** Every edit and cancel is recorded in the activity log with the account that did it.
- See customer **balances (udhaar)**, mobile and address, add customers and edit their contact details, and **record customer payments** (cash or online). Opening balances stay Owner-only.
- **Cash Book:** enter today's figures and expenses and close today. Earlier days, edits to closed days and month totals stay Owner-only.
- **Stock work without costs:** receive goods against existing purchase orders (quantities only), adjust stock counts, update the manual godown list, create and edit crate templates, view stock quantities and the reorder list (quantities only). View and share the Rate List (selling rates).
- Use the **Counter** screen (today's bills, no profit), share bill images and print bills and estimates.
- Check the status of, and retry, its own confirmations whose outcome is unknown (same account only, never after switching to another account).

**Owner only:**
- Buying costs, buy rates, supplier prices, FIFO lots and stock value, margins and profit, anywhere: screens, pulls, device files, exports, backups, bill or report output, error text.
- Products and price changes, suppliers and supplier prices, the Order Planner, creating purchase orders, closing remainders.
- Dashboard, Reports, GST bills, Settings (shop-wide), Backup & Restore, exports, legacy import and quarantine resolution, the activity log, Clear All, opening balances, Cash Book history.

**Technical defaults (accepted):**
- A role downgrade or revocation ends the session immediately and deletes that account's stored token; projections and queued output are purged as today; local drafts stay locked to that account.
- A refresh token may stay on a shared device until sign-out, downgrade or revocation; a memory-only session (no credential store) is acceptable.
- Real accounts are created by the Owner through a reviewed admin flow (Supabase dashboard or an Owner-only admin command), never with a client-side service-role key; the CI service-role token is test-only.
- Activity log entries name the account by its login identity (email), Owner-only to read.
- Manager refusals read "Only the owner can do this." (or the HTML's own wording where it has one).
- Change history kept for 10,000 revisions; 64-bit row hashes accepted; device IDs drawn per install until devices are provisioned (OPEN-005); drafts from before the shop-wide ID keep working as this device's drafts.
- The server should parse a raw legacy file itself before trusting a mapped plan (to implement); quarantined records are resolved by the Owner with the accounting reviewer (OPEN-003).

Supersedes the "Manager stays provisional-only" rule used in local development and DEC-015's unapproved proposal for the two roles it covers.

## DEC-035 Fractional packets: round money to the paisa per bill (OPEN-014)
- Decided by: owner (2026-11-28: "use rounding"); stock representation and pack-size rule proposed by the reviewer and recorded with it
- Status: OWNER DECISION

- **Money:** a bill's loading charge (packets × loading rate) and its total are rounded to the paisa **on that bill**, half up, and stored as exact paise. Balances add the stored paise. This departs from the HTML's floating-point totals; the documented example (three unpaid "1 pc" bills of a 3-piece pack at ₹1.49/pkt loading) shows ₹152 where the HTML shows ₹151. Accepted.
- **Stock:** crate components may hold loose pieces. Stock, FIFO lots and movements are kept exactly in pieces (whole numbers); packets are derived for display. Screens show stock as the HTML does ("9 + 3pc", and packet figures rounded for display as the HTML's `fmtN`), never a raw float.
- **Pack-size changes:** a sale keeps the pack size it was made with; changing a product's pack size later does not recompute earlier bills or stock (the HTML's retroactive recompute is not carried over).
