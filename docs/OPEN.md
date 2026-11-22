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
| `supabase/config.toml`, `supabase/.gitignore` | Untracked Supabase CLI configuration, not read |
| `docs/OWNER_REVIEW.md` | Untracked document, not read |
| `package.json`, `package-lock.json`, `node_modules/` | Untracked Node tooling files and installed packages |
| `.kiro/agents/default-eed8665b.json` | Untracked agent-tool configuration |
| `.gitignore` | Untracked root ignore file |
| `.DS_Store`, `phase2-local/.DS_Store` | Untracked macOS metadata |
| `phase2-local/tests/milestone_d_replay/outputs/.verify.DawLfe/` | Untracked replay run artifacts (SVG, PNG, HTML, text) |
| `phase2-local/tests/milestone_d_replay/recovery/harness.rs` | Untracked Rust test source |
