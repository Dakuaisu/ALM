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

## DEC-019 Local synthetic Phase 2 slice
- Decided by: owner
- Status: APPROVED (local synthetic implementation only)

The owner authorized a small Rust slice for provisional sales drafts and Owner/Manager access. Owner can use all operations in the slice; Manager must not receive total profit. This does not close Phase 1 checks, approve the full production permission matrix, authorize hosted or production access, or authorize Phase 3.
