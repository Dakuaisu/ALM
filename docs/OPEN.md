# Open items

## Consolidated owner checklist

Use this checklist for one combined owner request when needed. The numbered entries below retain the evidence, scope, and status of each item. Do not ask again for the deployed HTML identity or for routine local implementation approval. Owner replay of the demo is optional and is not on this checklist.

- [ ] **Discovery prerequisites — OPEN-001, OPEN-008:** Supply the remaining deployment details and dependencies, and provide the contained browser environment. Provisioning it is not a passing isolation probe or a completed walkthrough.
- [ ] **Protected evidence and access — OPEN-002, OPEN-006:** Separately authorize each required environment and real-record use. Specify the permitted read-only metadata or protected copies for discovery and, separately, the isolated hosted test project and accounts for integration tests. Neither permits production writes. Do not put credentials or raw records in the repository.
- [ ] **Business and permission decisions — OPEN-003, OPEN-004, OPEN-009:** Decide source conflicts and precedence with accounting; approve the production role/data matrix and stale-term authority. Require a written decision before any material departure from the HTML. In particular, do not change `BillDate` or either paste-mismatch threshold to clear an unverified case.
- [ ] **Device prerequisites and parity sign-off — OPEN-005:** Confirm targets, provide access to actual devices and approved share recipients, and resolve disputed parity results against the original. Keep installed workflows, image handoff, print and PDF subject to their existing checks.
- [ ] **Migration, accounting, and cutover — OPEN-002, OPEN-003, OPEN-007:** Authorize migration rehearsal with protected copies separately. Obtain owner and accounting sign-off on reconciliation and invoice continuity, agree the write pause and rollback authority, and obtain a distinct production cutover authorization before switching. Approval to rehearse is not approval to switch.

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
The [frozen parser](../ALM_Business_Manager.html#L6405) can produce a calendar-invalid date string. The Rust fixture instead retains the prior valid draft date when the paste has no valid date ([L07-04 development result](RESULTS.md#purchase-order-paste-formats-and-mismatch-boundaries)). Legacy runtime behavior is NOT RUN and remains unverified. Any permanent departure requires written owner approval. Keep `BillDate` validation and both mismatch thresholds unchanged.
