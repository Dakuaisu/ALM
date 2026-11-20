# Open items
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
## OPEN-006 Hosted test project and implementation authorization
- Status: OWNER-BLOCKED
The owner must authorize the isolated hosted project and test access and separately approve implementation after Phase 1 evidence is available. Do not use production as a substitute.
## OPEN-007 Cutover and rollback authority
- Status: OWNER-BLOCKED
The owner must confirm the production write pause, identify the accounting signer and rollback authority, and separately authorize production cutover after rehearsal and parity sign-off. The rollback proposal remains unapproved under DEC-016. Confirm how post-switch sales will be retained and reconciled before any rollback.

## OPEN-009 PO paste calendar date discrepancy (L07-04)
- Status: OWNER-BLOCKED
The [frozen parser](../ALM_Business_Manager.html#L6405) can produce a calendar-invalid date string. The Rust fixture instead retains the prior valid draft date when the paste has no valid date ([L07-04 development result](RESULTS.md#purchase-order-paste-formats-and-mismatch-boundaries)). Legacy runtime behavior is NOT RUN and remains unverified. Any permanent departure requires written owner approval. Keep `BillDate` validation and both mismatch thresholds unchanged.
