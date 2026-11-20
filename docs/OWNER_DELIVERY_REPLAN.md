# Owner decision: streamline ALM delivery

## D1 - Preauthorize routine local development

Review the entire approved plan against the current repo, demo, results, decisions, and open items. Revise the plan so reversible local implementation, synthetic tests, and internal milestone sequencing do not need separate owner approvals or feedback files. The owner may try the draft demo without blocking unrelated local work; do not record it as accepted. Continue all independent local work when one track is blocked. Do not mark a phase complete without its required evidence.

## D2 - Deliver integrated workflows in larger batches

Reuse the Rust core and the runnable draft demo. Prioritize the next runnable end-to-end billing workflow, then group the remaining local work into a few coherent milestones instead of one exchange per inventory case. Identify the first concrete build task, dependencies, and work that can proceed alongside discovery. Batch evidence and Supervisor review per milestone while preserving case traceability and tests for financial correctness, permissions, and retry behavior. Do not spend separate turns only reformatting evidence or requesting optional feedback.

## D3 - Keep genuine acceptance and access gates

Retain requirements, acceptance thresholds, Owner/Manager restrictions, and evidence labels. Clearly separate approvals from prerequisites. Consolidate genuinely required owner decisions into one checklist rather than interrupting for each one. Keep separate authorization for hosted or production access, real records, material departures from legacy behavior, and migration or cutover; keep actual device/parity and accounting sign-off requirements. No credentials, hosted access, real records, or production changes are authorized here. Do not change model settings or Bridge configuration. Record unverified evidence as blocked or not run, never passed.

## Done when

- [ ] The revised plan identifies which routine stops were removed and lists the remaining owner-only gates once.
- [ ] The milestones give the Builder an actionable local kickoff and a runnable deliverable, not a documentation-only task.
- [ ] Existing evidence and phase status remain intact; no owner feedback is fabricated and no acceptance check is weakened.
