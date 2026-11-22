# Owner decision: deliver multiple workflows per batch

## D1 - Replace the one-case cadence with multi-workflow delivery batches

Revise the PRD's local delivery section from the current checkpoint. Group many remaining related workflows into each delivery batch, not one billing workflow or one legacy case per exchange. Use the completed work and open evidence to choose batches based on shared dependencies and what can actually be exercised together. For example, group billing, estimates, receipts, customer balances, and Cash Book when they share the same transaction path; group supplier pricing, purchasing, receiving, FIFO, and stock when they share the same inventory path. These groupings are examples, not a restriction to those areas.

For each batch, specify the existing code to reuse, the remaining connected workflows to finish together, runnable integrated checks, and one consolidated Supervisor review and evidence handoff. Give the Builder a kickoff that names multiple workflows to tackle before handing back. Keep focused regressions and coherent commits within a batch, but avoid separate planning, review, or evidence-only turns for each case. Isolate blockers so independent local work can continue. Preserve the current archive-backed rounding work and any completed evidence rather than duplicating it.

## D2 - Keep the existing authorization and evidence boundaries

Keep the current local synthetic authorization, permissions, case traceability, and all formal phase exit criteria. Do not mark phases complete, weaken tests, or authorize hosted access, real data, device acceptance, migration, or production cutover. Do not change model or effort settings in the replan.

## Done when

- [ ] The revised plan defines a few multi-workflow batches covering the remaining authorized local work, with dependencies and runnable integrated checks.
- [ ] The kickoff assigns a batch of connected workflows rather than one billing workflow, one test case, or a documentation-only task.
- [ ] Completed evidence and owner gates are unchanged, and the Supervisor evaluates results by batch.
