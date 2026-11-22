# Results

## Phase 1 discovery

These are development checks and dispositions, not acceptance or production outcomes. **FAIL** means the preflight check did not meet its criterion; **BLOCKED** means prerequisite evidence or setup is missing; **NOT RUN** means the check was not attempted. No app-level pass or failure was observed. Build reference: contract `e0500b3`; the uncommitted repository HTML is owner-confirmed as the deployed file, with deployment dependencies still unverified.

| Result ID / case | Kind | Environment / fixture | Expected | Observed / value | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| DISC-HASH / R-1 | development | Local source file; no runtime fixture | Candidate digest matches the previously pinned digest | SHA-256 `cd2887712e8a5a7e2fb9d45299d9034614cb4f9ec640a08ab1670acc42dd0043`, matching the prior candidate fingerprint only | PASS | `shasum -a 256 /Users/apple/ALM/ALM_Business_Manager.html`; [baseline record](PHASE1_DISCOVERY.md#baseline-and-evidence-status) |
| DISC-ISO / R-1 | development | Local macOS shell; Playwright MCP; no ALM data or navigation | Actual disposable browser runs under verified deny-by-default egress, with an inert loopback-positive/outbound-negative probe before app load | Docker CLI could not connect to its local daemon; `sandbox-exec` ran inert `true` but that command did not test networking or establish inheritance by the Playwright browser. MCP tab listing showed `about:blank` only. No qualifying containment probe ran. | FAIL (preflight gate only) | `docker image ls`, `sandbox-exec` inert smoke check, Playwright `browser_tabs` list; [isolation checklist](PHASE1_DISCOVERY.md#isolated-walkthrough-checklist) |
| DISC-START / L01-01 through L01-13 | development | Synthetic cases planned; no profile loaded the candidate. L01-02/L01-03 revision pairs are synthetic inputs (localStorage / IndexedDB): 2 / 1, 3 / 3, and 1 / 2. | Observe startup, arbitration, corrupt data/recovery, restore, shortcut and one-time schema upgrade per case. Source-derived winner markers for the three pairs: `L01-FIXTURE-LS`, `L01-FIXTURE-LS`, and `L01-FIXTURE-IDB`, respectively. | No runtime outcomes observed. Static fixture check output: `true` (`jq -e` verified the fixture shape, revision pairs, markers, and NOT RUN labels). Static development evidence only. | NOT RUN | [Synthetic arbitration fixture](phase1-storage-arbitration.json), [fixture case notes](PHASE1_CASES.md#fixed-revision-arbitration-fixtures-not-run), [startup cases](PHASE1_CASES.md#startup-and-recovery-run-first-after-isolation-passes), [schema upgrade cases](PHASE1_CASES.md#one-time-schema-upgrade-not-run) |
| DISC-INLINE / L01-14 | development | Planned synthetic startup on an isolated loopback origin; candidate not loaded | Compare the candidate's inline resource declarations with browser rendering, script execution and observed requests | No rendering, script execution or request evidence was collected. Deployed dependency verification remains OWNER-BLOCKED under OPEN-001. | NOT RUN | [Inline-resource case](PHASE1_CASES.md#inline-page-resources-not-run), case commit `0c0f1a7`; existing DISC-ISO failure unchanged |
| DISC-MIRROR / L01-15 | development | Planned synthetic storage fixtures in an isolated disposable browser; candidate not loaded | Compare write attempts around the localStorage mirror cutoff, then distinguish the save warning, durable readback and reload selection after IndexedDB completion or abort at the cutoff | No write, readback, or reload outcome was observed. | NOT RUN | [Mirror cutoff and reload case](PHASE1_CASES.md#localstorage-mirror-cutoff-not-run); case commits `8c95600` and `ab47dcf`; existing DISC-ISO failure unchanged |
| DISC-ACTIONS / L02 through L20 | development | Synthetic cases planned | Compare documented source branches to isolated observations | Static expectations only; no observed values, artifacts or mock share run | NOT RUN | [Action cases](PHASE1_CASES.md) |
| DISC-DEPLOY / OPEN-001 | development | Owner confirmation and local repository hash; no laptop session | Identify the deployed HTML and capture dependency evidence | Owner confirmed `ALM_Business_Manager.html` is the deployed copy. A fresh `shasum -a 256` returned `cd2887712e8a5a7e2fb9d45299d9034614cb4f9ec640a08ab1670acc42dd0043`, matching the pinned digest. Deployment dependencies were not verified. | BLOCKED (dependency evidence only) | Owner message, [baseline record](PHASE1_DISCOVERY.md#baseline-and-evidence-status), DEC-012 |
| DISC-DATA / OPEN-002 | development | No authorized live discovery channel or approved samples | Capture live metadata and compare protected Supabase/Sheets/JSON evidence | No live connection or capture; no precedence assigned | BLOCKED | [Data checklist](PHASE1_DISCOVERY.md#read-only-data-discovery-checklist), DEC-013 |
| DISC-DEVICE / OPEN-005 | development | Windows/macOS laptops and Android tablets not confirmed | Record results on each approved target, including actual receipt of WhatsApp image and separate print/PDF output | No approved targets/test accounts or recipients; no handoff attempted | BLOCKED | [Device matrix](PHASE1_DISCOVERY.md#device-and-sharing-matrix), DEC-014 |
| DISC-REPORT / L16-02 through L16-06 | development | Static trace of candidate HTML; synthetic fixtures specified only | Verify cutoff, cancellation, valuation, product identity and chart behavior in isolation | Source expectations added; candidate not opened and no later containment preflight run | NOT RUN | [Action cases](PHASE1_CASES.md), original DISC-ISO failure above |
| DISC-ISO-FOLLOWUP / R-1 | development | Local macOS tool availability only; no browser probe | Launch a disposable browser with process-scoped deny-by-default network controls and verify allowed loopback and denied egress with inert local controls before opening the candidate | Docker CLI still could not reach its local daemon; no local browser or Playwright launcher was found on PATH for a process-scoped sandbox. A disposable MCP context alone does not establish OS-level egress containment. No host-wide changes, network probe or candidate navigation. | NOT RUN | Local `command -v` and `docker image ls`; original DISC-ISO failure is unchanged. Synthetic L01 and all candidate runtime cases remain NOT RUN. |
| DISC-VM / R-1, OPEN-008 | development | Owner-provided Lima VM `alm-walkthrough`; guest-only setup authorized | Establish a supported, pinned Playwright/Chromium pair and verify private-lab containment before transferring the HTML | Guest shell reached; `lsb_release` and `uname` reported Ubuntu 26.10 on `aarch64`. This release is outside Playwright's documented support matrix. No Playwright, Chromium, or container runtime was found on the guest's `PATH`. Mount inspection found no Lima shared filesystem, and the repository HTML path was not readable in the guest. No tooling was installed or HTML transferred. | BLOCKED (supported browser environment) | `limactl shell`, `lsb_release`, `uname`, `findmnt`, `command -v`, `test -r`; [Playwright system requirements](https://playwright.dev/docs/intro#system-requirements); [OPEN-008](OPEN.md#open-008-contained-browser-for-phase-1-walkthrough), DEC-017 |
| DISC-VM-GATE / R-1, OPEN-008 | development | Prospective contained guest browser; tooling prerequisite blocked at DISC-VM | Record matched reachability control, loopback receipt, browser-attributed OS denial, no matching non-loopback receipt, and effective default-deny policy | No browser launched and no containment evidence collected. The candidate was not transferred, served or opened; synthetic L01 remains NOT RUN. | NOT RUN | [Inert browser gate](PHASE1_DISCOVERY.md#inert-browser-gate-not-run); original `DISC-ISO` failure unchanged |

## Phase 2 local synthetic slice

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-LOCAL / R-5, R-6 | development | Local Rust crate, inline synthetic roles and drafts, in-memory store | Owner and Manager can save, reopen and edit provisional drafts; Owner receives synthetic total profit and Manager receives a denial without a value | `cargo test --offline`: 4 passed, 0 failed. Formatting and offline Clippy passed. LSP diagnostics timed out. | PASS (local slice only) | [Public API tests](../phase2-local/tests/phase2_local.rs), [Cargo manifest](../phase2-local/Cargo.toml), DEC-019; tested in the working tree before commit `ca02c32cbf7be19a9da8da008e932784be92b8b1`. |
| P2-SQLITE / R-5, R-6 | development | Local Rust crate, installed `sqlite3` CLI, synthetic database under the ignored Cargo build directory | A fresh process reopens committed drafts without posting; a failed edit preserves the prior draft; only Owner receives total profit | `cargo test --offline`: 7 passed, 0 failed; 1 worker excluded from direct discovery and invoked by the restart test. An edited draft and an empty draft reopened intact in a fresh process; the schema contained only draft tables. The injected line-insert failure rolled back the edit. Owner received the synthetic total and Manager received a denial. Formatting and offline Clippy passed; LSP diagnostics timed out. | PASS (local synthetic slice only) | [Restart and rollback tests](../phase2-local/tests/sqlite_restart.rs), [SQLite adapter](../phase2-local/src/sqlite.rs); working-tree run, uncommitted |

| P2-CONFIRM / R-6, R-7 | development | Single-product in-memory posting fixture; no SQLite posting or network | Explicit Owner confirmation applies fixture stock, receipt, and ledger effects once; stale terms, Manager or conflicting requests, and failed validation do not post | 7 confirmation tests passed. An identical retry after a price change returned the original receipt. Competing reviewed drafts did not oversell in sequential execution; checked arithmetic failures left posting state unchanged. | PASS (local fixture only) | [Confirmation implementation](../phase2-local/src/confirmation.rs), [confirmation tests](../phase2-local/src/confirmation/tests.rs); uncommitted working-tree run |
| P2-RECONNECT / R-5, R-6, R-7 | development | Synthetic SQLite draft, separate writer and reader processes, in-memory confirmation fixture | Reconnect reviews but does not post; a changed price requires a fresh Owner review and explicit confirmation; Manager cannot confirm or read total profit | Reconnect, stale review and Manager denial left receipt count, stock and ledger unchanged. Owner saw the new price and explicitly confirmed; the fixture posted once. Independent readback confirmed the draft was unchanged and SQLite still held only provisional tables. | PASS (local fixture only) | [Reconnect regression](../phase2-local/tests/sqlite_restart/reconnect.rs); `cargo test --offline --manifest-path phase2-local/Cargo.toml` |
| P3-DRAFT-ENTRY / L03-04 | development | Pure Rust draft-entry transitions, synthetic inputs | Preserve Cancel/Discard/Continue for guarded New Bill entry, and immediate fresh-draft outcomes for direct Fresh Bill and Clear | 7 transition tests passed. No screen, browser, storage mutation or target device was exercised. | PASS (local domain logic only) | [Draft-entry implementation and tests](../phase2-local/src/draft_entry.rs); uncommitted working-tree run |
| P3-DRAFT-ENTRY-REV / L03-04 | development | Clean `phase2-local` crate at `911c87c74367f602d5371d5598519b1b44336fe4`; local `sqlite3` on `PATH` | Reproduce the draft-entry transitions and adjacent local Rust regressions from committed source | 22 passed, 0 failed, including all 7 draft-entry tests. The 1 ignored restart worker ran through its integration tests. | PASS (committed local run only) | `cargo test --offline --manifest-path phase2-local/Cargo.toml`; recorded post-commit run at `911c87c` |
| P3-DRAFT-WORK / L03-04 | development | Synthetic typed active-form facts; working tree based on `911c87c74367f602d5371d5598519b1b44336fe4`, with the classifier and tests changed | Derive the New Bill prompt from the frozen HTML's `draftHasWork` (L3369-3370); preserve direct Fresh Bill and Clear without prompting or posting | 30 passed, 0 failed, including 15 draft-entry tests. Blank forms and qualifying work without finalized lines were distinguished. Both direct actions bypassed the prompt; the pure transition API has no posting path. The ignored restart worker ran through its integration tests. Formatting and offline Clippy passed; editor diagnostics timed out. | PASS (local domain logic only) | `cargo test --offline --manifest-path phase2-local/Cargo.toml`; [classifier](../phase2-local/src/draft_entry.rs) SHA-256 `ff9923cd11503bed62f4a36a97939a651baf7e563ee9f3e67d616074cacbc3e7`; [tests](../phase2-local/src/draft_entry/tests.rs) SHA-256 `a510d679ca04ce3681d4b6fdac9def14e26d0acb54a822ad6d1c13ccf12601cc`; not a committed-revision run |
| P3-DRAFT-WORK-REV / L03-04 | development | Clean crate at `1fc14a4147113c66a2d16a2c54c43308f9f84bea`; `sqlite3` on `PATH` | Reproduce the draft-work classifier suite from committed source | 30 passed, 0 failed, including 15 draft-entry tests. The restart worker was invoked by its integration tests. | PASS (committed local run only) | `cargo test --offline --manifest-path phase2-local/Cargo.toml`; previously reported post-commit rerun, separate from P3-DRAFT-WORK |
| P3-CUSTOMER-PREFILL / L03-04 | development | Clean crate at `ff90fe7c204f1ac1d870a0d3053a8b6cfc08b0be`; distinct synthetic customers | Match `openNewBill(cust)` (HTML L3373-3425): fresh and Discard set the requested ID and name; Cancel and Continue keep the existing customer; no-customer entry stays unassigned | 36 passed, 0 failed. Tests also preserved typed customer names without a selected ID. New forms contain empty provisional drafts; no posting or final invoice path is involved. Formatting and offline Clippy passed before commit; editor diagnostics timed out. | PASS (local draft logic only) | `cargo test --offline --manifest-path phase2-local/Cargo.toml`; [draft entry](../phase2-local/src/draft_entry.rs), [tests](../phase2-local/src/draft_entry/tests.rs); post-commit run |
| P2-CUSTOMER-SQLITE / R-5, R-6, L03-04 | development | Clean local crate at `c2e1cdd7be3ba491d42afc4fb573685e94e7c6f1`; `/usr/bin/sqlite3` on `PATH`; disposable SQLite database and synthetic fixtures only | Save and reopen the selected ID and name, typed name with no ID, and unassigned draft exactly; isolate a customer edit; keep save and reopen provisional and deny Manager total profit | 40 passed, 0 failed. The restart test used separate writer, reopen/edit, and final readback processes; all three states matched exactly, and only the edited draft changed. SQLite contained only provisional tables, with no sale or final invoice created by save or reopen; Manager received `AccessDenied` without a profit value. Older drafts reopened unassigned without losing lines, and a forced line-insert failure rolled back both customer and lines. The restart worker was excluded from direct discovery and invoked by its parent tests. Formatting and offline Clippy passed; editor diagnostics timed out. | PASS (local development only) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [restart and rollback tests](../phase2-local/tests/sqlite_restart/customer.rs); post-commit run |
| P2-CLIENT-RETRY / R-6, R-7 | development | Synthetic Owner clients in separate local processes; same in-memory posting fixture throughout; SQLite held the draft only, and a test-only file held the pending request, not a receipt | Drop the first response after posting; draft reopen sends nothing; an explicit retry repeats the exact identity and review and returns the original receipt; changed contents under the same identity conflict without further effects | The first client received EOF after posting. Reopen sent zero requests. Request `71` with the changed review returned `RequestConflict`; explicit retry sent the original identity and full review and the fixture returned the original receipt (draft `17`, invoice `1`, quantity `2`, total `250` minor units). There was one receipt, stock went from `5` to `3` and stayed there, ledger went from `0` to `250` minor units and stayed there, and next invoice stayed at `2` after posting. Offline suite: 41 passed, 0 failed; two workers excluded from direct discovery were invoked by their parent tests. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (local client restart only; no authority restart, hosted concurrency, or acceptance evidence) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [restart test](../phase2-local/src/confirmation/tests/restart.rs); tested working-tree revision below |

P2-CLIENT-RETRY tested revision: working tree based on `790bb7ae0243cd4665f8c7f20ea23c3a8ef7f36e` with the following crate source changes (SHA-256, paths relative to `phase2-local/src/confirmation/`):

- `tests.rs`: `63e8880a439098c57bcd80eb0388bdc781b3eb3476446dfa73e9bd23a292aa5d`
- `tests/restart.rs`: `9311886bb352cc394f05ceb6039f819c53739db463ed70b9459cee236169712b`
- `tests/restart/codec.rs`: `ce665c8fc8a0e20415122b3c6f237938a1cceb5bad2944beb37dd4d6a10ec0db`
- `tests/restart/transport.rs`: `bc92345aaca37a962117ddc38c8f5045235b6a31d4bf29a7d6cfcdd7324899e7`

### Client restart: committed run

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-CLIENT-RETRY-REV / R-6, R-7 | development | Clean `phase2-local` crate at `dd841c18f56b97487349f3896d134965c220d4ad`; `/usr/bin/sqlite3` on `PATH`. A test-only file preserved the pending request, and the same in-memory posting fixture stayed alive across client restarts. | Reopen sends no request; explicit retry sends the original identity and unchanged review and gets the original receipt; a changed review under the same identity conflicts without additional effects | 41 passed, 0 failed. Both subprocess workers were invoked by their parent tests, not counted as standalone passes. The client-restart regression confirmed zero requests on reopen, the original receipt on retry, and `RequestConflict` for changed contents. The fixture still had one posted sale, stock of 3, and a ledger of 250 minor units. | PASS (synthetic client retry only; not evidence of durable application request storage, server-restart recovery, hosted concurrency, or acceptance) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [client-restart regression](../phase2-local/src/confirmation/tests/restart.rs); separate post-commit run |

| P2-SQLITE-PENDING / R-6, R-7 | development | Clean local crate at `ae214e2c552bc6e9018385918345b0b82b8f3b47`; `/usr/bin/sqlite3` on `PATH`; fresh synthetic client processes, no pending sidecar file. SQLite holds the request only; the same in-memory fixture owns the sale and receipt. | Commit the exact Owner approval before sending; send nothing on failed save, draft reopen, or Manager retry; explicitly retry the stored ID and contents after response loss and a price change | 46 passed, 0 failed; subprocess workers ran through their parent tests. An independent connection read the committed approval before the fixture posted. A forced SQLite abort sent nothing, and a duplicate ID did not replace the original approval. Reopen and Manager retry sent nothing. After the response was dropped and the fixture price and local draft changed, the explicit Owner retry sent the original ID and review and got the original receipt. Receipt count stayed at one, stock at 5 (from 8), and ledger at 579 minor units. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (local synthetic only; no hosted authorization, server-restart recovery, concurrency, or phase acceptance) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [SQLite pending restart tests](../phase2-local/src/confirmation/tests/restart/sqlite_pending.rs); post-commit run |

### Competing local confirmations

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-COMPETING / R-6, R-7 | development | Clean crate at `60d659a7e96837a13a15a2d24fe2894758a49dd7`; `/usr/bin/sqlite3` on `PATH`; two synthetic Owner subprocesses with separate SQLite stores per case. The parent observed both sends and independently read both exact committed approvals before it called the same in-memory fixture's `confirm` serially. | Identical ID and review replay the same receipt; different IDs for one draft do not post twice; different drafts cannot both consume the last stock. Rejected requests have no partial posting effects. | Identical requests returned the same full receipt to both clients. The same-draft loser received `DraftAlreadyPosted`; the last-stock loser received `InsufficientStock` in both delivery orders. Each run recorded one sale and one receipt, issued invoice `1` only (next invoice `2`), allocated 2 units once, and left the ledger at 386 minor units. Stock went from 5 to 3 in the same-draft cases and from 2 to 0 in the last-stock case. No loser was recorded as posted. Save-before-send, reopen without automatic posting, explicit Owner retry, and Manager denial stayed covered. Focused: 7 passed, 0 failed; full: 49 passed, 0 failed. Worker entry points were invoked by their parent tests. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (overlapping local clients; not Supabase transaction isolation or acceptance) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::restart`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [competing confirmation tests](../phase2-local/src/confirmation/tests/restart/competing.rs); post-commit runs |

### Atomic multi-line local confirmation

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-MULTILINE / R-6, R-7 | development | Clean crate at `0759ac26cd8ac8ffb069231fcdfd33a312d8aa25`; `/usr/bin/sqlite3` on `PATH`; synthetic products A and B priced at 125 and 210 minor units. SQLite held only drafts and pending approvals; the in-memory fixture owned the sale and receipt. | An explicit Owner confirmation posts all reviewed lines together. Later or cumulative stock shortages leave every posting effect unchanged. A lost-response retry replays the original receipt; changed contents conflict. Manager is denied. | The A, B, A sale posted quantities 2, 3, 1 as one receipt and one invoice (`1`; next invoice `2`). A stock fell from 8 to 5, B from 5 to 2, and the ledger rose from 0 to 1,005 minor units. A later B shortage and cumulative A shortage each left both balances, ledger, next invoice and posting records unchanged. After response loss, a fresh SQLite client resent the exact request despite a fixture price change and received the original receipt without new effects; changed contents returned `RequestConflict`, reopen sent nothing, and Manager retry was denied. Focused: 26 passed, 0 failed; full: 58 passed, 0 failed. Workers ran through parent tests. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (local synthetic only; legacy numbering, FIFO valuation, hosted transactions, and acceptance not verified) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [atomic posting tests](../phase2-local/src/confirmation/tests/multiline.rs), [SQLite restart tests](../phase2-local/src/confirmation/tests/restart/sqlite_pending.rs); post-commit runs |

### Synthetic FIFO allocation

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-FIFO / R-5, R-6, R-7; L11-01, L03-02 | development | Local Rust working tree based on `8d970e80740f35ea5a0070f864eedbd41d666aae` (source fingerprints below); `sqlite3` on `PATH`; one synthetic product with oldest-first lots of 3 units at 80 and 4 at 20 minor units; sale price 125 minor units | Repeated lines share staged lot balances; a late shortage or cost overflow leaves every posting effect unchanged; an identical retry returns the original receipt without a second allocation; no cost appears in the client draft or SQLite | The first line took 2 from the oldest lot; the next took 1 from the oldest and 2 from the newer lot. Balances went from `[3, 4]` to `[0, 2]`; allocated cost was 280 minor units, separate from the 625 minor-unit sales ledger. Late shortage left `[3, 4]` and all posting state intact; cost overflow also rolled back. A fresh SQLite client retried after losing the first response and received the original receipt without changing the balances, cost, ledger, or posting records. The Manager-facing draft and independently queried SQLite schema and rows contained no lot costs. Focused: 5 passed, 0 failed; full: 63 passed, 0 failed. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (local synthetic only; not historical costing, location, production numbering, hosted, device, or parity evidence) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib fifo`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [allocation and rollback](../phase2-local/src/confirmation/tests/fifo.rs), [restart and client inspection](../phase2-local/src/confirmation/tests/restart/fifo.rs) |

Source trace: the frozen HTML sorts purchase lots by ascending date (line 1220), consumes them in order (1273–1289), and carries prior lines' consumption forward through `_extra` (4042–4073). See [inventory case L11-01](PHASE1_CASES.md#stock-lots-dialog-assertions-not-run) and L03-02. This test uses preordered lots and whole units at integer costs. It does not implement the HTML's shortage-cost fallback, derive order from historical records, or infer location boundaries. The inventory cases remain NOT RUN.

P2-FIFO tested source fingerprints (SHA-256; paths relative to `phase2-local/src/`):

- `confirmation.rs`: `efee3618fb84db145451fd7eed0746f5d324db4c8b55709aca7e5af95f75edab`
- `confirmation/catalog.rs`: `877af081382da8246568b132293401c2f288c4d88cc83fc9c27f748f9bc02d14`
- `confirmation/tests.rs`: `4693a56cbaa4be87634d814fe439056966a63a388b55375ac04fbe4e8d046100`
- `confirmation/tests/fifo.rs`: `240eb4447743b099cd116e4762fbc94018b2e084eca1545ebe01445b74fa89af`
- `confirmation/tests/restart.rs`: `deb1d991aa664971066506ee2cb1e024e667fa4d6ac75e531f942bf414229efc`
- `confirmation/tests/restart/fifo.rs`: `36d8044ff9069ed2d37677da78c83a327df37b7c983261744000c38b0eba2d60`

**P2-FIFO committed run (development):** Clean crate at `08542cea1875574d0975ce0a3698d0bd964d1917`, with `/usr/bin/sqlite3` on `PATH`. `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib fifo` passed **8 tests, 0 failed**; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed **66 tests, 0 failed**. The full suite dispatched both restart workers from their parent tests. Quantity-only lots posted and replayed without supplying an allocation or sale valuation; explicitly seeded zero-cost lots recorded a known zero with the same receipt and posting effects. Mixed known and unknown allocations retained their individual costs without claiming a known sale cost. The priced case again took 3 units from the oldest lot and 2 from the newer lot, leaving `[0, 2]` from `[3, 4]`. Allocated cost was 280 minor units, separate from the sales ledger of 625 minor units. Late shortage and cost overflow left posting state unchanged, replay did not consume either lot again, and the Manager-facing draft and raw SQLite records showed no lot costs. Historical costs, shortage pricing, and location rules were not inferred. Hosted, device, parity, and acceptance gates remain open.

### Named unpaid and part-paid sales

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-NAMED-PAYMENT / R-5, R-6, R-7; L03-02 | development | Clean local crate at `ec3e2cf39daa01e3fa815fb0d76fdb99c1d7d7c8`; `/usr/bin/sqlite3` on `PATH`; synthetic named customers and one live in-memory fixture across client restart | Draft save and reopen do not post. Explicit Owner confirmation posts sale, received amount, due, invoice, and stock together. Rejection has no effects; an identical retry returns the original outcome; Manager is denied. | An unpaid sale recorded a total of 125, received 0, and due of 125 minor units; its customer's balance rose from 0 to 125. A same-name customer with a different ID kept a separate balance. In the part-paid restart case, saving and reopening left received and balance at 0; confirmation posted a total of 250, received 100, and due of 150, moving the balance from 0 to 150 minor units and stock from 3 to 1. Stock shortage and balance overflow left posting state unchanged. After response loss, retry returned the original receipt without another payment, balance increase, invoice, or allocation. Changed payment under the same request ID conflicted, and Manager was denied. Focused: 6 passed, 0 failed; full: 72 passed, 0 failed. Subprocess workers ran through their parent tests. Formatting, Clippy, and build passed; editor diagnostics timed out. | PASS (local synthetic only; hosted, device, production, parity, and acceptance gates remain open) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib named_payment`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [posting tests](../phase2-local/src/confirmation/tests/named_payment.rs), [client restart test](../phase2-local/src/confirmation/tests/restart/named_payment.rs); post-commit runs |

Source trace: [L03-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML stores the named customer ID, `total`, and `paid` on the bill (lines 4075-4124), and `custBalance` adds `total - paid` for active sales keyed by customer ID (lines 1158-1173). This milestone tests only that bill-level relationship in integer minor units. SQLite is not the posting authority. Historical payment allocation, credit policy, and legacy rounding were not inferred; L03-02 parity was not run.

### Exact Piece/Case conversion and loading

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-CHECKOUT / R-5, R-6, R-7; L03-02 | development | Clean crate at `d1b0ae513b4bd33e30db7129e952e60596704d7f`; `/usr/bin/sqlite3` on `PATH`; 12 Pieces at 17 and 2 Cases at 150, 6 Pieces per packet, loading rate 13 per packet, proposed received 100 (money in minor units) | Owner review: 4 packets, merchandise subtotal 504, loading 52, total 556. After confirmation: due 456, FIFO cost 260, lot balances `[3, 4]` to `[0, 3]`. With only 3 packets available, reject without posting. | Review and posting matched those amounts and balances. The shortage left stock, allocations, invoice, payment, balance, and posting records unchanged. After a lost response and a change in the current loading rate, a fresh client loaded the original SQLite approval and received the original receipt with no additional posting. Manager was denied; altered approvals did not post. Focused: 10 passed, 0 failed; full: 82 passed, 0 failed. Formatting, Clippy, and offline build passed; editor diagnostics timed out. | PASS (local synthetic whole-packet slice only; crates and unverified rounding excluded; L03-02 parity and acceptance gates remain open) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib checkout`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [checkout tests](../phase2-local/src/confirmation/tests/restart/checkout.rs), [approval checks](../phase2-local/src/confirmation/tests/checkout_guard.rs) |

Source trace: [L03-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML divides Piece quantity by pack size and passes Case/Packet quantity through (1033-1062). It multiplies entered quantity by the entered-unit rate for merchandise and adds converted packet count times loading rate (3988-4041); save-time FIFO uses the converted quantity (4042-4073). The two-Case row contributes two packets, not one for the whole row; the fixture does not infer a physical case-to-packet multiplier. Fractional conversions are outside this exact synthetic slice. SQLite stores pending approvals, not posting effects.

### Distinct-product crate checkout

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-CRATE-CHECKOUT / R-5, R-6, R-7; L03-02, L10-01 | development | Clean crate at `b24f21928d784fd2d43ef3b1adf7b91d5eb25faa`; `/usr/bin/sqlite3` on `PATH`; one crate row, count 2, rate 450, with 9 Pieces of product 2 (pack size 6) and 2 Cases of product 1 per crate; loading rate 13 per packet, proposed payment 100. Monetary values are in minor units. | One bill row; deduct 3 and 4 packets respectively only on confirmation. Subtotal 900, loading 91, total 991, and due 891. Reject a shortage in the second component atomically; replay the original receipt. | Save and fresh-process reopen left lot balances and posting state unchanged. Review matched the expected row price, deductions and amounts. Owner confirmation moved product 2 lots from `[2, 4]` to `[0, 3]` and product 1 from `[3, 4]` to `[0, 3]`, with authority-only FIFO cost 310. Issued invoice 1 (next invoice 2); sales ledger, money received and customer due moved from zero to 991, 100 and 891 respectively. With only 3 packets of product 1, both products and all posting state remained unchanged. After response loss and a rate change, retry returned the original receipt without new effects. Manager was denied confirmation and retry; the raw populated SQLite draft and approval contained no lot costs. Focused: 17 passed, 0 failed; full: 89 passed, 0 failed. Formatting, Clippy and offline build passed; editor diagnostics timed out. | PASS (synthetic distinct products only; not legacy or hosted parity) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib checkout`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml`; [crate checkout regressions](../phase2-local/src/confirmation/tests/restart/crate_checkout.rs) |

Source trace: the frozen HTML keeps the crate as one priced line (10301-10348), multiplies each component quantity by crate count before packet conversion (1040-1053), and calculates loading from total packets (4003-4041). The fixture uses an explicit per-crate price, not the rounded suggestion. Repeated components round-trip in the draft without deduplication but are declined at review. The [source-derived repeated-product case](PHASE1_CASES.md#fifo-across-bill-rows-and-repeated-crate-components-not-run) distinguishes frozen cost from physical depletion because `_extra` advances only after the whole crate row (4042-4073). That legacy walkthrough remains NOT RUN; this result does not select a valuation for repeated components. Fractional rounding, hosted, device, production and acceptance gates remain open.

### Incomplete draft posting guard

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-BILL-GUARD / R-5, R-6, R-7; L03-03 | development | Synthetic local Rust and SQLite; `/usr/bin/sqlite3` on `PATH`; working tree based on `62cf209751ec3981792d86116d8bcade05732aa7`, fingerprint below | Save/reopen incomplete drafts without posting. At checkout, reject missing billable items, customer name, and bill date in that order. Only explicit Owner confirmation can post the corrected draft. | Each field missing alone returned its typed reason; all three missing returned `MissingBillableItems`. With items present but name and date absent, `MissingCustomerName` came first. No rejected request produced a receipt or changed invoice numbering, stock, FIFO, received amounts, customer balance, or ledger. The incomplete draft reopened and was corrected under the same ID without posting. Explicit Owner confirmation posted once; exact replay returned the same receipt. The control issued invoice 1, moved stock from 5 to 3, and posted total 250, received 40, and due 210 in minor units. Manager was denied. Focused: 9 passed, 0 failed; full: 98 passed, 0 failed. Worker entry points ran through parent tests. Formatting, Clippy, and offline build passed; editor diagnostics timed out. | PASS (local domain validation only; rendered warnings and legacy UI parity NOT RUN) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l03_03`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [guard and SQLite control](../phase2-local/src/confirmation/tests/bill_guard.rs), [direct confirmation checks](../phase2-local/src/confirmation/tests/checkout_guard.rs) |

Source trace: [L03-03](PHASE1_CASES.md#counter-checkout-edit-and-outputs) and frozen `saveBill` (HTML lines 4022-4038). Dates are optional while drafting and are not defaulted. Repeated crate components remain intact in drafts; declining them at checkout is a fixture limitation, not an approved change to legacy valuation. Acceptance gates remain open.

P2-BILL-GUARD tested working-tree fingerprint: `e21c2f028d64c210a64bb5d9224ec4466c1a82bbbc6238ed38aff2f657961225` (SHA-256 of the ordered `shasum -a 256` output for modified and untracked, non-ignored `phase2-local` files, sorted by repository-relative path with `LC_ALL=C` and hashed by absolute path).

**P2-BILL-GUARD committed run (development):** Clean `phase2-local` crate at `ee1633770f96ea60323946f00646e813dcbf485b`, with `/usr/bin/sqlite3` on `PATH`. `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l03_03` passed **9 tests, 0 failed**. `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` passed **98 tests, 0 failed**; the two ignored worker entry points were exercised by their parent tests. The focused run reproduced the ordered rejection reasons, unchanged posting state, and SQLite correction control: only explicit Owner confirmation posted, exact retry returned the original receipt, and Manager was denied. This is local domain evidence only. Rendered warnings and legacy UI parity remain NOT RUN; no acceptance gate changed.

### Dated walk-in cash sale and receipt acknowledgement

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-WALK-IN / R-5, R-6, R-7; L03-01 | development | Clean crate at `d3f777e722c6d9234e4f09247d57e4181488477b`; `/usr/bin/sqlite3` on `PATH`; synthetic dated `Local - Neighbor` bill, fresh client processes and one live fixture | Owner posts full cash with no named debt; rejection and loss retain the active draft; reopen is inert; exact retry restores the receipt without reposting; matching acknowledgement alone clears the active entry and selects the receipt for preview | One sale and receipt, fixture invoice 1, 2 units sold for 250 minor units received in full, zero due and no named balance. Stock went from 5 to 3 and FIFO lots from `[1, 4]` to `[0, 3]`; sales and received ledgers each went from zero to 250. Rejection left the draft editable. Reopen sent nothing after the lost response. Explicit retry returned the same receipt without further effects; acknowledgement cleared only the matching active snapshot and selected the receipt. An edited snapshot and an unrelated saved draft were preserved; Manager was denied. Focused: 5 passed, 0 failed. Full: 103 passed, 0 failed; the two worker entry points ran through parent tests. Formatting, Clippy and offline build passed; editor diagnostics timed out. | PASS (local synthetic domain and client state only; rendered preview and legacy parity NOT RUN) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib walk_in`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [walk-in tests](../phase2-local/src/confirmation/tests/walk_in.rs), [client restart](../phase2-local/src/confirmation/tests/restart/walk_in.rs) |

Source trace: [L03-01](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML offers `Local - Neighbor (Cash)` in the customer picker (3479-3503), provides `Paid Full` (3854-3883), and stores payment mode, total and paid on the sale (4075-4124). The successful-save callback clears the active form and selects the saved sale for preview (3978-3983, 4124-4131). The local client models the acknowledgement and preview selection in memory, retaining saved drafts and pending approvals in SQLite. It does not establish durable authority state, production numbering, or rendered-preview behavior. Acceptance gates remain open.

### Edit a posted sale with New Bill pending

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-SALE-EDIT / R-5, R-6, R-7; L04-01 | development | Clean crate at `c7680905b35b377ebf820d71d1c21a4b9a59dc72`; `/usr/bin/sqlite3` on `PATH`; named, part-paid sale in the synthetic in-memory fixture and a separate New Bill draft in SQLite | Reprice 2 retained units from 125 to 150 and add 3 units of a different product at 90 (minor units). Keep the original invoice, payment, and retained FIFO allocation. Cancel, shortage, and Manager denial must not post an edit. | The same sale and invoice 1 remained; the next invoice stayed at 2. Sales total and ledger moved from 250 to 570, money received stayed at 100, and customer due moved from 150 to 470 minor units. Retained stock stayed at 1, with frozen allocation cost 80. Added-product stock moved from 5 to 2; the new line cost 110, bringing sale FIFO cost from 80 to 190. Cancel and shortage left the original posting unchanged; Manager was denied. Save selected the edited receipt for preview and preserved the active and SQLite New Bill draft. Replaying the original posting returned its original receipt, and re-saving the stale edit was rejected. Focused: 6 passed, 0 failed. Full: 109 passed, 0 failed. Formatting, Clippy, and offline build passed; editor diagnostics timed out. | PASS (local synthetic domain and client state only; rendered, hosted, device and legacy parity not tested) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::edit`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [edit regressions](../phase2-local/src/confirmation/tests/edit.rs), [New Bill isolation](../phase2-local/src/confirmation/tests/edit/isolation.rs) |

Source trace: [L04-01](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML copies the sale identity and retained `costAtSale` into a separate edit form (4137-4179), computes cost for new lines (4042-4074), replaces the sale in place (4118-4124), and previews the replacement only on successful save; Cancel closes the edit copy (4180-4201). This fixture covers repricing an unchanged retained line and appending a different product. It does not infer historical recalculation rules, durable edit retries, or production authorization. SQLite remains non-authoritative and acceptance gates remain open.

### Cancel and restore a posted sale

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-SALE-CANCEL / R-5, R-7; L04-02 | development | Clean crate at `76fdb9131a98e28fb3a9e368b1d2822428d31ea7`; `/usr/bin/sqlite3` on `PATH`; dated, named part-paid sale in the synthetic in-memory authority, no intervening stock movements | Keep the original sale and invoice; cancellation removes the active stock, due and profit effects but retains the CSV row; undo restores those effects once. Deny Manager mutations and profit access. | Active / cancelled / restored: stock `2 / 4 / 2`; FIFO lot balances `[0, 2] / [1, 3] / [0, 2]`; active sales `250 / 0 / 250`; active received `100 / 0 / 100`; customer due `150 / 0 / 150`; Owner gross profit contribution `150 / 0 / 150` (money in minor units). The frozen FIFO cost of 100 was retained. The actual serialized Sales CSV row kept invoice `1`, bill total `250` and paid `100`, with status blank / `CANCELLED` / blank; the row contained no cost or profit fields. The original sale ID and request record survived, and next invoice stayed `2`. Repeated cancel/undo and original request replay did not apply another stock or financial effect. Manager attempts left state unchanged and returned no profit or CSV. Focused: 8 passed, 0 failed; full: 117 passed, 0 failed. Formatting, offline Clippy, and build passed; editor diagnostics timed out. | PASS (synthetic local authority and scoped CSV only; no parity or acceptance claim) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::cancellation`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [cancellation and CSV regressions](../phase2-local/src/confirmation/tests/cancellation.rs) |

Source trace: [L04-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML retains the sale and number when toggling cancellation (6190-6238), excludes cancelled sales from active stock, customer balance and P&L calculations (1077-1173, 8756-8777), and serializes cancelled rows with their status and original paid amount (10060-10070). This fixture reverses the saved quantities against the same lots and conservatively rejects intervening stock movements. Active received totals are a fixture projection, not a refund. Historical FIFO recalculation, full legacy export parity, hosted, device, production, and acceptance gates remain open.

### Saved bill balance modes

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-PREVIEW-BALANCE / L05-01 | development | Clean crate at `3a7c3d798d8b2fb863cb6421013d947b73bf8b42`; `/usr/bin/sqlite3` on `PATH`; in-memory named bill dated `2024-02-29`, total 250 and paid 100, with a separate receipt of 60 dated `2024-03-01` created later. Amounts are in whole rupees. | As on bill date / Latest due: initially `150 / 150`; after the later receipt `150 / 90`; cancelled `150 / -60`; restored `150 / 90`. Fully paid walk-in: zero due, no toggle. | All balance pairs matched. The cancelled marker appeared in the projection while saved lines, total and paid remained intact. The walk-in total and paid were both 250; both mode requests returned zero due with no toggle. Switching modes left fixture state, the separate receipt, and bill content unchanged. The projection contains no FIFO cost or profit fields, and Manager access was denied. Focused: 5 passed, 0 failed; full: 122 passed, 0 failed. Formatting, Clippy, and build passed; editor diagnostics timed out. | PASS (local synthetic projection only; no rendered-preview, device, or legacy parity claim) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l05_01`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [preview tests](../phase2-local/src/confirmation/tests/l05_01.rs) |

Source trace: [L05-01](PHASE1_CASES.md#counter-checkout-edit-and-outputs). The frozen HTML uses earlier-created events for the previous balance (4322-4334), the active customer balance for Latest due (1158-1173, 4221-4233), and a cancellation marker without zeroing saved bill fields (4287-4309). The whole-rupee fixture avoids legacy rounding differences; historical payment allocation and rounding parity were not inferred. Acceptance gates remain open.

### Inert Print and Save as PDF outputs

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-ESTIMATE-OUTPUT / L05-02 | development | Clean crate at `bad5cc4fb65a2d5b2e65aafe0d02186e23d03231`; `/usr/bin/sqlite3` on `PATH`; L05-01 bill dated `2024-02-29`, with a later ₹60 receipt dated `2024-03-01`; in-memory popup sink and explicit synthetic current date `2024-03-15` | Each action emits its own document with `Estimate-1-<customer>` title, 2 units at ₹125, total ₹250, paid ₹100, and the selected balance: ₹150 on bill date or ₹90 latest. No posting, payment, or cost disclosure. | Both handlers emitted one complete script-free HTML document per call with the same escaped title and ordered line content. The `Fixture Cola & <case>` row showed quantity 2, rate ₹125.00 and amount ₹250.00. Items Total and Bill Amount were ₹250.00 and Loading/Delivery was ₹0.00. Each route showed Paid ₹100.00 and Balance Due ₹150.00 in bill-date mode, versus Paid on this bill ₹100.00 and Total Due (as on 15-Mar-2024) ₹90.00 in latest mode. Print recorded `_print` and an on-load intent; Save as PDF recorded `_pdf` and a 350 ms after-load intent. Neither executed the intent. Changing private FIFO costs left both captures identical; neither emitted cost or profit, and fixture state was unchanged. Focused: 15 passed, 0 failed; full: 137 passed, 0 failed. Formatting, Clippy and offline build passed; editor diagnostics timed out. | PASS (scoped inert document content only; no pagination, saved PDF appearance, or device evidence) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l05_02`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [captured document tests](../phase2-local/src/confirmation/tests/l05_02.rs), [privacy control](../phase2-local/src/confirmation/tests/l05_02_privacy.rs) |

Source trace: [L05-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs). Frozen HTML lines 4436-4457 use distinct popup targets and print timing with the same estimate title and shared invoice body. That body reads the current balance mode (4221-4233, 4268-4309); the adjacent `waText` handler starting at 4459 is not part of these actions. The local output covers the tested fields for a regular synthetic estimate, not the full legacy layout or nonzero-loading checkout and crate outputs. The frozen HTML was not launched, and no printer, system dialog, saved PDF, browser rendering, or hosted service was used. All acceptance gates remain open.

Combined offline working-tree run, including pending draft-entry tests: 22 passed, 0 failed; the 1 ignored restart worker was invoked by its parent tests. Formatting and offline Clippy passed; editor diagnostics timed out. The SQLite result covers a clean draft restart only. Confirmation effects and replay records are not durable; hosted authorization, server transactions, multi-device concurrency, and legacy or device parity remain unverified. No Phase 1 disposition or deployment gate changed.

### Inert estimate PNG handoff

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-ESTIMATE-IMAGE-HANDOFF / L05-03 | development | Local offline Rust crate at `caadf905fa3ac33b3dfba648913a4930e1d33e24`; in-memory sinks, posted invoice `1` from draft `41`, separate synthetic receipt, and a complete 68-byte 1×1 RGBA PNG standing in for renderer output. | Select native sharing or fallback after checking the exact file. Native rejection is terminal; fallback downloads before optional chat. Deny Manager before callbacks; leave bill and payment state unchanged. | Every file-bearing capture matched the input bytes, `image/png`, and `Estimate-1.png`. Supported: probe → one share with `Estimate #1` and `Synthetic business — Estimate #1`, whether accepted or rejected. Unsupported: probe → one download → chat intent `910000000000` only for the resolved synthetic mobile; no chat for absent, unmatched, or short mobile. Manager returned `AccessDenied` with no callbacks. Whole-fixture equality held in all cases. Focused: 4 passed, 0 failed. Full: 141 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy and build passed; editor diagnostics timed out. | PASS (local handoff decision only, not acceptance) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l05_03`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [ordered sink captures](../phase2-local/src/confirmation/tests/l05_03.rs) |

Source trace: [L05-03](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [`waInvoice`](../ALM_Business_Manager.html#L4768) at lines 4768–4805, and [`cleanPhone`](../ALM_Business_Manager.html#L4819) for phone resolution. The frozen HTML was inspected, not executed. No URLs were opened, no recipient was contacted, and no file was actually shared or downloaded. Image rendering, visual comparison, actual WhatsApp handoff, and all acceptance gates remain open.

### Inert estimate canvas and text fallback

| Result ID / requirement | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| P2-ESTIMATE-FALLBACK / L05-04 | development | Offline Rust crate at `3d6f6b38d37b27d82618383b4dfa8c6e6f35cf82`; synthetic posted regular packet estimate with prior balance and later receipt; stub render outcomes and in-memory sinks. | An HTML PNG bypasses canvas. After an HTML miss, canvas PNG uses L05-03 once; if canvas also misses, emit only encoded `waText` to the phone-specific or empty-phone destination. Manager invokes no callbacks; bill and payment state are unchanged. | HTML hit: HTML → file probe → download → chat intent. Canvas hit: HTML → canvas → file probe → download → chat intent, with exact PNG bytes, `image/png`, and `Estimate-2.png` at probe and download; neither image path emitted text. Both miss: HTML → canvas → one `_blank` text intent, no file callbacks. Its destination was `https://wa.me/910000000000?text=…` for a matching mobile and `https://wa.me/?text=…` for absent, unmatched, or short mobile. The query escaped `&` as `%26` and decoded to the independently specified source text, including reserved characters, Unicode, and the selected bill-date or latest balance. Manager: `AccessDenied`, zero callbacks. Whole fixture unchanged in every run. Focused: 5 passed, 0 failed; full: 146 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy and build passed; editor diagnostics timed out. | PASS (local fallback only) | `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l05_04`; `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`; [ordered sink captures](../phase2-local/src/confirmation/tests/l05_04.rs) |

Source trace: [L05-04](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [`waInvoice`](../ALM_Business_Manager.html#L4795) at lines 4795–4817 and [`waText`](../ALM_Business_Manager.html#L4459) at lines 4459–4476. The text check covers the regular synthetic packet fixture, not crate, loading-rate, or MRP variants. The frozen HTML was not executed, no URL was opened and no recipient was contacted. Canvas rendering, image legibility, actual WhatsApp handoff, and all acceptance gates remain open.

### Read-only ordinary-sale GST breakdown

**P2-GST-ORDINARY / L06-01 (development, PASS):** Offline Rust crate at `a8e99ddf2cbe28351a26ef7f122765b2b994c4bf`. Both jurisdiction requests used the same posted synthetic invoice `1`, dated `2024-02-29`, with ₹100 paid. Only the pre-resolved jurisdiction changed. The input prices were inclusive; the expected taxable unit price was independently calculated as ₹100 on each line.

| Saved line and tax inputs | Expected effective HSN, GST + cess | Expected taxable / combined tax | Observed emitted fields |
| --- | --- | --- | --- |
| Water, 2 × ₹105; no override | `22011010`, 5% + 0% | ₹200 / ₹10 | `22011010`, 5% + 0%; ₹100 taxable per unit, ₹200 taxable / ₹10 tax |
| Product, 1 × ₹120; product override 18%, cess 2%, HSN `22029920` | `22029920`, 18% + 2% | ₹100 / ₹20 | `22029920`, 18% + 2%; ₹100 taxable per unit, ₹100 taxable / ₹20 tax |
| Override, 1 × ₹112; product 18%, cess 5%, HSN `22029090`; saved line 12%, HSN `21069099` | `21069099`, 12% + 0% | ₹100 / ₹12 | `21069099`, 12% + 0%; ₹100 taxable per unit, ₹100 taxable / ₹12 tax |

| Output | Expected CGST / SGST / IGST | Observed CGST / SGST / IGST |
| --- | --- | --- |
| Intrastate invoice | ₹21 / ₹21 / ₹0 | ₹21 / ₹21 / ₹0 |
| Intrastate HSN groups, in line order | ₹5 / ₹5 / ₹0; ₹10 / ₹10 / ₹0; ₹6 / ₹6 / ₹0 | ₹5 / ₹5 / ₹0; ₹10 / ₹10 / ₹0; ₹6 / ₹6 / ₹0 |
| Interstate invoice | ₹0 / ₹0 / ₹42 | ₹0 / ₹0 / ₹42 |
| Interstate HSN groups, in line order | ₹0 / ₹0 / ₹10; ₹0 / ₹0 / ₹20; ₹0 / ₹0 / ₹12 | ₹0 / ₹0 / ₹10; ₹0 / ₹0 / ₹20; ₹0 / ₹0 / ₹12 |

Expected and observed in both outputs: ₹400 taxable, ₹42 combined tax, ₹442 grand total. Cess is included in the split, not added to the grand total again. The whole fixture was unchanged: saved sale intact, remaining stock `3 / 4 / 4`, paid ₹100, and customer balance ₹342. The allowlisted invoice exposed no cost or profit and was identical across different private FIFO costs. Manager access was denied before emission. Inexact arithmetic was rejected without emitting rather than rounded.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l06_01` → 6 passed, 0 failed.
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` → 152 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy and build passed; editor diagnostics timed out.

Source trace: [L06-01](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [`gstInfo`](../ALM_Business_Manager.html#L4872), [`gstInvoiceHTML`](../ALM_Business_Manager.html#L4989) and [ordinary Sales actions](../ALM_Business_Manager.html#L6140). Evidence: [captured invoices](../phase2-local/src/confirmation/tests/l06_01.rs) and [HSN splits](../phase2-local/src/confirmation/tests/l06_01_hsn.rs). No operator action was added. Crates, fractional-line rounding, deployed tax treatment, operator reachability, and all acceptance gates remain open.

### Standalone GST bill lifecycle

**P2-GST-STANDALONE / L06-02 (development, PASS):** Offline Rust fixture at `061b19a77157817768b02f80d4ea92fea4d03e95`. Saved synthetic standalone record ID `1`, invoice number `700`, with 2 Cola at ₹140 inclusive (40% GST), 1 Water at ₹105 inclusive (5% GST), and an explicit interstate jurisdiction. The initial form also had an unnamed row at ₹105 (5%), which contributed to form totals but was not saved. Editing changed Cola quantity to 3, replacing the same record. Cancellation kept that record, number, and line data, and the emitted invoice carried the cancelled flag. A subsequent edit of the cancelled record did not write or emit.

| Check | Expected taxable / CGST / SGST / IGST / total | Observed taxable / CGST / SGST / IGST / total |
| --- | --- | --- |
| Initial draft form | ₹400 / ₹45 / ₹45 / ₹0 / ₹490 | ₹400 / ₹45 / ₹45 / ₹0 / ₹490 |
| Saved interstate invoice | ₹300 / ₹0 / ₹0 / ₹85 / ₹385 | ₹300 / ₹0 / ₹0 / ₹85 / ₹385 |
| Edited draft form | ₹400 / ₹62.50 / ₹62.50 / ₹0 / ₹525 | ₹400 / ₹62.50 / ₹62.50 / ₹0 / ₹525 |
| Edited and then cancelled interstate invoice | ₹400 / ₹0 / ₹0 / ₹125 / ₹525 | ₹400 / ₹0 / ₹0 / ₹125 / ₹525 |
| Intrastate control, initial named lines | ₹300 / ₹42.50 / ₹42.50 / ₹0 / ₹385 | ₹300 / ₹42.50 / ₹42.50 / ₹0 / ₹385 |

Missing items, customer, and date respectively returned `Add at least one item with quantity`, `Enter the customer name`, and `Pick the invoice date`. Malformed optional fields returned `GSTIN must be exactly 15 characters` and `E-Way Bill number must be 12 digits`. Rejected writes emitted nothing and preserved the saved record. Manager access was denied for form, save, view, and cancel. Fixture snapshots showed no changes to ordinary sales, stock, payments, or customer balances. A fractional-rupee total was rejected rather than rounded.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l06_02` → 5 passed, 0 failed.
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` → 157 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L06-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [standalone edit and cancellation](../ALM_Business_Manager.html#L5260), [form totals](../ALM_Business_Manager.html#L5735), [save and warning guards](../ALM_Business_Manager.html#L5770), and [saved invoice calculation](../ALM_Business_Manager.html#L4989). Evidence: [lifecycle captures](../phase2-local/src/confirmation/tests/l06_02.rs) and [exactness controls](../phase2-local/src/confirmation/tests/l06_02_edges.rs). The frozen HTML was not executed. Rounding parity, deployed tax rules, production numbering, hosted and device checks, and acceptance gates remain open.

### Purchase order Parse & Fill and explicit Save

**P2-PO-PARSE-SAVE / L07-01 and L07-02 (development, PASS):** Offline synthetic Rust fixture at `5a9539c6124037c73407c88d9a4ae7bd804979cb`. Initial draft: `Original Supplier`, `2024-02-29`, `Keep this note`, and `Prior item` (12 at ₹5.30). The headered tab-separated paste contained an unnamed North row on `2025-02-03` (2 at ₹1.23), followed by South rows for `fixture lime` (3 at ₹2.30, pasted total ₹6.90) and `Fixture Unknown` (4 at ₹1.05, pasted total ₹9.20).

| Check | Expected | Observed |
| --- | --- | --- |
| Draft after Parse & Fill | First party/date even though the first row is skipped; note stays; usable lines replace old lines. | `Synthetic North`, no supplier link, `2025-02-03`, `Keep this note`. `Fixture Lime` linked to product `81`, 3 at ₹2.30; `Fixture Unknown` unlinked, 4 at ₹1.05. The old item was replaced, not appended. |
| Warnings before Save | Unknown product remains in draft; mixed parties stay in one order; flag mismatched total and skipped row. | The unmatched notice named `Fixture Unknown`; the mixed-party notice said `Paste had 2 parties — all rows went into this one PO`. The mismatch also named `Fixture Unknown`; 1 row was skipped. No supplier or order existed after filling. A paste with no usable rows did not change the existing draft. |
| Explicit Owner Save | Store one new supplier and one linked order with both lines and no receipt. | Stored `Synthetic North` and one linked order with the parsed date, note, product links, rates, and ordered quantities. Both received counts were 0; both dated receipt lists were empty. |
| Save with independently omitted fields | Warn and write neither record. | Missing supplier: `Enter supplier`. Missing usable item/quantity: `Add at least one item with an order quantity`. Missing date: `Pick the order date`. Each left suppliers, orders and stock unchanged. |

Stock stayed at 17 in the main fixture. In a separate control, the full inventory catalog (25 on hand) and the prior order's received count and dated receipt (1 on `2025-01-06` at ₹3.30) were unchanged after both Parse & Fill and Save. Manager attempts were denied before parsing or saving.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l07_` → 9 passed, 0 failed.
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` → 166 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L07-01/L07-02](PHASE1_CASES.md#po-parse--fill-not-run), [paste parser](../ALM_Business_Manager.html#L6390), [fill and warnings](../ALM_Business_Manager.html#L6585), and [explicit Save](../ALM_Business_Manager.html#L6713). Evidence: [draft, warnings, and saved records](../phase2-local/tests/l07_purchase_orders.rs) and [prior receipt controls](../phase2-local/src/confirmation/purchase_orders/tests.rs). The frozen HTML was not executed. This covers the typed, positive whole-quantity, headered TSV slice, not general paste parity. Receipt processing, supplier output, hosted and device checks, and acceptance gates remain open.

### Partially received PO outputs

**P2-PO-OUTPUT / L07-03 (development, PASS):** Offline Rust fixture at `9e45ebe485b48243c0a3ef6395f918f432e59212`. Synthetic `PO-042` was supplied explicitly for output, with 4 ordered cases, 1 already received, and a seeded dated receipt on `2025-01-06` at ₹3.30. The saved private purchase rate was ₹3.00. No receipt processing was used to create the fixture.

| Output | Expected | Observed |
| --- | --- | --- |
| Internal View | Partially received; ordered 4, received 1, cancelled 0, remaining 3. | Matched; item-name metadata supplied 12 bottles and ₹67.50 MRP. |
| Supplier Print/PDF sink and image-renderer input | Original 4 cases, product-ID MRP ₹89.50 and pack; no purchase rate, receipt progress, or monetary total. | Both captured the same supplier body with 24 tins & jars, ₹89.50, 4 ordered cases and 4 total cases. The print sink captured `_p` and a 350 ms after-load intent; the renderer input width was 760. Neither exposed the purchase rate or receipt progress. |
| Purchase-rate control | Change only the saved line purchase rate from ₹3.00 to ₹22.22; both complete supplier payloads stay identical. | Both complete captured payloads were identical. |
| Synthetic recipient and fallback intents | Resolve supplier ID/name as in source. Native share rejection is terminal; unsupported share downloads then requests chat only for a valid phone; absent PNG requests encoded text only. | Source lookup precedence and valid/invalid phone cases matched. Native branch captured the `o.png` probe then `Order-PO-042.png` share; rejection did not fall back. Unsupported branch captured download then optional chat. No-PNG branch captured only encoded text with the original 4 cases. |

Manager was denied before all sinks. Whole-fixture snapshots showed the order, dated receipt, and stock unchanged across outputs. The supplied PNG was a valid synthetic image fixture, not a bill rendering.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l07_03` → 7 passed, 0 failed.
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` → 173 passed, 0 failed, 2 reported ignored by the top-level harness. Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L07-03](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [supplier document and WhatsApp](../ALM_Business_Manager.html#L6777), [internal View](../ALM_Business_Manager.html#L6848), and [phone normalization](../ALM_Business_Manager.html#L4819). Evidence: [view and supplier content](../phase2-local/src/confirmation/purchase_orders/tests/output.rs) and [handoff branches](../phase2-local/src/confirmation/purchase_orders/tests/output/whatsapp.rs). Frozen HTML was not executed; no printer, recipient, or URL was invoked. Actual rendering, device handoff, hosted checks, and acceptance gates remain open.

### Partial purchase receiving and input checks

**P2-PO-RECEIVE / L08-01 and L08-02 (development, PASS):** Offline synthetic Rust fixture at `ba326b2a2b5b77f2e629a59cc877d974f4fcec2f`. The PO was dated `2025-01-01` and ordered 5 cases. Opening stock was 10 cases at ₹10.00; the initial PO rate and product buy rate were ₹10.00, with no supplier quote.

| Check | Expected | Observed |
| --- | --- | --- |
| First receipt | `2025-01-20`, 2 at ₹13.00; received/remaining 2/3, Partially Received; on hand 12, average ₹10.50, buy rate and quote ₹13.00. | Stored the expected date, quantity and rate. Counts, status, stock, average, buy rate and quote matched. FIFO had the new Jan 20 layer, and the movement query showed 2 on Jan 20 and 0 on Jan 10. |
| Backdated completion | `2025-01-10`, 3 at ₹9.00; received/remaining 5/0, Completed; on hand 15, average ₹10.20, buy rate and quote ₹9.00. | Stored the expected date, quantity and rate after the first record in the log. Counts, status, stock and costs matched. FIFO traversed opening stock, then Jan 10 (3 at ₹9.00), then Jan 20 (2 at ₹13.00); the movement query showed 3 on Jan 10 and 2 on Jan 20. A follow-on sale control consumed the older dated layer first without changing earlier allocation indices. |
| Independent controls after the first receipt | Zero quantity warns without writing; request 50 caps at 3; blank date uses fixed Jan 25; zero rate is logged without replacing the line rate. | `Enter at least one quantity to receive` with identical fixture; cap stored 3; blank date stored `2025-01-25`; zero-rate receipt stored ₹0.00 while the line rate, buy rate and supplier quote stayed ₹13.00, average stayed ₹10.50, and the FIFO layer used ₹13.00. |

An unmatched item retained its dated movement of 2 without changing the matched inventory or costs. The match control preferred a resolved product ID and fell back to the item name when the ID was absent or unresolved. Manager denial and a later invalid line in a staged batch both preserved the whole fixture.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_` (7 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (180 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L08-01/L08-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [receive handler](../ALM_Business_Manager.html#L7022), [cost and quote](../ALM_Business_Manager.html#L999), [remaining and status](../ALM_Business_Manager.html#L6252), [FIFO layers](../ALM_Business_Manager.html#L1180), and [movement dates](../ALM_Business_Manager.html#L10866). Evidence: [receipt and movement checks](../phase2-local/src/confirmation/purchase_orders/tests/receiving.rs) and [matching and allocation controls](../phase2-local/src/confirmation/purchase_orders/tests/receiving/matching.rs). The frozen HTML was not executed. Same-day ordering across POs and repricing old zero-rate FIFO layers after subsequent rate changes are not established by this slice. Close-remaining, historical reconciliation, hosted and device checks, and all acceptance gates remain open.

### Close Remaining on a partially received PO

**P2-PO-CLOSE / L08-03 (development, PASS):** Offline Rust fixture at `db82a82f14ce21871a97ec626e6ea5493febaae1`. The existing receiving path recorded 2 of 5 ordered cases on `2025-01-20` at ₹13.00 each.

| Action | Expected and observed ordered / received / cancelled / remaining | Status |
| --- | --- | --- |
| Open and dismiss the confirmation | 5 / 2 / 0 / 3; whole fixture unchanged | Partially Received |
| Manager attempts confirmation | 5 / 2 / 0 / 3; `AccessDenied`, whole fixture unchanged | Partially Received |
| Owner confirms | 5 / 2 / 3 / 0; only the line cancelled quantity changed | Completed, not Cancelled |
| Owner repeats confirmation | 5 / 2 / 3 / 0; whole fixture unchanged from first close | Completed |

No receipt or stock movement was created. Before and after confirmation, the dated receipt and movement still showed 2 cases on `2025-01-20`, on-hand stock was 12, the FIFO catalog was unchanged (including the Jan 20 layer of 2 at ₹13.00), average cost was ₹10.50, and the buy rate and supplier quote were ₹13.00.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_close_remaining` (1 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (181 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L08-03](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [Close Remaining confirmation](../ALM_Business_Manager.html#L7007) and [remaining and status](../ALM_Business_Manager.html#L6252). Evidence: [close lifecycle and invariants](../phase2-local/src/confirmation/purchase_orders/tests/receiving/close_remaining.rs). The frozen HTML was not executed. Historical reconciliation, hosted and device checks, and all acceptance gates remain open.

### Pending-only purchase order Edit

**P2-PO-EDIT / L08-04 (development, PASS):** Offline Rust fixture at `45febd397022eae6e0f6391ddd95c794d73d0be0`. On a pending 4-case order, Owner Save changed `Original note` to `Updated note` under the same PO ID. The rest of the fixture was unchanged, including on-hand stock of 25 and receipt history. With an edit copy open, the existing receive path recorded 2 cases on `2025-01-12` at ₹14.00 each. That made the order Partially Received and removed Edit availability. Saving the stale copy returned `OrderNotPending` and left the note, received count of 2, dated receipt, on-hand stock of 27, Jan 12 FIFO layer of 2 at ₹14.00, and Jan 12 movement of 2 unchanged. A competing note save while Pending invalidated the older copy; Manager open and save were denied without mutation.

**Static source-derived comparison, not a legacy walkthrough:** A test-only model rebuilt separate copies of a PO dated `2025-01-10`. The target line, Alpha, began with 5 ordered, 1 received, 1 cancelled, a ₹10.00 line rate, and a dated receipt for 1 at ₹14.00 on `2025-01-12`. Save carries received and cancelled counts from the first exact, case-sensitive item-name match, not from the product ID, and omits the dated receipt list. Expected and observed model values for Alpha:

| Variant | Stored result | On-hand quantity | Inbound movement | FIFO value |
| --- | --- | ---: | --- | ---: |
| Before Save | 1 received, 1 cancelled, receipt retained | 1 | Jan 12: 1 | ₹14.00 |
| Note-only | 1 received, 1 cancelled, receipt list omitted | 1 | Jan 10: 1 | ₹10.00 |
| Case-only rename, same product ID | 0 received, 0 cancelled, receipt list omitted | 0 | None | ₹0.00 |
| Remove Alpha, retain Beta | Alpha absent | 0 | None | ₹0.00 |

The duplicate-name control copied the first matching line's counts for both saved rows, even with different product IDs; reversing the prior rows changed which counts were copied. No operational code calls the source model.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_04` (8 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (189 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L08-04](PHASE1_CASES.md#po-save-and-edit-not-run), [Pending-only Edit](../ALM_Business_Manager.html#L6338), [detached form copy](../ALM_Business_Manager.html#L6521), [Save reconstruction](../ALM_Business_Manager.html#L6735), [quantity](../ALM_Business_Manager.html#L1097), [FIFO fallback](../ALM_Business_Manager.html#L1198), and [movement date fallback](../ALM_Business_Manager.html#L10866). Evidence: [guarded edit tests](../phase2-local/src/confirmation/purchase_orders/tests/note_edit.rs) and [test-only comparison](../phase2-local/src/confirmation/purchase_orders/tests/legacy_edit.rs). The frozen HTML was not executed. Historical reconciliation, legacy parity, and all acceptance gates remain open.

### Delete PO on a disposable copy

**P2-PO-DELETE / L08-05 (development, PASS):** Offline Rust tests at `bde172297f228a9253ea5de859eabfebc243828a`. The local receive path seeded 10 opening cases at ₹10.00, a control PO with 1 case received on `2025-01-02` at ₹10.00, and a target PO with 2 cases received on `2025-01-04` at ₹13.90. Deletion was exercised only in a private test copy of the purchase records; the source fixture and its materialized catalog were not modified.

| Check | Expected | Observed |
| --- | --- | --- |
| Warning, open, decline, and Manager confirmation | Warn about the selected PO's 2 cases, not the combined 3; opening, declining, or Manager denial leaves the fixture unchanged. | Exact received-case warning matched; all three paths preserved the full fixture and copied records. |
| Owner confirmation | Remove only the target PO and its dated receipt; no compensating receipt. | The control PO and its dated receipt survived unchanged and were the only remaining order record. |
| Derived on-hand and movements | On-hand 13 to 11. Jan 4 inbound 2 to 0; Jan 2 inbound stays 1. | Matched. Inbound on both PO document dates, Jan 1 and Jan 3, stayed at 0. |
| Stored costs and supplier quote | No rollback of stored prices. | Average cost ₹10.60, buy rate ₹13.90, and target supplier quote ₹13.90 were carried unchanged in the copied product and cost fields. |
| Unreceived PO control | Show the no-stock-effect warning, then remove the PO without changing stock or inbound movement. | Exact warning matched. On-hand stayed at 10 and inbound stayed at 0 after Owner confirmation. |

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_05` (4 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (193 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L08-05](PHASE1_CASES.md#counter-checkout-edit-and-outputs), [warning and delete callback](../ALM_Business_Manager.html#L6342), [cumulative PO quantities](../ALM_Business_Manager.html#L1097), [dated movement contributions](../ALM_Business_Manager.html#L10866), and [stored cost and quote writes](../ALM_Business_Manager.html#L999). Evidence: [disposable tests](../phase2-local/src/confirmation/purchase_orders/tests/legacy_delete.rs). No operational delete path was added, and the frozen HTML was not executed. Historical reconciliation, legacy parity, and all acceptance gates remain open.

### Purchase order paste formats and mismatch boundaries

**P2-PO-PASTE-FORMATS / L07-04 (development, PASS):** Offline Rust fixture at `829604cc6546fea7779c1095a8c6d680ded87a74`. Reordered headered TSV and headerless positional TSV both replaced the existing draft lines, kept `Keep this note`, selected `North Supply`, and resolved `29-Feb-24` or `29/02/24` to `2024-02-29`. Each produced `Fixture Lime Green` (unique partial match, product `7`, 2 at ₹1,234.56) and `Fixture Lime` (ambiguous between Green and Gold, unlinked, 1 at ₹505.01). One zero-quantity row was skipped. The unmatched-product, total-mismatch, and mixed-party notices were captured before Save. Explicit Owner Save preserved both lines and their links exactly, with zero received and empty receipt lists. Parse alone created no supplier or order; both actions preserved the two stock counts of 12 and 5. Manager was denied both actions.

| Pasted total | Computed total | Difference | Expected and observed mismatch |
| --- | --- | --- | --- |
| ₹100.00 (₹2 floor) | ₹102.00 | ₹2.00 | No |
| ₹100.00 (₹2 floor) | ₹102.01 | ₹2.01 | Yes |
| ₹500.00 (1% = ₹5) | ₹505.00 | ₹5.00 | No |
| ₹500.00 (1% = ₹5) | ₹505.01 | ₹5.01 | Yes |

A date-only header was recognized; an item/quantity-only row did not qualify as a header. Calendar-invalid inputs left the existing typed draft date unchanged without dropping an otherwise usable line. The frozen source normalizes some impossible dates without validating them; that behavior was not reproduced. Formatted money stayed exact in minor units; malformed or out-of-range amounts were skipped by the Rust slice.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l07_04` (7 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (200 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L07-04](PHASE1_CASES.md#po-parse--fill-not-run), [dates and numeric cleanup](../ALM_Business_Manager.html#L6405), [header detection and positional columns](../ALM_Business_Manager.html#L6439), and [product matching and unchanged strict threshold](../ALM_Business_Manager.html#L6493). Evidence: [draft, Save, and boundary tests](../phase2-local/tests/l07_04_purchase_paste.rs). The frozen HTML was not executed. This is development evidence for the tested TSV layouts, not general legacy parser parity; all acceptance gates remain open.

### Receipt-rate edge cases

**P2-PO-RECEIPT-RATE / L08-06 (development, PASS):** Offline Rust tests at `34d3cfec71f4df46e9833e8aed35d7ea7a0361c7`. Each case cloned the same five-case PO after a receipt of 2 cases at ₹13.00 on `2025-01-20`, then attempted to receive 1 case dated `2025-01-10`. Baseline on-hand was 12, average cost ₹10.50, and the PO line rate, last-buy rate, and supplier quote were each ₹13.00. The only dated FIFO layer was the Jan 20 receipt, at ₹13.00 per case.

| Raw rate input | Frozen source expectation (static; runtime NOT RUN) | Observed Rust result |
| --- | --- | --- |
| `0` | Parses as ₹0.00 and logs that rate. Stock becomes 13; average stays ₹10.50, and line rate, last buy, quote and new FIFO lot rate stay ₹13.00. | Typed zero posted 1 on Jan 10 at a recorded rate of ₹0.00. Stock became 13, total received 3, remaining 2. Average stayed ₹10.50; line rate, last buy and quote stayed ₹13.00. The new Jan 10 FIFO layer cost ₹13.00, independently of the zero receipt rate. |
| `-3` | Parses as −₹3.00 and would log that rate. Stock becomes 13; blended average becomes ₹9.46. Line rate, last buy, quote and new FIFO lot rate stay ₹13.00. | The test decoder produced typed −₹3.00; receiving returned `InvalidReceipt`. No new receipt or lot. Whole fixture matched the baseline, including stock, PO rate, movements, and costs. |
| `not a rate` | `+value \|\| 0` would log ₹0.00 and have the same effects as the zero input. | The test-only decoder rejected the text before constructing `Money` or calling receiving. No receipt or lot, and the whole fixture matched the baseline. This does not establish a production form parsing policy. |

The successful zero-rate receipt followed Jan 20 in the receipt log but preceded it in FIFO order; dated movements remained Jan 20: 2 and Jan 10: 1. A Manager request with typed zero returned `AccessDenied` and left the whole fixture unchanged.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_06` (1 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (201 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [raw rate input](../ALM_Business_Manager.html#L6970), [receipt parsing and logging](../ALM_Business_Manager.html#L7031), [average and supplier quote](../ALM_Business_Manager.html#L999), and [FIFO fallback](../ALM_Business_Manager.html#L1198). Evidence: [independent receipt-rate cases](../phase2-local/src/confirmation/purchase_orders/tests/receiving/rate_inputs.rs). No operational receiving behavior changed. Frozen HTML execution, hosted and device checks, legacy parity, and all acceptance gates remain open.

### Receive form on a mixed-status PO

**P2-PO-RECEIVE-FORM / L08-10 (development, PASS):** Offline Rust fixture at `4f6950376c36d40be21a4c08635275f7d36ef1cb`. The existing receive path seeded the mixed-status order for `Synthetic Grove`. The detached form retained the selected PO identity and saved row order, and defaulted its date to the injected `2025-01-20` rather than the PO or receipt date.

| Form row | Ordered / received / remaining | Saved rate | Expected and observed Receive now | Expected and observed quantity / rate editability |
| --- | --- | --- | --- | --- |
| Tangerine | 8 / 3 / 5 | ₹11.25 | 5 | Editable / editable |
| Apple | 2 / 2 / 0 | ₹8.90 | 0 | Disabled / disabled |
| Lime | 4 / 0 / 4 | ₹0.00 | 4 | Editable / editable |

Opening the form, changing local inputs, and discarding it preserved the whole fixture: no new receipt, and no change to stock, costs, or PO progress. Manager requests for existing and missing IDs both returned `AccessDenied` without form data or state changes. In a separate direct helper test, an Owner request for a missing ID returned exactly `PO not found`, with no form data or mutation. That direct call is not an operator action.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l08_10` (4 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (205 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L08-10](PHASE1_CASES.md#L168), [lookup and form defaults](../ALM_Business_Manager.html#L6924), and [field disabling](../ALM_Business_Manager.html#L6970). Evidence: [form projection tests](../phase2-local/src/confirmation/purchase_orders/tests/receive_form.rs). No frozen HTML was executed, and all acceptance gates remain open.

### Product entry and pricing

**P2-PRODUCT-ENTRY / L09-01 (development, PASS):** Offline Rust fixture at `e5d25cf05bbcb73685745bdcdb9cf1ed9db1086c`.

| Check | Expected | Observed |
| --- | --- | --- |
| Brand filter and duplicate add | Exact brand match only; a case- and whitespace-variant of an existing name is rejected even under a different brand. | `Grove` displayed only the soda, `grove` only the juice, and `Gro` neither. The duplicate warning named `Synthetic Soda` and advised using a different name. Filtering and duplicate rejection left the entire fixture unchanged. |
| Buy and margin inputs | Changing buy to ₹12.34 with the existing ₹2.00 margin gives sell ₹14.34; margin ₹2.67 then gives sell ₹15.01. | Both calculated values matched. Save and reopen returned buy ₹12.34, margin ₹2.67 and sell ₹15.01. |
| Sell input | Changing sell to ₹14.25 at buy ₹12.34 gives margin ₹1.91. | Save and reopen returned buy ₹12.34, margin ₹1.91 and sell ₹14.25. A fresh catalog-backed sale review used ₹14.25. On-hand stock stayed 11; stock lots and movement epoch were unchanged. |
| Blank versus zero reorder | Blank uses the current default; explicit zero remains zero. | Reopened values were 5 for the fresh fixture, 13 for the configured default, and 0 for an explicit zero. |
| Blank versus zero tax settings | Blank GST, cess and HSN remain unset and use defaults; explicit zero is retained. | Reopened soda overrides were unset and resolved to GST 18%, cess 0% and HSN `22011010`. Explicit GST and cess zero and HSN `12345678` survived reopen; clearing Mango Juice HSN restored `22029920` while retaining cess 1.25%. |
| Manager boundary | No private product data and no edits. | Manager filter, open, add and save were denied; denied writes left the entire fixture unchanged. |

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l09_01` (11 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (216 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L09-01](PHASE1_CASES.md#L234), [brand filter](../ALM_Business_Manager.html#L7515), [pricing events](../ALM_Business_Manager.html#L7685), [duplicate, defaults and Save](../ALM_Business_Manager.html#L7788), and [GST defaults](../ALM_Business_Manager.html#L4872). Evidence: [entry and reopen tests](../phase2-local/tests/l09_product_entry.rs) and [fixture invariants](../phase2-local/src/confirmation/catalog/product_entry/tests.rs). The frozen HTML was not executed. This covers exact minor-unit synthetic pricing, not general rounding or rendered behavior. OPEN-009, `BillDate`, and both paste-mismatch thresholds are unchanged; rendered, hosted, device, parity, and acceptance gates remain open.

### Supplier price editors and comparison

**P2-SUPPLIER-PRICES / L09-02 and L09-03 (development, PASS):** Offline Rust fixture at `adeb7cd5f0be740af13662654b846e9b3bfce309`, using `Synthetic Orange`. Grove Supply's quote of ₹10.00 was seeded through the existing receipt path. Product and Comparison Editor supplier creation were exercised on independent fixtures.

| Check | Expected and observed |
| --- | --- |
| Comparison before and after a direct quote edit | Grove Supply at ₹10.00 was initially highlighted over Market Supply at ₹12.50. After changing Market Supply to ₹8.75 and reopening, only Market Supply was highlighted. Unquoted Supply stayed blank and unhighlighted. |
| Add through Product Entry | `Orchard Supply` was saved as a supplier and appeared as a blank comparison column immediately. Its ₹12.50 draft quote remained unsaved through a rejected Product Save, then appeared in both reopened editors after a valid Product Save. |
| Add through Comparison | `Market Supply` was saved as a supplier and appeared as a blank column, then retained its quote across comparison reopen and Product Entry reopen. |
| Quote-only effects and role gate | The existing sale (invoice 1, 3 units, ₹60.00 total), stock of 9, and existing PO (5 ordered, dated receipt of 2 at ₹10.00) were unchanged. Average cost stayed ₹8.33 and last buy stayed ₹10.00. Manager access to both editors, supplier creation, and quote writes was denied; saved comparison and directory were unchanged. |

A stored −₹5.00 quote was not eligible for lowest price. Typed blank and zero cleared quotes. The frozen Product Editor publishes draft quotes on Product Save, whereas the frozen comparison editor persists cell changes directly; both can add the supplier before its quote is saved.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l09_supplier_prices` (4 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (220 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L09-02/L09-03](PHASE1_CASES.md#L235), [Product Editor fields and Add Supplier](../ALM_Business_Manager.html#L7725), [Product Editor Save](../ALM_Business_Manager.html#L7782), [Comparison Editor, columns and highlighting](../ALM_Business_Manager.html#L7835), and [Suppliers entry point](../ALM_Business_Manager.html#L10362). Evidence: [cross-editor tests](../phase2-local/tests/l09_supplier_prices.rs). The frozen HTML was not executed. OPEN-009, `BillDate`, and both paste-mismatch thresholds are unchanged. Product rename, deletion with historical links, and all acceptance gates remain open.

### Quoted supplier deletion on a disposable copy

**P2-QUOTED-SUPPLIER-DELETE / L09-03 (development, PASS):** Offline Rust fixture at `371152ad077b81ceef75c4fd9e462aa9d86fe222`. `Synthetic Quoted Supply` had a saved ₹8.75 quote for `Synthetic Orange` and an order dated `2025-01-02`, completed by a dated receipt of 3 cases at ₹13.00 on `2025-01-03`. `Synthetic Control Supply` was an unrelated, unquoted contact. Only the disposable directory and explicit column list modeled deletion; the frozen HTML was not executed and no operational deletion path was added.

| Check | Before confirmation | Observed after Owner confirmation |
| --- | --- | --- |
| Supplier directory | Control and quoted supplier contacts | Only the unchanged control contact remained. |
| Stored product quote | ₹8.75 under the quoted supplier's exact name | Same stored quote and name. |
| Reopened comparison | Control column blank, quoted supplier column ₹8.75 | Retained supplier appeared once as an explicit column, with the same ₹8.75 price and highlight. The control column stayed blank. Both full and selected-product comparisons agreed. |
| Purchase history and stock | Completed PO linked to the quoted supplier, dated receipt for 3 at ₹13.00, stock 13 | PO identity and supplier link, dated receipt, and stock 13 were unchanged. |

The disposable prompt matched the source title and the warning that purchase history and buy rates stay visible. Opening or declining left the copy unchanged. Manager requests to open or confirm deletion and to read the directory or quotes were denied without data disclosure or mutation.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l09_03_supplier_delete` (4 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (224 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L09-03](PHASE1_CASES.md#L259), [delete confirmation and quote-column retention](../ALM_Business_Manager.html#L10466), and [comparison column order](../ALM_Business_Manager.html#L7941). Evidence: [disposable supplier tests](../phase2-local/src/confirmation/purchase_orders/supplier_prices/legacy_delete.rs). OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds were unchanged. Product rename, product deletion with historical links, and all acceptance gates remain open.

### Product rename with historical links

**P2-PRODUCT-RENAME / L09-04 (development, PASS):** Offline Rust fixture at `9caea0077abc14ed2bf08fcd177ecac559b48cbe`. Owner Save renamed product `9` from `Synthetic Peach Soda` to `Synthetic Peach Fizz`. As expected, exact catalog lookup changed from old name → `9`, new name → no match to old name → no match, new name → `9`. Separately, posted invoice `1` retained its stored `Synthetic Peach Soda` label and product `9`, quantity 2, total ₹10.00 and due ₹8.00. The saved PO line retained the old label and product `9`, ordered quantity 5, and its dated receipt of 4 at ₹3.00 on `2025-02-02`. On-hand stayed at 3, FIFO lot balances stayed `[0, 3]`, and allocations were unchanged. Replay returned the original receipt without mutation; a new sale used the new label. Manager denial left the entire fixture unchanged.

**Disposable source model, not an observed legacy run:** Each independent copy renamed `Orange 250ml` to `Orange Glass 250ml`; the stock pairs below are `(opening, fridge)`.

| Legacy stock case | Expected and observed key changes | Expected and observed dependent names |
| --- | --- | --- |
| Destination absent | Old `(17, 4)` was rekeyed to the new name with the same `(101, 7, legacy)` stamp. | Matching Godown and name-only crate names changed; the PID-linked crate component kept the old label and `p9`. |
| Destination occupied | New `(3, 1)` and its `(100, 5, destination)` stamp stayed; old `(17, 4)` was dropped, not merged. | Matching Godown and name-only crate names changed; the PID-linked component stayed unchanged. |
| Exact old-name twin | The twin retained the old name; the cascade left the old key and all dependents unchanged. | Godown and both crate components kept the old label. The primary product still saved under the new name. |

In every modeled case, the primary stamp `(_m, _v, _d)` changed from `(92, 4, catalog)` to `(200, 5, editor)`. Existing ID-keyed stock `(42, 6)` and its stamp were unchanged. Godown, crate, posted bill and PO metadata were not restamped. The modeled stock delta with cursor `100` included the rekeyed row in the empty-destination case, no stock row in the occupied case, and the unchanged old row in the twin case. No deletion marker was emitted. The rekey or removal does not establish a stock transfer or hosted sync behavior.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l09_04` (7 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (231 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy and offline build passed; editor diagnostics timed out.

Source trace: [L09-04](PHASE1_CASES.md#L362), [rename cascade](../ALM_Business_Manager.html#L735), [product Save and stamps](../ALM_Business_Manager.html#L7814), and [stock delta filter](../ALM_Business_Manager.html#L9486). Evidence: [operational history checks](../phase2-local/src/confirmation/l09_04_operational.rs) and [disposable source model](../phase2-local/tests/l09_04_legacy_rename.rs). The frozen HTML was not executed. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds were unchanged. Product deletion, hosted sync, device checks, and all acceptance gates remain open.

### Product deletion with historical links on a disposable copy

**P2-PRODUCT-DELETE / L09-02 and L09-03 (development, PASS):** Offline test-only source model at `d47ce919d08ff162f1018a32f3b05a2a5a9d63a5`. The synthetic catalog contained `p9` (`Synthetic Orange 250ml`) and unrelated control `p10` (`Synthetic Lime 250ml`). Declining as Owner and attempting confirmation as Manager left the whole copy unchanged. The Manager check is local fixture policy, not a claim about the frozen dialog.

| Check | Before | Expected and observed after Owner confirmation |
| --- | --- | --- |
| Catalog and delete marker | `p9` and `p10` resolve; no marker. | Only `p9` was removed. One `products:p9` marker was recorded; `p10` still resolved unchanged. With no exact-name replacement, both linked and name-only bill and PO lookups failed. |
| Stored linked bill | `p9`, `Synthetic Orange 250ml`, quantity 3, MRP 25, rate 20, amount 60, frozen cost-at-sale field 11 (synthetic money units). | All stored fields unchanged. |
| Stored linked PO and receipt | `p9`, same label, 12 ordered and 5 received, line rate 13; receipt `2025-01-02`, quantity 5, rate 14 (synthetic money units). | Line identity, text, quantities, rate and dated receipt unchanged. |
| Stored stock | `p9` opening/fridge `42/6`; `p10` opening/fridge `17/4`. | Both stock keys and values unchanged. No stock was moved. |

Linked history still aggregated under `p9` (3 sold, 5 received). The name-only controls (2 sold, 2 received) aggregated under `n:Synthetic Orange 250ml` without an exact-name master. On a separate copy with exact-name product `p8`, both line lookups resolved to `p8` after deletion, but only name-only history aggregated there. Linked IDs and the `p9` stock entry did not move to `p8`.

- Focused: `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l09_legacy_product_delete` (4 passed, 0 failed).
- Full: `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (235 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [historical references after deletion](PHASE1_CASES.md#historical-references-after-deletion), [product delete confirmation](../ALM_Business_Manager.html#L7769), [catalog lookups](../ALM_Business_Manager.html#L950), [record removal and tombstone](../ALM_Business_Manager.html#L974), [sale quantity keys](../ALM_Business_Manager.html#L1040), and [PO lot keys](../ALM_Business_Manager.html#L1198). Evidence: [disposable product deletion tests](../phase2-local/tests/l09_legacy_product_delete.rs). No operational deletion path was added and the frozen HTML was not executed. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged. Hosted, device, parity, and acceptance gates remain open.

### Mixed-unit crate template checkout

**P2-CRATE-TEMPLATE / L10-01 template portion (development, PASS):** Offline Rust fixture at `f246f04f025a004cc6681307cda9d7fb2755e8b5`. The template copied two distinct products into the existing checkout path.

| Check | Expected and observed |
| --- | --- |
| Suggestion and entered rate | 6 Piece at ₹11 MRP plus 3 Case at ₹73 sell gave a ₹285 suggestion. The saved and copied price was ₹250, distinct from both the suggestion and the ad hoc crate rate of ₹321. |
| Posted bill | One crate row at ₹250 each, quantity 2, total ₹500. The copied component order and quantities were 6 Piece then 3 Case per crate. |
| Stock and frozen FIFO | The Piece product had 4 pieces per pack; 12 pieces took 3 packs, reducing stock from 6 to 3. The Case product took 6 cases, reducing stock from 9 to 3. FIFO froze ₹54 and ₹150 respectively, for ₹204 total; both products' remaining lot balances were `[0, 3]`. |
| Template edit and deletion | A newly prepared draft loaded the edited order of 1 Case then 4 Piece and the edited ₹95 price. After editing and deleting the template, the original sale reopened with identical row contents, price, quantities, receipt, and frozen allocations. Stock and ledger were unchanged. |

The copied row survived SQLite draft reopen and pending confirmation replay; replay returned the original receipt without another stock or ledger effect. Manager template requests were denied and left the fixture and draft unchanged.

- Focused: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l10_01` (2 passed, 0 failed).
- Full: `cargo test --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (239 passed, 0 failed, 2 reported ignored by the top-level harness). Formatting, offline Clippy, and offline build passed; editor diagnostics timed out.

Source trace: [L10-01](PHASE1_CASES.md#L236), [template suggestion, edit and delete](../ALM_Business_Manager.html#L7967), [crate builder](../ALM_Business_Manager.html#L10124), [packet expansion](../ALM_Business_Manager.html#L1039), and [bill snapshot and FIFO](../ALM_Business_Manager.html#L4039). Evidence: [template lifecycle and checkout tests](../phase2-local/src/confirmation/tests/restart/crate_template.rs). Template storage is in-memory; only copied rows and pending approvals were exercised through SQLite. The frozen HTML was not executed. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds were unchanged. Repeated-product valuation, fractional rounding, hosted and device checks, parity, and acceptance gates remain open.

### Owner Stock Lots and shortage valuation

**P2-STOCK-LOTS / L11-01 (development, PASS):** Local in-memory Rust fixture on macOS at `aa8cb6a942d36a9b8bdd8a4f87946950161deb4f`. Before the run, expected rows were calculated from the receipt quantities and rates, consuming the oldest lot and one packet from the next in each four-packet sale. The [fixture observations](../phase2-local/src/confirmation/catalog/l11_01.rs) matched at all stages. Rows below are in displayed projection order, as `receipt day: packets left × unit cost`; money is in minor units and receipt dates are in February 2025.

| Receipt entry order | Before sale: expected = observed | After sale: expected = observed | After cancellation: expected = observed |
| --- | --- | --- | --- |
| Feb 17, then backdated Feb 4 | `4: 3 × 110; 17: 5 × 230` → 8 on hand, value 1,480 | `17: 4 × 230` → 4 on hand, value 920; frozen sale cost 560 | Original rows, 8 on hand, value 1,480 |
| Feb 8, rate 110 then 230 | `8: 3 × 110; 8: 3 × 230` → 6 on hand, value 1,020 | `8: 2 × 230` → 2 on hand, value 460; frozen sale cost 560 | Original rows, 6 on hand, value 1,020 |
| Feb 8, rate 230 then 110 | `8: 3 × 230; 8: 3 × 110` → 6 on hand, value 1,020 | `8: 2 × 110` → 2 on hand, value 220; frozen sale cost 800 | Original rows, 6 on hand, value 1,020 |

There was no intervening stock movement between the sale and cancellation. Whole-fixture equality held around every open. Manager got `AccessDenied` with no rows or costs for known and unknown products; Owner got `ProductNotFound` for the unknown product. Both multiplication and total overflow returned `Overflow` with no state change. An undated opening layer of 2 packets with unknown cost plus 1 receipt packet at 110 returned 3 on hand and unknown total, not a fabricated rate. Empty stock returned no rows and zero value.

**P2-STOCK-LOTS-SHORT / L11-01 (development, PASS on the same revision):** The [Node fixture](../phase2-local/tests/l11_01_shortage.cjs) executed only the extracted, hash-checked [frozen `fifoCostOf` helper](../ALM_Business_Manager.html#L1252) against fresh disposable in-memory synthetic copies, with synthetic dependencies. No browser or application boot ran. With 8 units requested, the expected and observed shortage entries all had `short: true`:

| Fallback condition | Shortfall quantity × rate | Total cost | Unit cost |
| --- | --- | --- | --- |
| Nonzero buy rate takes priority over the last lot | 4 × 31 | 162 | 20.25 |
| Zero buy rate uses the last lot | 4 × 11 | 82 | 10.25 |
| Missing buy rate uses the last lot even when it is exhausted | 8 × 11 | 88 | 11 |
| No buy rate or lot | 8 × 0 | 0 | 0 |

Separately, the [local posting regression](../phase2-local/tests/l11_01_shortage.rs) expected and observed `InsufficientStock` for 4 units against 3 on hand. Stock stayed at 3; sales ledger, received ledger, customer due and posted request count stayed at 0. There was no posted sale and the draft was unchanged. Correcting the same draft to 3 units and reusing request `21` issued invoice `1`, with total 300, received 125, due 175 and stock 0. The helper is not used to authorize or post a sale.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l11_01 -- --nocapture` (7 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (246 passed, 0 failed, 2 ignored).
- `NODE_OPTIONS= node --test /Users/apple/ALM/phase2-local/tests/l11_01_shortage.cjs` (4 passed, 0 failed).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L11-01](PHASE1_CASES.md#L237), [equal-date construction order](PHASE1_CASES.md#equal-date-lot-order), and [legacy Stock Lots dialog](../ALM_Business_Manager.html#L1339). The local view is a typed projection, not a native dialog; it does not reproduce the legacy source or original-quantity columns, mixed historical purchase/PO import ordering, or fractional display rounding. The legacy walkthrough remains **NOT RUN**. No real records, credentials, hosted services or target devices were used. OPEN-009 remains OWNER-BLOCKED; `BillDate`, both paste-mismatch thresholds, and all phase and acceptance gates are unchanged.

### Owner physical stock counts

**P2-PHYSICAL-COUNT / L11-02 (development, PASS):** [Local in-memory Rust tests](../phase2-local/tests/l11_02.rs) on macOS at `3f71adc6b953f945079ad6884dfb5d1e9795374e`. Each case began with independent synthetic stock of 7 billing packets (FIFO layers of 3 at 80 and 4 at 120 minor units), a separate fridge display of 11, and empty adjustment history. Expected deltas were calculated as counted minus billing stock, without the fridge count.

| Submitted count | Billing stock, expected = observed | Fridge display, expected = observed | Adjustment history, expected = observed |
| --- | --- | --- | --- |
| Above: 10 | 10 | 11 | One movement dated 2025-02-12: 7 to 10, delta +3; Count correction and trimmed note `recount at close` |
| Below: 3 | 3 | 11 | One movement dated 2025-02-12: 7 to 3, delta -4; blank note omitted |
| Equal: 7, with and without a fridge edit | 7 in both cases | 13 after edit; otherwise 11 | No movement; FIFO layers unchanged |
| Invalid count, with proposed fridge 12 | 7 unchanged | 11 unchanged | Rejected; no movement or FIFO change |

Invalid controls covered blank, non-finite, negative, fractional, overflowing, and nonnumeric input in each field. Explicit endpoints 0 and 4,294,967,295 produced signed deltas -7 and +4,294,967,288. The increase appended 3 packets at unknown cost; after the decrease, only 3 packets at 120 remained, with FIFO value 360 minor units. A sale demanding 8 with 7 billing packets and 11 displayed in the fridge returned `InsufficientStock`, with no posting or ledger change. Manager reads and submissions returned `AccessDenied` before lookup or validation and left stock, fridge, and history unchanged. Only a nonzero correction made sale cancellation undo stale; equal, fridge-only, and invalid inputs did not. The submission-time control used current stock of 6 rather than the previously displayed 7, recording 6 to 7, delta +1.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l11_02 -- --nocapture` (9 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (255 passed, 0 failed, 2 ignored).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L11-02](PHASE1_CASES.md#L238), [frozen count handler](../ALM_Business_Manager.html#L8184), and [billing stock and fridge projection](../ALM_Business_Manager.html#L1129). The frozen handler rejects blank or non-finite count text, rounds the delta to hundredths against modal-open stock, and records only nonzero deltas. It saves the fridge separately. The fixture accepts whole nonnegative packets within its typed range, validates a supplied fridge count, and uses submission-time stock. Its unknown-cost adjustment layer is not a receipt, a transfer, or a claim of legacy FIFO equivalence. The legacy walkthrough remains **NOT RUN**. Hosted, target-device, phase, and acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Manual Godown count isolation

**P2-MANUAL-GODOWN / L12-01 (development, PASS):** [Local in-memory Rust fixture](../phase2-local/src/confirmation/tests/l12_01.rs) on macOS at `d4fbce40359585913ca3cb2be8080fe287982485`, with one synthetic product. Expected billing quantities were calculated from dated receipts and a posted sale before the edit. The Owner edited the same manual row using different case and surrounding whitespace. Reopening it returned the new count and preserved the `back rack` note.

| Reopened view | Before edit, expected = observed | After edit, expected = observed |
| --- | --- | --- |
| Manual Godown | 3 cases | 8 cases |
| Billing stock | 7 packets | 7 packets |
| Fridge display | 11 packets | 11 packets |
| FIFO lots (money in minor units) | Feb 10: 3 at 80 (value 240); Feb 11: 4 at 120 (value 480); total 720; posted sale cost 160 | Identical dates, quantities, costs, and values |

The [test-only dated register](../phase2-local/src/confirmation/tests/l12_01/dated_register.rs) was recomputed from billing records on both sides of the edit. Each below is in packets; expected rows matched both observations.

| Date | Opening | Received | Dispatched | Closing |
| --- | ---: | ---: | ---: | ---: |
| 2025-02-10 | 0 | 5 | 0 | 5 |
| 2025-02-11 | 5 | 4 | 2 | 7 |
| 2025-02-12 | 7 | 0 | 0 | 7 |

Whole-fixture equality with only the expected manual row changed confirmed that the edit created no receipt, dispatch, adjustment, or transfer. Manager read and save attempts returned `AccessDenied` before validation; each left the entire fixture unchanged. The Owner validation test also covered atomic rejection and saving a manual-only item without a billing product link.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l12_01 -- --nocapture` (3 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (258 passed, 0 failed, 2 ignored).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L12-01](PHASE1_CASES.md#L239), [manual storage list and save handler](../ALM_Business_Manager.html#L10950), and [billing movement computation](../ALM_Business_Manager.html#L10850). Whole-case input is a synthetic fixture restriction, not legacy fractional-input parity. The comparison does not implement L12-02 or resolve undated movement attribution or production location boundaries. The legacy walkthrough remains **NOT RUN**; hosted, device, phase, and acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Owner daily movement register

**P2-MOVEMENT-REGISTER / L12-02 (development, PASS):** [Local in-memory Rust fixture](../phase2-local/src/confirmation/tests/l12_02.rs) on macOS at `7c3ff1d79f8543e26f085f51da243cc295d27815`. Each row was independently calculated from 16 seeded billing packets, dated PO receipts entered out of date order, the posted sale, and signed physical adjustments. Fridge 13 packets and manual Godown 31 cases did not enter the calculation. Values below are packets; expectations matched observations. A dash represents a zero display quantity; display dispatch is shown with a minus sign.

| Date | Sale state | Opening | PO receipts | Sale dispatch | Net adjustment | Display received | Display dispatched | Closing |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2025-02-08 | Active | 16 | 0 | 0 | 0 | — | — | 16 |
| 2025-02-10 | Active | 16 | 5 | 0 | +2 | +7 | — | 23 |
| 2025-02-11 | Active | 23 | 4 | 7 | -3 | +4 | -10 | 17 |
| 2025-02-12 | Active | 17 | 0 | 0 | 0 | — | — | 17 |
| 2025-02-25 | Active | 17 | 0 | 0 | 0 | — | — | 17 |
| 2025-02-11 | Cancelled | 23 | 4 | 0 | -3 | +4 | -3 | 24 |
| 2025-02-12 | Cancelled | 24 | 0 | 0 | 0 | — | — | 24 |

Cancellation removed the 7-packet dispatch on its original bill date and changed the next opening without adding a return. Opening left the entire fixture unchanged; Manager received `AccessDenied`, not rows. The focused suite also checked idle-row omission for offsetting adjustments, overflow, and frozen dispatch quantities after bill edit, checkout unit conversion, and crate expansion.

[PO key controls](../phase2-local/src/confirmation/tests/l12_02/purchase_receipts.rs) retain the frozen register distinction between a name-only line and a present stale ID. Expected and observed name-only rows `(opening, received, closing)` on Feb 10, 11, and 12 were `(10, 3, 13)`, `(13, 5, 18)`, and `(18, 0, 18)` after the current name matched. A stale stored ID was not rekeyed onto the name match: the matched product's Feb 11 and 12 rows were both `(10, 0, 10)`. Local on-hand ended at 15 in both controls. This disagreement is recorded, not resolved as a production attribution policy.

**P2-MOVEMENT-REMAINDER / L12-02 (development, PASS on the same revision):** The [disposable Node fixture](../phase2-local/tests/l12_02_remainder.cjs) executed only the hash-checked frozen helpers against fresh synthetic copies. No operational Rust fallback was enabled. Cumulative received was 5, with 2 supported by a dated Feb 12 log and 3 attributed to the Feb 10 PO date by the source rule. The PO date is **not** evidence of a historical receipt date. Expected and observed rows:

| Selected date | Opening | Received | Dispatched | Closing |
| --- | ---: | ---: | ---: | ---: |
| 2025-02-09 | 0 | 0 | 0 | 0 |
| 2025-02-10 (PO-date fallback) | 0 | 3 | 0 | 3 |
| 2025-02-12 (dated log) | 3 | 2 | 0 | 5 |
| 2025-02-13 | 5 | 0 | 0 | 5 |

Neither quantity was counted twice. The fully logged control received all 5 on Feb 12 and none on the PO date. An undated log put 2 into opening and left 3 for the PO date; without a PO date the remainder of 3 went into opening and the dated receipt of 2 remained on Feb 12. All synthetic inputs were unchanged by the helper reads.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l12_02 -- --nocapture` (8 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (266 passed, 0 failed, 2 ignored).
- `NODE_OPTIONS= node --test /Users/apple/ALM/phase2-local/tests/l12_02_remainder.cjs` (4 passed, 0 failed).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L12-02](PHASE1_CASES.md#L240), [frozen register buckets](../ALM_Business_Manager.html#L10850), [receipt remainder](../ALM_Business_Manager.html#L10866), and [row calculation](../ALM_Business_Manager.html#L11188). This is a typed local projection, not a verified legacy or installed-app display. The legacy walkthrough remains **NOT RUN**. Historical date attribution, production location boundaries, hosted and device checks, and all acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Owner Reorder Planner and Copy List

**P2-REORDER / L13-01 (development, PASS):** [Local synthetic Rust fixture](../phase2-local/src/confirmation/catalog/reorder/l13_01.rs) on macOS at `1ba36d115b50c326ad388ab71fc13e227af4da81`. The expected and observed rows below are in ascending billing on-hand order. The suggested quantity is `max(ceil(3 × reorder level − on-hand), 1)`. Rate and raw estimate are in minor units; display estimates round to rupees.

| Item | On hand / reorder | Suggested packets | Rate source / rate | Raw estimate | Display |
| --- | --- | ---: | --- | ---: | ---: |
| Synthetic Zero | 0 / 0 | 1 | Buy fallback / 151 | 151 | ₹2 |
| Synthetic Lime & Soda | 2 / 5 | 13 | Synthetic Orchard / 835 | 10,855 | ₹109 |
| Synthetic Tied Stock | 2 / 123,458 | 370,372 | Buy fallback / 101 | 37,407,572 | ₹3,74,076 |
| Synthetic Equal | 5 / 5 | 10 | Buy fallback / 2,350 | 23,500 | ₹235 |

Synthetic Above (6 on hand, threshold 5) was excluded. The two equal-stock rows retained insertion order. A lower quote outside the active comparison columns did not determine the minimum; among eligible positive quotes, the minimum beat the higher quote, and stored quote order broke the tie. Zero and negative quotes did not replace the buy-rate fallback. The [buy-rate controls](../phase2-local/src/confirmation/catalog/reorder/l13_01/buy_rate.rs) checked fallback after clearing the positive receipt's quote, explicit manual Buy edits, and a later receipt. At 12 suggested packets the current receipt rate of 1,235 yielded 14,820, and the explicit manual rate of 950 yielded 11,400. The later receipt at 1,475 yielded 16,225 for 11 packets. Receiving preserved the sell rate and refreshed the product margin.

Exact expected and observed Copy List payloads, escaped to show trailing newlines. Each Owner copy called the inert sink once; the nonempty lines matched the planner's order and quantities. The empty planner returned no rows, but copying still emitted its header.

```text
Nonempty: "*ALM Reorder List* (09-Feb-2024)\n\nSynthetic Zero — 1 pkt\nSynthetic Lime & Soda — 13 pkt (Synthetic Orchard ₹8.35)\nSynthetic Tied Stock — 3,70,372 pkt\nSynthetic Equal — 10 pkt\n"
Empty:    "*ALM Reorder List* (09-Feb-2024)\n\n"
```

[New Purchase Order](../phase2-local/src/confirmation/purchase_orders/fresh_form/tests.rs) returned a detached draft dated 2025-03-14, with no supplier, an empty note, and one blank item, quantity and rate. The expected previews `PO-001` with an empty book and `PO-002` with an existing order matched observation; opening did not reserve the number or import suggestions. Fixture equality confirmed that opening, copying, and starting the draft did not create a saved PO, receive stock, or change quotes. Manager requests were denied with no price data and zero sink calls.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l13_01 -- --nocapture` (15 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (281 passed, 0 failed, 2 ignored).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L13-01](PHASE1_CASES.md#L241), [Reorder projection, `suggestQty` and Copy List](../ALM_Business_Manager.html#L8308), [fresh PO defaults](../ALM_Business_Manager.html#L6521), and [positive receipt buy-rate update](../ALM_Business_Manager.html#L1013). These are local development results for whole-packet fixtures; the real clipboard and legacy walkthrough were **NOT RUN**. Hosted, device, parity, and acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Order Planner checkout and explicit PO Save

**P2-ORDER-PLANNER / L13-02 (development, PASS for scoped whole-packet flow):** [Local synthetic Rust tests](../phase2-local/tests/l13_02_order_planner.rs) at `abc5459f60b633e0009c0b977d7ed9fb4dccd835` on macOS. The frozen `orderRate` handler selects the minimum positive quote from live supplier columns, or the buy rate with no supplier attribution if none qualifies. The PO supplier is inferred from the whole cart; editing the supplier in the detached PO form does not reprice the captured lines.

| Main case, amounts in minor units | Expected and observed cart, checkout draft, and saved PO |
| --- | --- |
| Product 1, Synthetic Lime | Supplier A quote 825 beat B quote 1,100; 3 packets × 825 = 2,475 at all stages. |
| Product 2, Synthetic Soda | A's quote was -100 and B's absent, so no eligible quote; 4 packets × buy rate 1,325 = 5,300 at all stages. |
| Supplier and total | Cart attributed Lime to A and had no attribution for Soda. Checkout prefilled A. Owner selected B on the detached draft and explicitly saved it under B; total stayed 7,775 throughout. |

The draft dated 2024-02-09 saved nothing on checkout. Explicit Save created one Pending PO (`PO-001`) with 3 and 4 packets remaining and no receipt entries, then cleared the saved lines from the cart. Billing stock stayed at 2 and 4; FIFO lots, buy rates, and quotes were unchanged. Closing without saving preserved the cart and order book. Manager plan, checkout, and Save attempts returned `AccessDenied` with no price data and left the entire fixture and cart unchanged.

[Controls](../phase2-local/tests/l13_02_order_planner/controls.rs): When only B quoted Soda, checkout inferred B and kept its rate of 975, not the buy rate, when the Owner saved under A; 4 × 975 = 3,900. With both positive lines attributed to different suppliers, checkout retained both lines and the 6,375 total but left supplier blank; Save returned `MissingSupplier` without changes. Setting Soda to zero omitted it from the draft but still prevented supplier inference; removing it restored A and the Lime-only total of 2,475. Successful Save retained cart entries omitted from the saved PO.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml l13_02 -- --nocapture` (8 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (289 passed, 0 failed, 2 ignored).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [L13-02](PHASE1_CASES.md#L242), [planner rate](../ALM_Business_Manager.html#L10523), [checkout](../ALM_Business_Manager.html#L10546), and [explicit PO Save](../ALM_Business_Manager.html#L6713). Fractional quantities and group Add all remain outside this implementation; no permanent legacy departure is approved. The frozen HTML was unchanged and the legacy walkthrough was **NOT RUN**. Hosted, device, parity, and acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Order Planner Group Add All and Copy Plan

**P2-ORDER-PLAN-ACTIONS / L13-02 group action and L13-03 (development, PASS):** Synthetic local Rust fixture on macOS at `05ad1101403d7fdda78fc28c71e8639d97b2b811`. [Group Add All](../phase2-local/tests/l13_02_group_add_all.rs) received a supplied list of visible members. Expected and observed cart order was Existing (17 packets, unchanged), First (8 suggested packets), Visible High (9 packets at 475 paise from Group Fixture Supplier), and Last (6 packets). The supplied above-threshold member was included; the omitted hidden member and unknown ID were not. Repeating the full action left the cart identical. Manager denial did not change the cart, quotes, stock, or order book.

[Copy Plan](../phase2-local/src/confirmation/purchase_orders/order_planner/copy/tests.rs) emitted the exact expected payloads below. The nonempty cart kept the zero line and used its captured supplier rates; the buy-rate fallback had no supplier suffix. Its two nonzero line amounts of 149 paise each sum to 298 paise, displayed as ₹3 rather than the ₹2 sum of individually rounded lines. The empty cart read current catalog rows at or below threshold, independently of a restricted visible subset containing only Synthetic Lime & Soda. It excluded the above-threshold item and emitted no total. Backslash escapes below show the actual newline placement.

```text
Cart: "*ALM Order Plan* (09-Feb-2024)\n\nSynthetic Lime & Soda — 1 pkt (Synthetic Orchard ₹1.49)\nSynthetic Above — 0 pkt (Synthetic Orchard ₹1,23,456.78)\nSynthetic Equal — 1 pkt\n\nTotal ₹3"
Empty cart: "*ALM Order Plan* (09-Feb-2024)\n\nSynthetic Equal — 10 pkt\nSynthetic Lime & Soda — 13 pkt (Synthetic Orchard ₹8.45)\nSynthetic New Low — 1,23,456 pkt\n"
Rejected, no low rows: "*ALM Order Plan* (09-Feb-2024)\n\n"
Numeric keys: "*ALM Order Plan* (09-Feb-2024)\n\n0 — 1 pkt\n2 — 1 pkt\n10 — 1 pkt\n4294967294 — 1 pkt\nZed — 1 pkt\n4294967295 — 1 pkt\n01 — 1 pkt\n+1 — 1 pkt\n4294967296 — 1 pkt\nAlpha — 1 pkt\n\nTotal ₹10"
```

Expected and observed feedback: `Order plan copied ✓` after one accepted write, `Copy failed` after one rejected write, and `Copy not available in this browser` when no writer was supplied. Manager received `AccessDenied`, no price data, and zero writer calls. Copy left the complete fixture and plan unchanged; neither action saved a PO, received stock, or changed quotes. The display-only Node arithmetic control `50 * 0.29` yielded `14.499999999999998`, which rounds to ₹14 in the frozen calculation; the Rust payload matched, without changing exact cart arithmetic. Additional tests cover numeric key order, signed half ties, selected large values, and overflow without a partial write; they do not establish exhaustive numeric parity.

Commands run from `/Users/apple/ALM` on the recorded revision:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l13_02_group_add_all -- --nocapture` (2 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l13_03 -- --nocapture` (11 passed, 0 failed).
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (302 passed, 0 failed, 2 ignored).
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check`, `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings`, and `cargo build --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml` passed. Editor diagnostics timed out.

Source trace: [group preservation](PHASE1_CASES.md#L622), [Copy Plan cases](PHASE1_CASES.md#L627), [frozen Group Add All handler](../ALM_Business_Manager.html#L10721), and [frozen Copy Plan handler](../ALM_Business_Manager.html#L10821). Installed UI group selection, fractional-quantity parity, actual clipboard behavior, the legacy walkthrough, and all acceptance gates remain open. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds are unchanged.

### Local terminal provisional-draft demo

**DEV-DRAFT-DEMO / R-6 (development, PASS):** [Terminal launcher](../phase2-local/src/bin/alm-draft-demo.rs) and [two-process test](../phase2-local/tests/draft_demo.rs) at `3292b202f6d217b4812f8ea5ece1423fc0d9117a`, on the development Mac. The Owner approved this synthetic demo only. It uses the existing Rust `Role::Manager` session and SQLite draft store, not a real login or an Owner confirmation review. It exposes no posting, invoice, payment, stock reservation, or goods-release action.

**Owner replay:** Use Cargo and `sqlite3` on `PATH`. In the same terminal, run the setup once to make a fresh, unsynced directory under the ignored local build directory. Never point the demo at a real or shared database; use one demo process at a time.

```sh
cd /Users/apple/ALM && mkdir -p /Users/apple/ALM/phase2-local/target && db_dir=$(mktemp -d /Users/apple/ALM/phase2-local/target/alm-demo.XXXXXX)
```

Run this launcher for both sessions, keeping the same `db_dir`:

```sh
cd /Users/apple/ALM && cargo run --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --bin alm-draft-demo -- --db "$db_dir/drafts.sqlite"
```

1. Enter `new`, `Synthetic Customer`, `1` (Synthetic Lime), `3`, `review`, `save`, and `quit`, one entry per line. Note the saved draft ID.
2. Run the same launcher again. Enter `open`, the saved draft ID (`1` in a fresh database), `review`, and `quit`.

**Expected and observed:** The reopened draft retained its ID, customer, item, and quantity. `SAVED` indicates SQLite persistence only; both reviews showed `PROVISIONAL — NOT POSTED`. Selected transcript from the actual launcher replay (paths and repeated prompts omitted):

```text
Launch 1
ALM synthetic Manager demo — local drafts only; not authenticated.
PROVISIONAL — NOT POSTED
Draft ID: 1
State: UNSAVED
Customer: Synthetic Customer
Item: Synthetic Lime | Quantity: 3
Saved draft 1: PROVISIONAL — NOT POSTED.

Launch 2
ALM synthetic Manager demo — local drafts only; not authenticated.
Opened draft 1: SAVED.
PROVISIONAL — NOT POSTED
Draft ID: 1
State: SAVED
Customer: Synthetic Customer
Item: Synthetic Lime | Quantity: 3
```

The automated restart assertion checked distinct CLI process IDs. An attempted `post` was rejected as an unknown command. A read-only SQLite check of the launcher replay found 1 provisional draft and 0 pending owner confirmations. No real records were used.

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test draft_demo -- --nocapture`: 2 passed, 0 failed.
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`: 305 passed, 0 failed, 2 ignored.
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check` and `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --bin alm-draft-demo --test draft_demo -- -D warnings` passed. The launcher built and ran offline in both sessions. Editor diagnostics timed out.

Owner testing is pending. This is terminal development evidence, not visual parity, a verified legacy walkthrough, or phase acceptance. No hosted, device, migration, or production gate changed. OPEN-009 remains OWNER-BLOCKED; `BillDate` and both paste-mismatch thresholds remain unchanged.

### PC-002 milestone A: Draft to reviewed bill

| Item | Disposition and commit | Run ID |
| --- | --- | --- |
| D1 | Contract verified at `a7f9def` (PC-002 / DEC-022). Standing approval covers reversible synthetic local work; per-case approval and optional demo-feedback waits are no longer prerequisites. No feedback file was created or awaited. | `PC002-CONTRACT` (static) |
| D2 | Runnable local kickoff verified: shared pricing/GST `4f733ef`, Manager preview `2a624e4`, separate Owner simulation `1bdbfed`. | `A-MANAGER-REPLAY`, `A-OWNER-SIM` (executed development) |
| D3 | Consolidated checklist verified, not satisfied as acceptance. The approved plan, prior evidence, and blocked date work were unchanged by the milestone implementation at `1bdbfed`. | `PC002-CONTRACT` (static), `A-OFFLINE` (executed development) |

**PC002-CONTRACT:** `git apply --reverse --check /Users/apple/ALM/.bridge/plan/changes/PC-002.diff` confirmed the approved patch was present; `git diff --exit-code a7f9def HEAD -- /Users/apple/ALM/{CLAUDE.md,bridge.toml,docs,phase2-local/src/bill_date.rs,phase2-local/src/confirmation/purchase_orders/paste.rs}` passed before appending this entry. [OPEN.md](OPEN.md#consolidated-owner-checklist) retains the separate owner decisions and prerequisites. The replan's Done when checklist holds: routine stops are removed, milestone A has a runnable kickoff, and historical evidence, phase status, and acceptance criteria are intact. No owner replay or approval of the result is asserted.

**A-MANAGER-REPLAY (development, PASS; R-6, L03-02 and L06-01 calculation slices):** On the development Mac at `1bdbfed`, used the [DEV-DRAFT-DEMO replay commands](#local-terminal-provisional-draft-demo) with a fresh disposable SQLite database. Both launches used the same database. After reopening, `review` displayed the existing draft and the new preview. The fixed synthetic catalog uses an inclusive unit price of ₹118.00 at 18% intra-state GST. The independent expectation was 3 × ₹118.00 = ₹354.00; ₹354.00 ÷ 1.18 = ₹300.00 taxable; the ₹54.00 difference splits into ₹27.00 each. The observed second-launch transcript matched:

```text
Opened draft 1: SAVED.
PROVISIONAL — NOT POSTED
Draft ID: 1
State: SAVED
Customer: Synthetic Customer
Item: Synthetic Lime | Quantity: 3
Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹354.00
Taxable: ₹300.00
CGST: ₹27.00
SGST: ₹27.00
Inclusive total: ₹354.00
```

The [two-process test](../phase2-local/tests/draft_demo.rs) confirmed distinct processes, unchanged SQLite logical contents and `data_version` across reopen/review, and 0 pending confirmations. The Manager output contained no cost or profit fields; `post` was rejected. The launcher has no invoice, payment, reservation, release, or posting path. The [shared GST calculation](../phase2-local/src/bill_math.rs) follows the inclusive-tax source rules for this exact fixture; this is not a legacy runtime or broader rounding comparison.

**A-OWNER-SIM (development, PASS; R-7 local test double):** The [separate test](../phase2-local/tests/owner_simulation.rs) used a detached copy of the same CLI draft shape reopened from its disposable SQLite file. The unit rate changed from 11,800 to 14,160 minor units. Stale confirmation and Manager confirmation were rejected; fresh review left stock at 7, with no sale or financial effects. Explicit Owner confirmation applied one simulated sale before the test dropped its reply. Expected and observed: stock fell to 4, FIFO value moved from 47,000 to 28,000, sales ledger became 42,480, received ledger 20,000, due 22,480. Identical retry returned the original simulated receipt without another effect. Conflicting reuse returned `RequestConflict`; the SQLite draft remained unchanged. No final invoice was issued by the launcher.

**A-OFFLINE (development):** Commands run from `/Users/apple/ALM` at `1bdbfed`:

- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test draft_demo --test owner_simulation -- --nocapture`: 3 passed, 0 failed.
- `cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet`: 313 passed, 0 failed, 2 ignored.
- `cargo fmt --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml -- --check` and `cargo clippy --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --all-targets -- -D warnings` passed. The launcher also built and ran offline; editor diagnostics timed out.

The contained legacy walkthrough, hosted integration, actual device/parity, accounting sign-off, migration, and cutover were not run and remain gated. Phase 1 remains open; no phase exit or acceptance is recorded. OPEN-009 remains OWNER-BLOCKED. Local milestone work may continue without optional owner feedback.

### Milestone B: Purchasing to stock

**B-PURCHASE-REPLAY (development, PASS):** [One local Rust replay](../phase2-local/tests/milestone_b_replay.rs), with its [purchasing checkpoints](../phase2-local/tests/milestone_b_replay/purchasing.rs), ran on the development Mac at `14fbc3a24dbc9b5b6ad1d0ed6c6fae39e1b29a51`. All quantities below are billing packets unless noted; amounts are minor units. A and B refer to the two synthetic products.

| Checkpoint, February 2025 | Expected = observed (A / B) | Case disposition |
| --- | --- | --- |
| Feb 10, plan and explicit PO Save | Winning quotes 100 / 180 beat 160 / 260. Reorder levels 4 / 3 at zero stock suggested 12 / 9; planned value was 2,820. Checkout saved nothing. Save created one Pending PO with 12 / 9 outstanding, no receipts, no register movement, and zero on-hand and FIFO value. | [L09-02](PHASE1_CASES.md#L235), [L13-02](PHASE1_CASES.md#L242), [L07-02](PHASE1_CASES.md#L159): quote selection and save slices passed. |
| Feb 12 receipt, entered first | Received 5 at 130 / 4 at 210. On hand 5 / 4; outstanding 7 / 5; FIFO value 1,490. | [L08-01](PHASE1_CASES.md#L161): dated partial receipt slice passed. |
| Feb 11 receipt, entered second | Received 3 at 110 / 1 at 190. Cumulative received and on hand 8 / 5; outstanding 4 / 4; FIFO value 2,010. Both receipt dates, quantities, and rates were retained. PO remained Partially Received. | [L08-01](PHASE1_CASES.md#L161), [L11-01](PHASE1_CASES.md#L237): backdated receipt and FIFO ordering slices passed. |
| Feb 13, simulated dispatch | Two crates loaded from a saved template used 2 Case of A and 6 Piece of B per crate, with 6 pieces per packet for B. Dispatch consumed 4 / 2 packets, exhausting the Feb 11 layers first. Only Feb 12 lots remained: 4 at 130 / 3 at 210, worth 520 / 630. PO outstanding stayed 4 / 4. | [L10-01](PHASE1_CASES.md#L236), [L11-01](PHASE1_CASES.md#L237): mixed-unit dispatch and remaining valuation slices passed. |

**Valuation reconciliation:** FIFO value received, 2,010, equaled the value removed from inventory, 860 (`3×110 + 1×130 + 1×190 + 1×210`), plus 1,150 remaining (`4×130 + 3×210`). The replay compared the dated lot balances and remaining valuation; it did not read a stored sale-cost field.

**Movement register:** Expected and observed tuples `(opening, received, dispatched, closing)` matched; adjustments were zero.

| Date | A | B |
| --- | --- | --- |
| Feb 10, PO date | No row | No row |
| Feb 11 | (0, 3, 0, 3) | (0, 1, 0, 1) |
| Feb 12 | (3, 5, 0, 8) | (1, 4, 0, 5) |
| Feb 13 | (8, 0, 4, 4) | (5, 0, 2, 3) |
| Feb 14 | (4, 0, 0, 4) | (3, 0, 0, 3) |

The manual Godown row for A reopened at 31 cases, then at 47 after the edit. Billing stock, FIFO, the register, and PO progress stayed unchanged. [L12-01](PHASE1_CASES.md#L239) manual isolation and [L12-02](PHASE1_CASES.md#L240) dated register slices passed. The tested Manager actions were denied at the fixture boundary.

- **B-PURCHASE-REPLAY:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_b_replay -- --nocapture` (1 passed, 0 failed).
- **B-OFFLINE:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (314 passed, 0 failed, 2 ignored). All-target offline Clippy with `-D warnings` and formatting also passed.
- **Blocked:** OPEN-009 remains OWNER-BLOCKED. LSP diagnostics timed out; no analyzer result is claimed.
- **Not run by this replay:** PO completion, template edit/delete, sale cancellation or shortage, production location/date attribution, undated receipt fallback, or restart of the in-memory purchasing fixture. The contained legacy walkthrough, hosted and device checks, migration, and cutover were not run. No acceptance or phase exit is claimed. `BillDate`, both paste-mismatch thresholds, and the approved contract and gates were unchanged.

### Milestone C: Bill history to accounts

**C-BILL-REPLAY (development, PASS):** [Local Rust integration replay](../phase2-local/tests/milestone_c_replay.rs) on the development Mac at `aa220fad0d85c3ca50c71919b0054b3af2a3a031` (account projections `fcc273a`). Reused A's explicit synthetic Owner confirmation pattern and B's reverse-entry dated PO receipts. All amounts are minor units; A/B stock is in billing packets. Independent expected values matched the observed values below for synthetic invoice 1, dated 2025-02-21.

| Checkpoint | Saved invoice total / bill paid | GST taxable / tax | Active sales / bill-paid ledger | Udhaar closing balance | Cash Book Feb 21 Cash / Mar 1 Online | FIFO A / B; remaining value |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Before confirmation | No invoice | No invoice | 0 / 0 | 0 (customer ledger not yet available) | 0 / 0 | 4 / 2; 32,000 |
| Posted | 23,600 / 10,000 | 20,000 / 3,600 | 23,600 / 10,000 | 13,600 | 10,000 / 0 | 2 / 2; 19,000 |
| Separate Online receipt | 23,600 / 10,000 | 20,000 / 3,600 | 23,600 / 10,000 | 8,600 | 10,000 / 5,000 | 2 / 2; 19,000 |
| Edited | 41,000 / 10,000 | 35,000 / 6,000 | 41,000 / 10,000 | 26,000 | 10,000 / 5,000 | 2 / 1; 16,500 |
| Cancelled | 41,000 / 10,000 (archived) | 35,000 / 6,000 (marked cancelled) | 0 / 0 | -5,000 advance | 0 / 5,000 | 4 / 2; 32,000 |

The original two A packets at 11,800 consumed A's Feb 19 lot at 6,000 and one from the Feb 20 lot at 7,000. Editing repriced the retained line to 17,700 per packet and appended one B at 5,600; the edit consumed one B from Feb 20 at 2,500, without consuming A again. Cancellation restored every original lot and left the separate 5,000 receipt as the only live customer credit. Sales CSV retained both edited lines marked `CANCELLED` with the original invoice identity and date. GST kept the saved document and cancellation marker: original CGST/SGST 1,800 each; edited and cancelled 3,000 each, with the appended product in its own HSN group.

Estimate As on bill date/Latest was 13,600/13,600 after posting, 13,600/8,600 after the separate receipt, 31,000/26,000 after edit, and 31,000/-5,000 after cancellation. The estimate's Print and Save as PDF intents were captured in memory only. Manager calls to restricted history/CSV, profit, GST, estimate, account and Cash Book methods returned typed denials, with no restricted output; denied writes preserved the compared Owner readbacks. Trusted fixture observers are not exposed by the provisional Manager launcher. These checks do not attest to hosted authentication or staff-device data isolation.

**Case dispositions:** [L03-02, L04-01, L04-02, L05-01 and L05-02](PHASE1_CASES.md#L137): ordinary part-paid bill, current invoice readback, edit, cancel, and inert estimate slices passed. [L06-01](PHASE1_CASES.md#L145): ordinary intra-state GST renderer slice passed. [L11-01](PHASE1_CASES.md#L237): dated depletion and restoration passed. [L14-01](PHASE1_CASES.md#L243): separate receipt and customer ledger slice passed. [L15-01](PHASE1_CASES.md#L245): dated Cash Book system comparison slice passed. No complete legacy case or phase exit is asserted.

- **C-BILL-REPLAY:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_c_replay -- --nocapture` (1 passed, 0 failed).
- **C-ACCOUNTS:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml customer_accounts` (8 passed, 0 failed), including backdated receipt ordering, signed rounding, and denial/invalid/overflow atomicity.
- **C-OFFLINE:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (323 passed, 0 failed, 2 ignored). Formatting, all-target offline Clippy with `-D warnings`, and offline build passed.
- **Blocked:** OPEN-009 remains OWNER-BLOCKED. LSP diagnostics timed out; no analyzer pass is claimed. `BillDate` and both paste-mismatch thresholds were unchanged.
- **Not run by this replay:** Online payment made on a bill (the synthetic ordinary bill path is Cash), manual Cash Book entry and monthly reports, full date-filtered Sales history, cancel undo, actual print/PDF/share, and broad rounding parity. The legacy walkthrough, hosted, device, accounting acceptance, migration, and cutover checks remain unrun and gated.

### Milestone D: Outputs, settings and recovery

**D-OUTPUT-RECOVERY (development, scoped PASS):** [Joined replay](../phase2-local/tests/milestone_d_replay.rs) on the development Mac at `b6dd68f42bb0abf2b8434c084154230585f91bd2`. Reused C's cancelled invoice, dated PO receipts and customer credit. The [coverage crosswalk](MILESTONE_D_COVERAGE.md) maps 118 explicit inventory IDs with no missing or duplicate rows; unexercised variants retain their unrun or blocked disposition.

```sh
cd /Users/apple/ALM && ALM_REPLAY_QUICKLOOK_PNG=1 cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_replay --quiet -- --nocapture
```

The optional raster command uses the development Mac's Quick Look and `sips` and retains artifacts under the printed report directory. The [observed artifact directory](../phase2-local/target/milestone-d-replay-1795244138172422000-0/) contains the [SVG](../phase2-local/target/milestone-d-replay-1795244138172422000-0/synthetic-cancelled-estimate.svg), [PNG](../phase2-local/target/milestone-d-replay-1795244138172422000-0/synthetic-cancelled-estimate.svg.png), [print HTML](../phase2-local/target/milestone-d-replay-1795244138172422000-0/synthetic-estimate-print.html), [Sales CSV](../phase2-local/target/milestone-d-replay-1795244138172422000-0/sales.csv), [backup](../phase2-local/target/milestone-d-replay-1795244138172422000-0/d-read-model-v2.sqlite) and [restored store](../phase2-local/target/milestone-d-replay-1795244138172422000-0/d-read-model-restored.sqlite). These are disposable synthetic files, not source-controlled artifacts; the command regenerates them.

| Case slice | Expected versus observed development result |
| --- | --- |
| L05-01 to L05-05: estimate output and routing | Compared frozen source anchors to the generated SVG's header and cancellation marker, identity, table column order and summary/footer. Invoice 1 dated 2025-02-21 showed Synthetic Buyer, amounts ₹354.00 and ₹56.00, total ₹410.00, paid ₹100.00 and Latest balance ₹-50.00. Quick Look generated a 1440 × 940 PNG, SHA-256 `e9d11d636b5c236b4a02604f9ac69884434fa50e2fc60c1107e3c54ccf616969`; the inert file sink received the same bytes. The SVG is a reconstruction, not a frozen-app render; print HTML layout and number formatting differ. Print and Save as PDF were captured intents with shared HTML, not actual print/PDF files. Share/download and text fallbacks were inert; Sales-row UI and actual device handoff were not run. |
| L13-01, L13-03 and L19-03: clipboard and export | Clipboard callbacks captured Reorder and Copy Plan text; no system clipboard was accessed. Sales CSV retained the two cancelled lines with bill totals and paid once. The [restored CSV](../phase2-local/target/milestone-d-replay-1795244138172422000-0/d-restored-sales.csv) came from archived output bytes, not from rerunning the exporter against structured records. It matched the original byte for byte, SHA-256 `7538b74ca2fadad140d7dadfb2e799d78a335a89f52959bf7e6dbcc20c3000bc`. SpreadsheetML and full export parity were not run. |
| L01-07, L18-01 and L19-01: synthetic settings and recovery comparison | Backed up the versioned SQLite **read model**, restored to a fresh file without reseeding, and reopened through a separate read-only SQLite connection. Sale and GST projections, PO/receipt records, Cash Book, customer credit, FIFO and synthetic profile/control readbacks matched before backup and after reopen. The test control still showed the synthetic business profile and `online` preference. Denied, malformed, wrong-version and existing-destination restore controls did not publish or replace a store. This is not application settings persistence, full application-state rehydration, or a legacy JSON/browser/folder restore. |
| R-5 local fixture boundary | Inspected the separately reopened Manager provisional draft for forbidden cost, FIFO, profit, quote and balance fields or markers. Restricted document/export/share methods returned denial without callbacks; archive denials exercised the test helper, not OS file permissions. Staff-device/cache isolation and hosted authorization remain unverified. |

- **D-OUTPUT-RECOVERY:** Opt-in command above, 2 passed, 0 failed. The default offline path uses a sentinel only for image routing and does not perform rasterization.
- **D-OFFLINE:** `cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet` (325 passed, 0 failed, 2 ignored). Formatting, offline all-target Clippy with `-D warnings`, and offline build passed. Editor diagnostics timed out.
- **Remaining:** Application backup/restore and settings, remaining exports and output branches, actual WhatsApp/print/PDF/clipboard delivery, controlled legacy walkthrough, and installed-device parity remain unrun. Hosted, protected-data, accounting, migration and cutover gates remain open. OPEN-009 remains OWNER-BLOCKED. C's Online bill payment and manual Cash Book entry dispositions were not changed.

#### Structured Sales exports and fresh recovery

**D-STRUCTURED-190ee03 (development, scoped PASS).** Tested `190ee03030d6d6bff7ffe0340adba381cba8f0fa` on the local development Mac using synthetic data only. The [replay](../phase2-local/tests/milestone_d_structured.rs) reused C's cancelled two-line bill and separate customer receipt. It added an earlier active control bill to distinguish stored order from date order. Expectations come from [L19-03](PHASE1_CASES.md#sales-export-projections-not-run) and the frozen source, not a legacy runtime run.

```sh
cd /Users/apple/ALM && ALM_STRUCTURED_RETAIN=1 cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test milestone_d_setting --test milestone_d_replay --test l09_product_entry -- --nocapture
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

| Case slice | Expected versus observed (development) |
| --- | --- |
| L19-03 Sales CSV | Exact rows matched: active invoice 99 dated 2025-01-01 appeared before cancelled invoice 1 dated 2025-02-21, with `Mode` and `Pack`. Cancelled lines showed ₹354 and ₹56; ₹410 total and ₹100 paid appeared only on the first line. On the active bill the stored quantities were 6 pieces (pack size 12) and 2 crates, with active status blank and no component expansion. |
| L19-03 SpreadsheetML | Four named sheets: Products, Customers, Sales and Stock. Sales kept stored order (cancelled before active), omitted `Mode` and `Pack`, and matched the checked numeric cells, quantities, blank later-line totals and paid, and status markers. Neither sales projection included synthetic sale IDs, crate component details, or the standalone receipt. Opening the workbook in Excel was not run. |
| L19-03 full supported JSON | Parsed versioned JSON retained both synthetic sale IDs and nested lines, the crate component quantity of 3, and C's standalone Online receipt dated 2025-03-01 for ₹50 (synthetic receipt ID). This is a complete snapshot of the supported local read model: products with stock fields, customers, sales, receipts and reorder default. It is not the complete legacy `DB` format or evidence that POs, FIFO lots, ledger history, or unsupported settings can be restored through it. |
| L18-01 / L09-01 saved setting | Saved the [source-backed reorder default](../ALM_Business_Manager.html#L9092) through the Owner-only local Rust service, from 5 to 13. The blank-reorder product save and reopen showed 13 both before backup and after restore through the [product-entry behavior](../ALM_Business_Manager.html#L7809). No installed Settings UI or legacy browser controls ran. |
| L01-07 / L19-01 recovery and R-5 local guard | Dropped the source store handle, restored from the saved JSON backup file into an absent destination, and opened the restored file separately without reseeding its records. Regenerated both exports from structured records; each matched its pre-backup bytes. Manager received no CSV, workbook, or backup bytes and could not save the setting or restore. Manager denial, malformed JSON, unsupported version, and invalid date left the destination absent; an existing destination rejected restore. These are local service checks, not OS isolation or hosted authorization. Neither this JSON store nor the earlier SQLite read-model archive is authoritative for posted sales. |

Artifacts: [structured-d-61325-1795245908991209000](../phase2-local/target/structured-d-61325-1795245908991209000/). The source and restored CSVs share SHA-256 `91ab9449a55a5ed65c59fc8ca316e2b0f5810e50336afc96e91dc0435cf55c85`; the SpreadsheetML pair shares `8ed2e7aa8511355ab60b185356f48d553a14d01121ce180e146e9b634a2c9630`; the backup and reopened JSON store share `18f9f596408287d237e38014bb3738d86edceb739c6b9047984f17b02d9794a3`. Artifacts and logs are disposable and unstaged.

- **D-STRUCTURED-190ee03-focused:** [Transcript](../phase2-local/target/D-STRUCTURED-190ee03-focused.log): 10 passed, 0 failed, 0 ignored, including the existing SQLite archive regression in the default non-raster path. An initial invocation selected the nonexistent `product_entry` target and ran no tests; the corrected target was `l09_product_entry`.
- **D-STRUCTURED-190ee03-offline:** [Transcript](../phase2-local/target/D-STRUCTURED-190ee03-offline.log): 328 passed, 0 failed, 2 ignored (subprocess worker entry points). `cargo fmt --check`, offline locked all-target Clippy with `-D warnings`, and offline locked build passed. Rust editor diagnostics timed out; no LSP pass is claimed.
- **Not run / blocked:** Frozen legacy runtime and pixel parity, actual WhatsApp/print/PDF/clipboard delivery, installed-device checks, hosted access and authorization, and protected-data recovery were not run. No owner or accounting acceptance, migration result, or cutover approval is claimed. OPEN-009 remains OWNER-BLOCKED and did not block this run; `BillDate` and both mismatch thresholds were unchanged.

#### Business identity save and output recovery

**D-L18-04-757e119 (development, scoped PASS).** Tested `757e119268b2ebf96fe2094abe0c36bb86726924` on the development Mac. The [single replay](../phase2-local/tests/l18_04.rs) used synthetic FSSAI `F<&`, business GSTIN `G<&`, and a distinct synthetic buyer GSTIN. Expectations are from [L18-04](PHASE1_CASES.md#rename-cascades-and-other-source-order-cases) and frozen source: [estimate HTML](../ALM_Business_Manager.html#L4285), [canvas](../ALM_Business_Manager.html#L4576), [GST invoice](../ALM_Business_Manager.html#L5076), and [settings save](../ALM_Business_Manager.html#L9082).

| Saved profile variant | Expected and observed in estimate HTML and fallback draw calls | Observed in GST business header |
| --- | --- | --- |
| Both flags unset; FSSAI on/GSTIN off | FSSAI only | Both identifiers |
| FSSAI off/GSTIN unset; both off | Neither | Both identifiers |
| FSSAI unset/GSTIN on; both on | FSSAI then GSTIN | Both identifiers |
| Empty FSSAI, both flags on | GSTIN only | GSTIN; no FSSAI row |
| Empty GSTIN, both flags on | FSSAI only | FSSAI and an empty `GSTIN/UIN:` field |
| Both values empty | Neither | No FSSAI row; empty `GSTIN/UIN:` field |

For each variant, saved through the local Owner settings service, dropped and reopened the source store, backed up, then restored to a fresh local file and reopened without reseeding the settings. The replay compared generated estimate HTML, ordered literal canvas identity draw calls, and standalone GST business-header markup before and after restore. Markup escaped the synthetic values, an HTML image success skipped the canvas fallback, and the GST header did not substitute the buyer GSTIN. The local profile stamp advanced on save and survived restore; prior archives without a profile loaded the default. Manager identity save and read were denied. This did not restore posting authority or render a complete GST invoice document.

```sh
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l18_04 -- --nocapture
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-L18-04-757e119-focused:** [Transcript](../phase2-local/target/D-L18-04-757e119-focused.log): 1 passed, 0 failed, 0 ignored. Final replay files were confined to a disposable repository `target` directory. An earlier development run used the OS temporary directory before the test path was corrected; it is not the cited run.
- **D-L18-04-757e119-offline:** [Transcript](../phase2-local/target/D-L18-04-757e119-offline.log): 329 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out. [DEC-023](DECISIONS.md#dec-023-use-the-standard-rust-json-serializer-for-local-recovery) pins `serde` with `derive` at `1.0.229` and `serde_json` at `1.0.154`; `Cargo.lock` was unchanged.
- **Not run / blocked:** Installed Settings UI, legacy runtime and pixel parity, actual raster or device delivery, hosted access, and acceptance were not run. OPEN-009 remains owner-blocked and independent.

#### Presentation preferences and storage failures

**D-PREF-ec56ac6 (development, scoped PASS).** Tested `ec56ac6a5f8a393d294e822f6b1215968dcfdb2c` on the development Mac. The [single synthetic replay](../phase2-local/tests/preference_replay.rs) exercises the [L18-01/L05-01 failure cases](PHASE1_CASES.md#L625) through separate local preference files, a readable business archive, and the Rust estimate renderer. Replay Buyer's ₹250.00 bill had ₹100.00 paid and a later ₹60.00 standalone receipt. The observed markup distinguished bill-date **Balance Due: ₹150.00** from latest **Total Due (as on 01-Mar-2025): ₹90.00**.

| Case | Expected and observed (development) |
| --- | --- |
| Successful save and reopen | Dark theme, privacy on, and latest balance returned. The business archive's bytes stayed unchanged; its records and profile remained readable and unchanged. |
| Preference read failures only | Unreadable key fixtures fell back to light, privacy off (the sample figure remained visible), and bill-date balance with the ₹150.00 row. After restoring key reads, dark, privacy on, and the latest ₹90.00 row returned. Business reads continued to succeed. |
| Reject each write separately with saved keys | The current session changed only the attempted preference: light theme, visible figure, or the bill-date ₹150.00 row, respectively. Each reopen returned the prior dark, privacy-on, latest ₹90.00 state. |
| Reject each write separately with missing keys | The current session changed only the attempted preference: dark theme, masked figure, or the latest ₹90.00 row, respectively. Each reopen returned light, privacy off, and the bill-date ₹150.00 row. |
| Manager with privacy on and off | The presentation mask changed; the local role checks did not. Named sale review, profit, and business backup remained denied, and provisional draft save/reopen remained allowed. Masking is not authorization. |

```sh
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test preference_replay -- --nocapture
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-PREF-ec56ac6-focused:** [Transcript](../phase2-local/target/D-PREF-ec56ac6-focused.log): 1 passed, 0 failed, 0 ignored.
- **D-PREF-ec56ac6-offline:** [Transcript](../phase2-local/target/D-PREF-ec56ac6-offline.log): 330 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Installed UI, browser storage behavior and legacy runtime parity, system theme detection, visual masking and reveal controls, actual device output, and hosted authorization. No acceptance is claimed. OPEN-009 remains owner-blocked and did not block this replay.

#### Clear All versus JSON restore with split writes

**D-CLEAR-f01c338 (development, scoped PASS).** Tested `f01c33865eb2bfccaf09ca60eca7859d8159d42e` on the development Mac. The [replay](../phase2-local/tests/clear_restore_replay.rs) models [L18-02 and L19-01](PHASE1_CASES.md#clear-all-data-versus-json-restore-not-run) using independent disposable fixtures, file-backed stand-ins for the localStorage mirror and IndexedDB primary, and no transport adapters. Expectations come from the frozen [clear handler](../ALM_Business_Manager.html#L9392), [save and reload logic](../ALM_Business_Manager.html#L684), and [restore handler](../ALM_Business_Manager.html#L10029).

| Scenario | Expected and observed immediate state | Expected and observed slot contents before reopen (mirror / primary) | Expected and observed selection |
| --- | --- | --- | --- |
| Decline Clear All | Populated revision 9 and prior activity; no writes attempted | Populated 9 / populated 9 | Populated mirror 9 |
| Confirm; both writes accepted | Cleared revision 1; only `clear` activity | Cleared 1 / cleared 1 | Cleared mirror 1 |
| Confirm; mirror accepts, primary rejects | Cleared revision 1; only `clear` activity | Cleared 1 / populated 9 | Older populated primary 9; separately, the modeled successful follow-up saved populated revision 10 to both slots |
| Confirm; primary accepts, mirror rejects | Cleared revision 1; only `clear` activity | Populated 9 / cleared 1 | Older populated mirror 9 |
| Restore marked JSON; both writes accepted | Imported revision 3 advanced to 4, not from outgoing 9; imported history plus `restore` | Imported 4 / imported 4 | Imported mirror 4 |
| Equal-revision control, distinct payloads | Not an action; reload selection only | Cleared 7 / populated 7 | Cleared mirror 7; no modeled follow-up write |

The cleared payload matched a separately created fresh archive: no products or stock rows, customers, sales, or receipts; reorder default 5 and default business profile. The JSON restore used a file with distinct marked records and profile, including imported reorder default 27 instead of outgoing 11. Dark theme, privacy masking, and latest balance reopened from separate preference files after each action scenario. The typed Rust archive handled the supported records and profile; the test harness modeled revision, activity, write rejection and reload selection. The immediate cleared state did not prove durable deletion.

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test clear_restore_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-CLEAR-f01c338-focused:** [Transcript](../phase2-local/target/D-CLEAR-f01c338-focused.log): 2 passed, 0 failed, 0 ignored.
- **D-CLEAR-f01c338-offline:** [Transcript](../phase2-local/target/D-CLEAR-f01c338-offline.log): 332 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual browser storage, asynchronous save timing, confirmation UI, legacy runtime parity, full legacy JSON migration, sync watermarks, and backup-folder handles. No remote deletion, migration, or acceptance evidence is claimed. OPEN-009 remains owner-blocked and independent.

#### Folder backup, scheduling, and reconnect

**D-FOLDER-d1ea4e5 (development, scoped PASS).** Tested `d1ea4e5132857374bbd6e6667d1bc4a032764c6a` on the development Mac. The [Rust replay](../phase2-local/tests/folder_backup_replay.rs) models the frozen [folder flow](../ALM_Business_Manager.html#L854) and [Settings actions](../ALM_Business_Manager.html#L9273), using a controlled clock, simulated picker and permission responses, an in-memory handle-token store shared across simulated sessions, and real synthetic JSON files in disposable unsynced directories. Feedback labels are normalized, not browser toasts.

| Case | Expected and observed feedback and file readback | Expected and observed timestamp and reopened handle |
| --- | --- | --- |
| L19-05 automatic, no handle / burst | No handle: no pending timer, callback, or files. With a handle, the burst moved the deadline from 1600 to 1700 ms. No write at the superseded deadline; both `ALM_data.json` and `ALM_backup_2024-05-01.json` parsed to the callback-time record at the replacement deadline, with no prompt or control feedback. | The successful callback advanced `lastFileBackup` from 7 to 1700 on the controlled clock; serialized files still carried 7. The directly assigned test handle was not persisted. |
| L19-05 automatic, permission denied at callback | Queried after denial, marked reconnect needed, and neither requested permission nor wrote either file. No manual feedback. | `lastFileBackup` stayed 7; no stored token in this fixture. |
| L19-02 manual Save and Reconnect | Save requested permission and wrote both files when granted; denied Save reported “Could not write” with neither file. Denied Reconnect reported “Permission denied”; granted retry reported “Reconnected & backed up” and updated current while preserving the same-day dated copy. Day rollover created a separate new dated copy with the updated record while retaining the earlier one. | Successful manual sequence advanced 7 to 100 to 200; rejected operations did not advance it. The directly assigned manual handles were not persisted. |
| L19-06 / L19-08 Connect and reopen | Successful Connect reported “Backup folder set” and both parsed files matched the selected-folder snapshot. Reopen with granted, prompt, or denied permission queried without requesting permission or writing; prompt and denied marked reconnect needed. | Connect advanced 7 to 100 and the selected token reopened. |
| L19-07 picker cancel vs. failure | Cancellation was silent; failure reported “Could not connect folder.” Neither reached the writer or produced either file. | Timestamp stayed 7 and prior handle remained selected and reopened in both cases. |
| L19-08 Connect with denied permission | The modeled permission request was denied. Connect still reported “Backup folder set”; neither file existed. | Timestamp stayed 7; newly stored handle reopened with reconnect needed. |
| L19-08 selected handle not persisted | A false token-store result did not stop “Backup folder set” or either backup file; both parsed to the new folder's synthetic record. | Timestamp advanced 7 to 100. Fresh session restored the prior token or none, not the newly selected handle. |
| L19-08 dated publication failure after current close | Connect still reported “Backup folder set.” Current JSON contained the expected record; dated JSON did not publish. Manual retry wrote the dated copy and reported “Saved to folder.” | Timestamp stayed 7 and daily marker stayed unset on failure; the new token reopened. Retry advanced the timestamp to 200. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test folder_backup_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-FOLDER-d1ea4e5-focused:** [Transcript](../phase2-local/target/D-FOLDER-d1ea4e5-focused.log): 11 passed, 0 failed, 0 ignored.
- **D-FOLDER-d1ea4e5-offline:** [Transcript](../phase2-local/target/D-FOLDER-d1ea4e5-offline.log): 343 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual browser picker and permissions, IndexedDB handle durability, boot warning and Settings rendering, unsupported picker, missing permission methods, rejected queries, initial current-file failure, full `persist()` fan-out, and device behavior. The staged dated-file publication is a test model, not evidence of browser write atomicity. OPEN-009 remains owner-blocked and independent.

#### Persist fan-out before local write completion

**D-PERSIST-89a8c38 (development, scoped PASS).** Tested `89a8c3893b75b230001093c9bc1b97a6d2e5a9b6` on the development Mac. The [offline Rust replay](../phase2-local/tests/persist_replay.rs) models [L19-04](PHASE1_CASES.md#persist-fan-out-not-run) from frozen [`saveDB()`](../ALM_Business_Manager.html#L717) and [`persist()`](../ALM_Business_Manager.html#L785). It seeded two disposable JSON files as stand-ins for the localStorage mirror and IndexedDB primary. Capture and completion were controlled independently; scheduler calls were recorded in-process, without transport adapters.

**Expected and observed order for every call:** revision increment → mirror write attempt and result → IndexedDB invocation → folder scheduler → cloud scheduler → clear previous Sheets timer and register its 2,500 ms replacement → side-statistics refresh. The burst canceled the first pending Sheets timer and retained only the second. All synchronous follow-up events were recorded with IndexedDB completion held, including when the mirror had already failed. The delay is a modeled timer argument, not measured browser time.

| Scenario | Expected and observed local file contents and deferred warning |
| --- | --- |
| Single, both writes succeed | Before releasing IndexedDB, mirror held revision 11 (`first`) and primary still held seed revision 10. After release, both held 11; no warning. |
| Burst, both writes succeed | Before either release, mirror held revision 12 (`second`) and primary still held 10. Releases in the scripted order wrote primary 11 (`first`), then 12 (`second`); no warning. |
| Single or burst, neither store writes | The model advanced to 11 or 12 respectively and ran the same scheduling and stats sequence. Both files retained the original seed bytes. No warning while pending; each failed release produced the matching save's “Could not save — use folder/cloud backup!” warning (`bad`, 6,000 ms argument). |
| Mixed burst, later primary released first | Mirror failed for 11 and saved 12 (`second`); primary stayed at seed 10. The later failed primary emitted no warning because its mirror saved. The earlier failed primary emitted the warning for 11 only. This is scripted release order, not an IndexedDB ordering claim. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test persist_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-PERSIST-89a8c38-focused:** [Ordered trace](../phase2-local/target/D-PERSIST-89a8c38-focused.log): 2 passed, 0 failed, 0 ignored.
- **D-PERSIST-89a8c38-offline:** [Transcript](../phase2-local/target/D-PERSIST-89a8c38-offline.log): 345 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual browser timing, storage and warning display, mirror serialization or size-limit skips, scheduler callback execution in this replay, hosted delivery, and device parity. The prior folder replay covers its own simulated callback cases; L20 delivery and callback guards are not covered by the scheduler call trace. OPEN-009 remains owner-blocked and independent.

#### Activity log retention and unsent changes

**D-ACTIVITY-4b66ccd (development, scoped PASS).** Tested `4b66ccd302c2bbc82f6d31ab8d2fc3a5a33def7e` on the development Mac. The [test-only Rust replay](../phase2-local/tests/l18_05_replay.rs) follows [L18-05](PHASE1_CASES.md#settings-activity-log-not-run) and the frozen [append](../ALM_Business_Manager.html#L838), [Settings projection](../ALM_Business_Manager.html#L9346), [backup](../ALM_Business_Manager.html#L10016), and [change selection](../ALM_Business_Manager.html#L9472) paths. It serialized and decoded the full modeled database (`log` and `deleted`) and a separate unsent `p_records` envelope. No request was sent.

| Check | Expected and observed |
| --- | --- |
| Stored append boundary | Appending entry 4,000 retained every ID `log-1` through `log-4000`. Appending 4,001 retained exactly 3,000 IDs, `log-1002` through `log-4001`, in original order. The highest seeded timestamp belonged to `log-1` and did not save it from trimming. |
| Independent Settings boundary | The 200-entry fixture projected Details positions 200 down to 1. The 201-entry fixture projected 201 down to 2, with 201 ahead of 200 despite its earlier timestamp, and the highest-timestamp position 1 omitted. Neither projection mutated the stored fixture. After the large trim, projected Details were 4,001 down to 3,802. |
| Full JSON before and after trim | Before: `log-1` through `log-4000`; after: `log-1002` through `log-4001`. Thus `log-1001` disappeared, `log-3801` and `log-4000` remained, and `log-4001` entered the backup. `deleted` was empty in both captures. |
| Unsent changes at fixed `pushedM = 50,000` | Before: exactly `log-1001`, `log-3801`; after: exactly `log-3801`, `log-4001`, all in the `log` collection. `log-3801` remained eligible despite becoming hidden in Settings. `log-4000` remained visible after trim but was ineligible because `_m` equaled the watermark. No deletion collection row was emitted. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l18_05_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-ACTIVITY-4b66ccd-focused:** [Transcript](../phase2-local/target/D-ACTIVITY-4b66ccd-focused.log): 3 passed, 0 failed, 0 ignored.
- **D-ACTIVITY-4b66ccd-offline:** [Transcript](../phase2-local/target/D-ACTIVITY-4b66ccd-offline.log): 348 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Legacy or installed Settings execution, browser timestamp formatting, durable browser storage, and hosted upload or retention. Absence from Settings or the unsent payload is not evidence of remote deletion or prior delivery. OPEN-009 remains owner-blocked and independent.

#### Modal dismissal and privacy reveal

**D-MODAL-6e420d5 (development, scoped PASS).** Tested `6e420d520e4ec2e1a4cad1793b47510b0e84a26b` on the development Mac. The [offline Rust replay](../phase2-local/tests/l18_03_replay.rs) follows the [L18-03 dismissal assertions](PHASE1_CASES.md#shared-modal-dismissal-not-run) and frozen [privacy and modal handlers](../ALM_Business_Manager.html#L1503), [Edit Bill](../ALM_Business_Manager.html#L4174), [Receive Payment](../ALM_Business_Manager.html#L5928), and [regular confirmation](../ALM_Business_Manager.html#L7069). Frozen HTML SHA-256 observed: `cd2887712e8a5a7e2fb9d45299d9034614cb4f9ec640a08ab1670acc42dd0043`. Events run against a test-only model, not a browser; bill and ledger readback and local role denials use the Rust synthetic fixture.

| Scenario | Expected and observed |
| --- | --- |
| Edit Bill and Receive Payment, clean or dirty | Header Close popped the form directly, discarding any unsaved input. Escape or an overlay-target press closed a clean form but opened a discard question over a dirty form. An inside press kept the form and its input intact. A top-level clean form closed to an empty stack with each applicable close control. |
| Discard question over either form | Keep editing, child header Close, Escape, and overlay-target press popped only the question and returned to the same dirty form with the same unsaved inputs. Repeating the original dismissal reopened the question. Discard changes then popped the question and form, dropping the unsaved inputs and leaving only the helper fixture's lower modal. |
| Regular nested confirmation control | Cancel popped the child without invoking the callback. Yes popped the child before invoking the inert callback, which observed the original dirty form on top. Neither branch saved a record. This is a helper control, not a claim that either normal Save action opens that confirmation. |
| Dirty events and footer controls | Body input, change and button/descendant clicks marked dirty; plain body and footer-only events did not. Restoring the original field value did not clear dirty. Dirty Edit Bill footer Cancel closed directly. A modeled failed Receive Payment footer action left a clean form eligible for direct Escape. |
| Records and privacy | The dismissal matrix and nested actions left the synthetic posted bill, existing standalone receipt and ledger unchanged; the matrix also checked the separate New Bill draft. The record comparison's positive mutation control intentionally added a receipt and triggered the expected assertion failure. With privacy on, an eligible figure reveal consumed the modeled click, exposed only that figure, and did not open the card; tapping again re-masked it. The Owner's permitted data was unchanged. Manager bill and ledger reads remained denied before and after reveal. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l18_03_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-MODAL-6e420d5-focused:** [Transcript](../phase2-local/target/D-MODAL-6e420d5-focused.log): 7 passed, 0 failed, 0 ignored, including the expected-panic mutation control.
- **D-MODAL-6e420d5-offline:** [Transcript](../phase2-local/target/D-MODAL-6e420d5-offline.log): 355 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual browser keyboard, pointer or event propagation, installed-device behavior, visual blur, real footer validation or save execution, and hosted authorization. OPEN-009 remains owner-blocked and independent.

#### Rate List selection and output capture

**D-RATE-73f4099 (development, scoped PASS).** Tested `73f4099340a61d605c15424e13cab62c618500c3` on the development Mac. The [offline Rust workflow](../phase2-local/tests/rate_list_replay.rs) uses a synthetic Alpha/Beta catalog and a later unbranded item. Expectations are from [L17-01 through L17-03](PHASE1_CASES.md#rate-list-selection-and-output-guards-not-run), [selection source](../ALM_Business_Manager.html#L11263), and [output source](../ALM_Business_Manager.html#L11475). No browser or production Rate List was run.

| Case | Expected and observed |
| --- | --- |
| L17-01: shown, selected, emitted | First visit showed IDs `[3,2,1]`, selected `[1,2,3]`, and chose them in catalog order `[1,2,3]`. Filtering Alpha to show only ID 2 and clearing shown left `[1,3]`; the filtered group toggle changed only 2, preserving hidden 3 and Beta 1. With Beta showing the unticked ID 1 and only hidden ID 3 selected, Preview, Print, PDF-request markup and WhatsApp emitted 3 only. With no picker matches in the session scenario, selected IDs 1 and 4 still reached all four outputs. With all rows selected, HTML grouped by brand and kept catalog order within each group: `[2,3,1,4]`. |
| L17-02: format and sink behavior | For ID 3 the picker rate was ₹123; markup and WhatsApp used ₹123.46. HTML included escaped item text, `1 L ×4`, MRP ₹200, business/date header, brand subset and footer. Print and PDF captured identical HTML with an inert print trigger, each requested `_rl` at `820 × 900`, and PDF added the 3,800 ms hint. WhatsApp captured the exact plain-text heading, date, group, item and sell rate, without Pack, MRP or mobile, plus the encoded recipient-free URI. Restricted fixture markers were excluded from the public row projection and absent from the generated and retained outputs. The blocked-popup case recorded attempts but no document; PDF still emitted the hint. |
| L17-03: new IDs, revisit, empty guard | A later visit selected new ID 4 but left deselected known IDs 2 and 3 off. Same-session revisit preserved the Alpha and search filters and selected `[1,4]`; fresh-session initialization showed `[4,3,2,1]` and selected all. With a populated catalog and no selection, preview emitted no document; Print, PDF and WhatsApp made no popup, document or chat calls. They warned, and PDF also emitted its hint. |

Retained [preview](../phase2-local/target/rate-list-replay-22290-1795257561321851000/preview.html), [Print markup](../phase2-local/target/rate-list-replay-22290-1795257561321851000/print.html), [PDF-request markup](../phase2-local/target/rate-list-replay-22290-1795257561321851000/pdf-print.html), [WhatsApp text](../phase2-local/target/rate-list-replay-22290-1795257561321851000/whatsapp.txt) and [captured URI as text](../phase2-local/target/rate-list-replay-22290-1795257561321851000/whatsapp-uri.txt). The Print and PDF-request files share SHA-256 `d64e1de14fea13661f31bc23ef2060d7521fb1358d812bf53c6902f70d935375`. These are disposable local artifacts, not delivered outputs.

```sh
cd /Users/apple/ALM && ALM_RATE_LIST_ARTIFACTS=1 cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test rate_list_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-RATE-73f4099-focused:** [Transcript](../phase2-local/target/D-RATE-73f4099-focused.log): 4 passed, 0 failed, 0 ignored.
- **D-RATE-73f4099-offline:** [Transcript](../phase2-local/target/D-RATE-73f4099-offline.log): 359 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual Rate List UI or browser locale/rounding parity beyond the fixed fixture, print layout and printer delivery, PDF creation, WhatsApp delivery, installed-device parity and hosted role authorization. The replay did not change application permissions. OPEN-009 remains owner-blocked and independent.

#### Standalone GST Print, Save PDF, and markup page boundaries

**D-GST-e24c787 (development, scoped PASS).** Tested `e24c7877960590a0dd988154f18c0fb1be16d908` on the development Mac. The [offline replay](../phase2-local/tests/gst_preview_replay.rs) saves and reads back synthetic standalone invoices through the local Rust fixture. It then models the frozen [preview actions](../ALM_Business_Manager.html#L5135), [popup handler](../ALM_Business_Manager.html#L4826), and [page chunker](../ALM_Business_Manager.html#L5101). This is an in-memory invoice readback and source-derived markup, not a browser run or complete invoice-layout comparison.

**L06-04 (local PASS):** Print and Save PDF each rendered before requesting `_p` (`width=840,height=920`). On success each captured and closed an inert document with the same preview body and title `GST-Invoice-24-25-47`, and a 350 ms on-load print intent. An updated fixture reached the renderer rather than reusing the earlier body. For each blocked action, the trace recorded the open attempt then “Pop-up blocked — allow pop-ups for this page to print” (`warn`), with no document delivered.

**L06-05 (local PASS):** Expected and observed item IDs matched in both actions' HTML readback. Only the last markup page contained CGST, SGST, grand total and footer. Each preceding page contained its next-page continuation and `page-break-after:always`. All item IDs appeared exactly once and in order.

| Item count | Expected and observed page contents (`ITEM-` IDs) | Summary page |
| --- | --- | --- |
| 12 | `001–012` | 1, with items |
| 13 | `001–013`; no items | 2, summary only |
| 20 | `001–020`; no items | 2, summary only |
| 32 | `001–020`; `021–032` | 2, with items |
| 33 | `001–020`; `021–033`; no items | 3, summary only |

Retained representative HTML: [12-row Print](../phase2-local/target/gst-preview-replay-33080-1795259050868309000/invoice-12-print.html), [33-row Print](../phase2-local/target/gst-preview-replay-33080-1795259050868309000/invoice-33-print.html), and [33-row Save PDF request](../phase2-local/target/gst-preview-replay-33080-1795259050868309000/invoice-33-save-pdf.html). The print script is inert; the replay omits unrelated invoice details such as transport metadata, the HSN breakdown and amounts in words.

```sh
cd /Users/apple/ALM && ALM_GST_PREVIEW_ARTIFACTS=1 cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test gst_preview_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-GST-e24c787-focused:** [Transcript](../phase2-local/target/D-GST-e24c787-focused.log): 4 passed, 0 failed, 0 ignored.
- **D-GST-e24c787-offline:** [Transcript](../phase2-local/target/D-GST-e24c787-offline.log): 363 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Actual browser preview or popup, physical page fit, printer delivery, PDF creation, and installed-device behavior. OPEN-009 remains owner-blocked and independent.

#### Milestone B purchase order outputs after partial receipt

**D-PO-c315f51 (development, scoped PASS).** Tested `c315f512389a43dbeb03fd81f22227702007a6b1` on the development Mac. The [L07-03 replay](../phase2-local/tests/milestone_b_replay/l07_03.rs) reused Milestone B's planned `PO-001` and dated partial receipts, then invoked the existing Rust View, shared Save PDF / Print capture, and handoff paths. Expectations are from [Supplier PO Outputs](PHASE1_CASES.md#supplier-po-outputs-not-run).

| Check | Expected and observed |
| --- | --- |
| Internal View | `PartiallyReceived`. Synthetic A: 12 ordered, 8 received, 0 cancelled, 4 outstanding; Synthetic B: 9 ordered, 5 received, 0 cancelled, 4 outstanding. View resolved A by item name to `11 jars`, MRP ₹71.75. |
| Supplier copy | The shared Save PDF / Print HTML and PNG render input used A's product ID to show `12 bottles`, MRP ₹67.50, and 12 cases; B used the item-name fallback and showed `6 cans`, MRP ₹89.50, and 9 cases. The footer counted 21 cases, with the stored supplier name and note retained. No receipt progress, purchase rates (₹1.00 and ₹1.80), or monetary total (₹28.20) appeared. The captured `_p` document carried an inert 350 ms on-load print intent. |
| Image and recipient branches | Native share success and rejection each stopped at one share request without download, chat or retry. Unsupported file sharing captured a download and a separate chat intent only when a usable phone resolved. The matching ID took priority over an earlier exact-name match; a stale or missing ID used the first exact-name match; without an exact name, a resolved ID served as fallback. An unusable chosen phone did not trigger another lookup. With no PNG, only targeted or generic text was captured, with the original 12 and 9 cases. Manager View, Print and handoff were denied before output. |
| No inventory mutation | Before and after the outputs, the saved PO and receipt records were equal, billing stock stayed at 8 and 5 packets, and dated FIFO layers were unchanged at ₹9.80 and ₹10.30 (₹20.10 combined). The supplier copy labels ordered quantities as cases; no conversion between those counts and billing stock is inferred. A separate explicit Close Remaining control moved the 4 open cases on each line to cancelled without changing stock or FIFO; its supplier copy still showed the original ordered cases. |

Retained [supplier HTML](../phase2-local/target/l07-03-po-51976-1795260954504708000/supplier.html), [Print/PDF-request HTML](../phase2-local/target/l07-03-po-51976-1795260954504708000/print-intent.html), [raster source SVG](../phase2-local/target/l07-03-po-51976-1795260954504708000/supplier.svg), [PNG](../phase2-local/target/l07-03-po-51976-1795260954504708000/supplier.svg.png), and [branch and inventory trace](../phase2-local/target/l07-03-po-51976-1795260954504708000/replay-trace.txt). Quick Look rasterized a fixture-specific SVG derived from the captured supplier HTML. The decoded and inspected 760 × 760 image showed both MRPs and original quantities. Its SHA-256 is `b0d7f289566961fe05582bde94324cd94bb1728eac33e9020b92d895b4f47339`; the inert share sink received the generated PNG bytes. Other image branches used routing-only bytes, and the default offline suite does not rasterize.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target/tmp ALM_L07_03_PNG=1 cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_b_replay l07_03_supplier_outputs_preserve_milestone_b_progress_and_inventory -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-PO-c315f51-focused:** [Transcript](../phase2-local/target/D-PO-c315f51-focused.log): 1 passed, 0 failed, 0 ignored; artifact mode enabled.
- **D-PO-c315f51-offline:** [Transcript](../phase2-local/target/D-PO-c315f51-offline.log): 364 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run:** Legacy browser HTML-to-PNG encoder and layout parity, printer delivery, PDF creation, WhatsApp delivery or attachment, and installed-device behavior. OPEN-009 remains owner-blocked and independent.

#### Supplier chat and customer reminder navigation capture

**D-CONTACT-c12fa44 (development, scoped PASS).** Tested `c12fa440f8b87adf26be3110a71be34528cdb1b3` on the development Mac. The [offline replay](../phase2-local/tests/contact_reminder_replay.rs) models [L09-06](PHASE1_CASES.md#supplier-contact-chat-navigation-not-run) and [L14-06](PHASE1_CASES.md#customer-reminder-navigation-not-run) using synthetic contacts and an in-memory recorder with no navigation or delivery adapter.

| Case | Expected and observed |
| --- | --- |
| L09-06 supplier WA | Blank mobile hid the control and emitted nothing. Nonempty malformed mobile showed the control and captured `_blank` at `wa.me/91`; the formatted synthetic mobile showed the control and captured `_blank` at `wa.me/910000000001`. Neither request had message text, a warning, or a PO attachment. |
| L14-06 visibility | For Customers **Remind**, ledger **Send Reminder**, udhaar **WA**, Dashboard Top debtors **WA**, and ageing detail **WA**, the valid and malformed mobile fixtures showed the control; missing mobile and the advance fixture without an outstanding row did not. The ageing model separately checked that a positive outstanding row can show WA without positive current balance, and that 49 and 50 paise are excluded while 51 paise qualifies. Top-debtor ranking and ageing allocation were not replayed. |
| L14-06 warning and request | At all five entry points the malformed debtor recorded `No valid mobile for A & B` (`warn`) with no open request. The valid debtor captured one `_blank` request to `wa.me/910000000001` with the exact encoded `text` query and no warning. Decoding recovered the greeting, synthetic business name, pending balance and sign-off. |
| L14-06 click-time balance | The modeled current balance recomputed by ID was ₹740 (₹100 opening + ₹500 bill − ₹80 bill payment + ₹250 second bill − ₹30 standalone receipt). A cancelled sale and a same-name customer’s records were excluded. After a further ₹125 receipt, clicking the same ageing row captured ₹615, not the fixture’s ₹420 invoice snapshot or ₹250 ageing-row amount. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test contact_reminder_replay -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-CONTACT-c12fa44-focused:** [Transcript](../phase2-local/target/D-CONTACT-c12fa44-focused.log): 2 passed, 0 failed, 0 ignored.
- **D-CONTACT-c12fa44-offline:** [Transcript](../phase2-local/target/D-CONTACT-c12fa44-offline.log): 366 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run / blocked:** Actual legacy and installed controls remain unrun; actual handoff is owner-blocked under OPEN-005. No chat was opened and no message was sent. OPEN-009 remains independent.

#### Sales-row WhatsApp and cancelled estimate history

**D-SALES-6e12f3a (development, scoped PASS).** Tested `6e12f3a85c8d8426ede9a12e1fb71da57427c618` on the development Mac. The [offline replay](../phase2-local/tests/milestone_d_replay/sales_output.rs) invokes the fixture-backed Rust Sales-row and history routes against [L05-05](PHASE1_CASES.md#L150). It reuses the existing estimate renderer and inert handoff helpers but does not run the legacy or installed Sales UI.

| Entry point | Expected and observed |
| --- | --- |
| Active Sales row → WA, usable phone | Selected stored row `DraftId(21)` for Estimate #1 with a second posted sale and a decoy contact in the fixture. The row resolved the matching customer ID to synthetic recipient `910000000000`. With the first image generator returning sentinel bytes and native sharing unavailable, the sink captured an `Estimate-1.png` download request and a separate chat target, with no text fallback or canvas attempt. |
| Active Sales row → WA, no usable phone | The same selected row, with only the nonmatching contact available, resolved no recipient. The sink captured the same file request with no chat or text request. |
| Cancelled history row → View → preview WA | View reopened stored Estimate #1 with `<p>CANCELLED</p>` in the preview markup. When the HTML and canvas generators both failed, in that order, the sink captured one encoded `_blank` text request for the matching synthetic recipient, without a file probe, file request or separate chat. The literal comparison used the frozen text format: 21-Feb-2025, Synthetic Buyer, Synthetic A ×2 at ₹354 and Synthetic B ×1 at ₹56 as line amounts, loading ₹0, bill amount ₹410, paid ₹100 and latest due ₹-50 as of 01-Mar-2025. The text did not include a cancellation marker. |
| Denied routes | Manager row selection was denied; cancelled direct row WA was rejected before either image generator or any output sink was called. |

```sh
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_replay sales_output:: -- --nocapture
cd /Users/apple/ALM && cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-SALES-6e12f3a-focused:** [Transcript](../phase2-local/target/D-SALES-6e12f3a-focused.log): 3 passed, 0 failed, 0 ignored, 2 filtered out.
- **D-SALES-6e12f3a-offline:** [Transcript](../phase2-local/target/D-SALES-6e12f3a-offline.log): 369 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy with `-D warnings`, and locked offline build passed. Rust editor diagnostics timed out.
- **Not run / blocked:** Actual Sales-row UI, legacy runtime and cancellation marker in a rendered image were not checked by this replay. File bytes in the active-row test were a routing sentinel. No chat opened and no file was sent; actual device handoff remains owner-blocked under OPEN-005. OPEN-009 remains independent.

#### HTML PNG encoder selection offline replay

**D-PNG-33d9bd8 (development, scoped PASS).** Tested `33d9bd8b13c39556ad53b79a011d9c0e73efb7fe` on the development Mac. The [test-only encoder](../phase2-local/tests/milestone_d_replay/png_encoder.rs) models the [source-defined post-image-load branch](PHASE1_CASES.md#html-png-encoder-selection-not-run). The [caller replay](../phase2-local/tests/milestone_d_replay/png_output.rs) runs the existing Rust estimate and supplier PO handoff paths with synthetic records, controlled PNG encoders and in-memory output sinks. The primary and hand-drawn fallback fixtures have distinct bytes; neither is an image of the document.

| Case and control | Ordered helper calls and callback result (both callers) | Ordered caller actions after the helper callback |
| --- | --- | --- |
| L05-03 / L07-03: `toBlob` returns a Blob | `drawImage` → `toBlob("image/png")` → callback with the **same Blob object** (reference identity checked). No `toDataURL` or decode. | Estimate: probe → capture download of `Estimate-1.png`. PO: probe `o.png` → capture download of `Order-PO-001.png`. Probe and download bytes match the primary fixture; no fallback. |
| L05-03 / L07-03: `toBlob` absent | `drawImage` → `toDataURL("image/png")` → base64 decode → callback with `image/png` and bytes equal to the fixed primary fixture. No `toBlob` call. | Both callers took the same file routes as above with the exact decoded bytes; no caller fallback. |
| L05-04 / L07-03: `toBlob` calls back with `null` | `drawImage` → `toBlob("image/png")` → callback with `null`. No helper `toDataURL`, decode or second callback. The deliberate retry control failed before the retry was removed. | Estimate: separate hand-drawn canvas stub → its own `toDataURL("image/png")` → decode → probe → capture download of `Estimate-1.png` with the distinct fallback bytes. PO: one `_blank` text intent for the original Synthetic A order of 3 cases, with no second render, canvas, file probe or download. |

The PO caller requested the HTML render at width 760 once in each variant. File probes reported native sharing unavailable and both fixtures deliberately omitted a recipient. No share or separate chat request was allowed. The estimate's second-canvas failure was not injected by this encoder replay; [the existing estimate fallback slice](#inert-estimate-canvas-and-text-fallback) covers that case separately.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_replay png_ -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-PNG-33d9bd8-focused:** [Transcript](../phase2-local/target/D-PNG-33d9bd8-focused.log): 3 passed, 0 failed, 0 ignored, 5 filtered out. PASS for the named helper and caller branches only.
- **D-PNG-33d9bd8-offline:** [Transcript](../phase2-local/target/D-PNG-33d9bd8-offline.log): 372 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy (`-D warnings`) and locked offline build passed. Rust editor diagnostics timed out.
- **Not run / blocked:** This is branch behavior, not image fidelity. The frozen HTML was not served or executed; the Phase 1 runtime cases remain not run under OPEN-008. No chat was opened and no file was sent. Actual handoff remains owner-blocked under OPEN-005. OPEN-009 is independent and unchanged.

#### Standalone GST form and invoice actions

**D-GST-289dfaf (development, scoped PASS).** Tested `289dfaf8023d0b1c07e2058ae4231b79aa83329f` on the development Mac with synthetic records. Form state and the hard-delete confirmation were modeled in Rust tests. Save, invoice readback, cancellation, billing stock and customer ledger used the existing Rust fixture APIs. No production workflow changed.

**L06-06: GSTIN form behavior.** [Replay](../phase2-local/src/confirmation/tests/gst_form_replay.rs); source expectations: [input handler](../ALM_Business_Manager.html#L5421) and [initial control setup](../ALM_Business_Manager.html#L5575).

| Step | Buyer state / code, expected and observed | Place of supply, expected and observed | Buyer State / Code editable | Place of Supply State / Code editable | GSTIN editable | Length warning, expected and observed |
| --- | --- | --- | --- | --- | --- | --- |
| Reopen populated draft with recognized `27`; no input event | Kerala / `32` retained | Goa / `30` retained | No / No | Yes / Yes | Yes | None for the stored 15-character value; the padded, lowercase reopen control retained its raw text and warned at 17. |
| Enter recognized `27`, correct trimmed length | Maharashtra / `27` inferred | Maharashtra / `27` inferred | No / No | Yes / Yes | Yes | None. |
| Enter recognized `27`, wrong trimmed length | Maharashtra / `27` still inferred | Maharashtra / `27` still inferred | No / No | Yes / Yes | Yes | Warned at 14, independent of recognition. |
| Enter unrecognized `88`, correct trimmed length, on a separate copy of the prefilled form | Kerala / `32` retained | Goa / `30` retained | Yes / Yes | Yes / Yes | Yes | None. |
| Clear after recognized input | Maharashtra / `27` retained | Maharashtra / `27` retained | Yes / Yes | Yes / Yes | Yes | None; GSTIN empty. |

Modeled input uppercased the text without stripping padding; the input warning counted the trimmed value. Separately, Rust Save rejected the wrong-length value with `InvalidGstin` and emitted no invoice. The correct-length unknown prefix round-tripped through Rust Save/readback, retaining the caller-supplied `Inter` jurisdiction. Neither prefix recognition nor this Save check validates registration or establishes tax policy.

**L06-03: delete versus cancel.** [Replay](../phase2-local/src/confirmation/tests/gst_action_replay.rs); source expectations: [row actions](../ALM_Business_Manager.html#L5260) and [number suggestion](../ALM_Business_Manager.html#L5170). Each branch started from an independent copy with an active target numbered `700` and control invoice `699`. Billing stock for fixture products `1`, `2`, and `3` and named-customer ledger dues were read before and after each action.

| Action | Expected target | Observed Rust readback | Billing stock before → after (units) | Customer dues before → after | Modeled next number, observation only |
| --- | --- | --- | --- | --- | --- |
| Decline delete | Same active invoice | Full invoice unchanged, `cancelled=false`. | `[3, 4, 4]` → `[3, 4, 4]` | ₹342 → ₹342 | `701` |
| Confirm delete | Target absent | Test-only removal; real readback returned `NotFound` and emitted nothing. | `[3, 4, 4]` → `[3, 4, 4]` | ₹342 → ₹342 | `700` |
| Cancel invoice | Same invoice retained, flagged cancelled | Readback matched the full original record and totals except `cancelled=true`. | `[3, 4, 4]` → `[3, 4, 4]` | ₹342 → ₹342 | `701` |

The control invoice and ordinary customer ledger rows remained identical in all branches. The source-shaped number observation used the remaining records, including cancelled invoices; no subsequent invoice was issued and no reuse rule was approved. Legacy deletion persistence, tombstones and sync were not exercised.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::gst_ -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- **D-GST-289dfaf-focused:** [Transcript](../phase2-local/target/D-GST-289dfaf-focused.log): 6 passed, 0 failed, 0 ignored, 256 filtered out. Before the final replay, the harness failed on two padding assertions and an unsupported rounding fixture; the model and synthetic fixture were corrected without changing production code.
- **D-GST-289dfaf-offline:** [Transcript](../phase2-local/target/D-GST-289dfaf-offline.log): 378 passed, 0 failed, 2 ignored (subprocess workers). Formatting, locked offline all-target Clippy (`-D warnings`) and locked offline build passed. Rust editor diagnostics timed out.
- **Not run / blocked:** Frozen HTML remained unserved under OPEN-008. No browser, installed device, registration, numbering policy or accounting acceptance is implied.

#### Milestone D handoff rerun

**Scoped local development evidence** at `ab376e9c9f7711ed2691e0c7b227ad56d70146c1`. See the [coverage crosswalk](MILESTONE_D_COVERAGE.md#handoff-rerun-dispositions).

| Run ID / log | Observed |
| --- | --- |
| [D-HANDOFF-ab376e9-focused](../phase2-local/target/D-HANDOFF-ab376e9-focused.log) | 10 passed, 0 failed, 0 ignored: `l18_04` (1), `milestone_d_replay` (8, one joined pre-raster workflow plus separate encoder/routing tests), `milestone_d_structured` (1). |
| [D-HANDOFF-ab376e9-offline](../phase2-local/target/D-HANDOFF-ab376e9-offline.log) | 378 passed, 0 failed, 2 ignored. The green regression suite does not establish a full handoff PASS. |
| [D-HANDOFF-ab376e9-gst-guard](../phase2-local/target/D-HANDOFF-ab376e9-gst-guard.log) | 1 passed, 0 failed, 0 ignored, 261 filtered out. Ordinary GST negative control only: asserted `UnsupportedRounding` and no output. |

Replay commands:

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test milestone_d_replay --test l18_04 -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
cd /Users/apple/ALM && cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::l06_01::l06_01_unrepresentable_taxable_rate_emits_nothing -- --exact --nocapture
```

- **End-to-end, structured exports and reorder setting only:** The Owner changed the reorder default from 5 to 13, reopened the source store, generated Sales CSV and SpreadsheetML from C's cancelled bill and an active control, and saved the backup before restoring. The populated store also held a separate receipt. The test restored to an absent destination, reopened without reseeding, and regenerated both exports. Assertions checked date-sorted CSV rows with Mode/Pack, stored-order SpreadsheetML Sales rows without those columns, first-line totals and paid amounts, and survival of IDs, nested crate components, and the receipt. Before and after restore, an Owner product save with the reorder field blank reopened with level 13. On the source store the Manager received no CSV, SpreadsheetML or backup output and could not restore or modify the setting; applying the setting was denied before and after restore.
- **Retained structured artifacts:** [source.json](../phase2-local/target/structured-d-1119-1795266188096201000/source.json), [backup.json](../phase2-local/target/structured-d-1119-1795266188096201000/backup.json), [restored.json](../phase2-local/target/structured-d-1119-1795266188096201000/restored.json), [sales.csv](../phase2-local/target/structured-d-1119-1795266188096201000/sales.csv) and [regenerated-sales.csv](../phase2-local/target/structured-d-1119-1795266188096201000/regenerated-sales.csv), [workbook.xls](../phase2-local/target/structured-d-1119-1795266188096201000/workbook.xls) and [regenerated-workbook.xls](../phase2-local/target/structured-d-1119-1795266188096201000/regenerated-workbook.xls). `cmp` found exact matches for both original versus regenerated export pairs; the content and restored control assertions above went beyond byte equality.
- **Separate SQLite pre-raster replay:** The joined workflow produced estimate HTML and reconstructed SVG, captured inert Print/PDF/share/clipboard intents, checked the exact ordinary GST projection, and verified scoped archive restore readback. The CSV was copied, not regenerated. Provisional Manager output contained no restricted markers; restricted output attempts and the SQLite backup helper call as Manager were denied. No artifacts were retained from this nonraster replay. Encoder and routing tests ran separately from the joined workflow.
- **Separate `l18_04` identity replay:** Restored profile settings from an archive with empty sales and bound them to in-memory bills to check estimate markup, canvas identity lines, and standalone GST header markup. This was not full bill recovery.
- **Unsupported / not run:** Regenerating estimates, GST, PO and other outputs from the same reopened populated store was not supported or run. No passing helper was substituted for that integration step. Separately, the original standalone L06-03 fixture at an inclusive rate of 10,500 paise and 18% GST failed with `UnsupportedRounding`; `289dfaf` replaced it with the exact 11,800-paise fixture. The original is excluded and unsupported, was not tested in this handoff, and is not covered by the ordinary GST guard.

OPEN-008 still gates the legacy walkthrough, and OPEN-005 still gates actual device outputs and handoff. No full application restore, accounting sign-off, acceptance, or phase completion is claimed.

#### Milestone D same-store estimate recovery

**Scoped development PASS** at `17fb557c9e68e35c1e45f8c9cbb536884952b862` (renderer refactor `1b07f6b`). This closes only the estimate HTML regeneration gap recorded in the historical `ab376e9` handoff. The joined structured test rendered from the populated source, backed it up, dropped the origin fixture and source handle, then restored and freshly reopened the JSON without reseeding. It regenerated the estimate from the reopened records and saved profile, not cached HTML. Estimate rendering is read-only and does not restore posting authority.

| Check | Expected and observed |
| --- | --- |
| Cancelled C bill | Invoice #1, 21-Feb-2025, Synthetic Buyer, `CANCELLED`; A × 2 at ₹354.00 for the row and B × 1 at ₹56.00 for the row; bill ₹410.00, paid ₹100.00, latest balance ₹-50.00 as of 01-Mar-2025. FSSAI `A<&>` was HTML-escaped; GSTIN was hidden by the saved profile. |
| Independent controls | In a disposable control store, changing the linked receipt to ₹70 yielded a latest balance of ₹-70.00; flipping the profile flags hid FSSAI and displayed the escaped GSTIN. In another control, assigning the active bill to the same account changed the cancelled estimate’s latest balance to ₹-42.00. Missing, duplicate, or dangling links failed closed. The supported scope uses an explicit zero opening balance; fractional latest balances and nonzero opening balances remain unsupported. Older unscoped v1 JSON remained readable but did not provide an estimate. |
| Access and read-only behavior | Manager received exactly `AccessDenied` for estimate and backup before and after restore. Estimate reads left both the disk state and the in-memory backup unchanged. |

The test independently checked the case anchors in both HTML outputs; `cmp` confirmed an exact HTML match. Retained evidence: [estimate.html](../phase2-local/target/structured-d-54111-1795267841077953000/estimate.html), [regenerated-estimate.html](../phase2-local/target/structured-d-54111-1795267841077953000/regenerated-estimate.html), [backup.json](../phase2-local/target/structured-d-54111-1795267841077953000/backup.json), and [restored.json](../phase2-local/target/structured-d-54111-1795267841077953000/restored.json).

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test l18_04 -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-ESTIMATE-17fb557-focused](../phase2-local/target/D-ESTIMATE-17fb557-focused.log): 2 passed, 0 failed, 0 ignored (one joined structured workflow and the existing separate `l18_04` identity replay).
- [D-ESTIMATE-17fb557-offline](../phase2-local/target/D-ESTIMATE-17fb557-offline.log): 378 passed, 0 failed, 2 ignored. Build, formatting, and all-target Clippy passed; Rust LSP timed out.

At `17fb557`, same-store GST and PO regeneration were **not run**. See the ordinary GST follow-up below. The original standalone GST fixture (10,500 paise inclusive at 18%) remains unsupported. OPEN-008 and OPEN-005 still gate the legacy walkthrough and actual device outputs and handoff. This is not a full restore, accounting sign-off, acceptance, or phase completion.

#### Milestone D same-store ordinary GST recovery

**Scoped L06-01 / L19-01 development PASS** at `a3f3af989dc0ce141284a07a6a92e4e37713a355` (shared calculator `7150513`). The structured test generated a typed ordinary GST document, backed up the store, dropped the source, then restored and reopened the JSON. It regenerated the document from the saved sale and ordered tax metadata, not from saved calculated outputs, a catalog lookup, or a replay of the sale.

The source and regenerated documents independently matched the Milestone C case: cancelled invoice #1, 21-Feb-2025, Synthetic Buyer; A × 2 for ₹354 and B × 1 for ₹56. Total ₹410, taxable ₹350, GST ₹60, CGST and SGST ₹30 each, IGST ₹0.

| Line | HSN | GST rate | Taxable | GST | CGST | SGST |
| --- | --- | --- | --- | --- | --- | --- |
| A | `22029920` | 18% | ₹300 | ₹54 | ₹27 | ₹27 |
| B | `21069099` | 12% | ₹50 | ₹6 | ₹3 | ₹3 |

**Controls:** Manager requests for known and unknown sales returned `AccessDenied`. The same cancelled sale without tax metadata returned `MissingTax`. An archived second-line rate of 1,201 basis points returned `UnsupportedRounding` with unchanged sale totals. Malformed positions, lines, or totals were invalid. The rejected cases produced no output. Both successful and rejected requests left the store file and serialized in-memory backup unchanged.

`cmp` confirmed an exact match between [gst.json](../phase2-local/target/structured-d-23970-1795269290231793000/gst.json) and [regenerated-gst.json](../phase2-local/target/structured-d-23970-1795269290231793000/regenerated-gst.json). Retained recovery files: [backup.json](../phase2-local/target/structured-d-23970-1795269290231793000/backup.json) and [restored.json](../phase2-local/target/structured-d-23970-1795269290231793000/restored.json).

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test milestone_c_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-ORD-GST-focused](../phase2-local/target/D-ORD-GST-focused.log): 2 passed (Milestones C and D), 0 failed, 0 ignored.
- [D-ORD-GST-offline](../phase2-local/target/D-ORD-GST-offline.log): 380 passed, 0 failed, 2 ignored. Formatting, all-target Clippy, and build passed; Rust LSP timed out.

Same-store PO regeneration was **not run at `a3f3af9`**; see the PO follow-up below. The separate standalone ₹105 inclusive fixture at 18% remains unsupported. OPEN-008 and OPEN-005 remain in force. This is local typed JSON evidence only, not actual print or device evidence, accounting sign-off, or phase acceptance.

#### Milestone D same-store purchase-order recovery

**Scoped L07-03 / L19-01 development PASS** at `68c661a68fd6a13e10ef6ca1c15fa822d378fbe8` (renderer `fae3b7e`). After dropping the Milestone B and C fixtures, the structured test restored from backup and freshly reopened the store. It regenerated supplier HTML and internal PO readback from saved records, without replaying receipts or using cached markup. The store is a read model, not posting authority.

- **Supplier HTML:** Independent golden checks on both outputs confirmed original quantities of 12 and 9 cases and MRPs of ₹67.50 and ₹89.50, rendered as ₹67.5 and ₹89.5. The only total was 21 cases; no order rates, receipt progress, or monetary total appeared.
- **Internal readback:** Ordered/received/outstanding quantities were A 12/8/4 and B 9/5/4; cancelled quantities were zero. Receipt entries were unchanged across restore and retained append order: Feb 12, then Feb 11.
- **Access and state:** Manager requests for known and unknown POs returned `AccessDenied` with no output. Readback and rendering left state and persisted bytes unchanged.

`cmp` matched both artifact pairs exactly: [purchase-order.html](../phase2-local/target/structured-d-15409-1795271320015425000/purchase-order.html) and [regenerated-purchase-order.html](../phase2-local/target/structured-d-15409-1795271320015425000/regenerated-purchase-order.html); [purchase-order-readback.txt](../phase2-local/target/structured-d-15409-1795271320015425000/purchase-order-readback.txt) and [regenerated-purchase-order-readback.txt](../phase2-local/target/structured-d-15409-1795271320015425000/regenerated-purchase-order-readback.txt). Retained recovery files: [backup.json](../phase2-local/target/structured-d-15409-1795271320015425000/backup.json) and [restored.json](../phase2-local/target/structured-d-15409-1795271320015425000/restored.json).

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test milestone_b_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-PO-RESTORE-68c661a-focused](../phase2-local/target/D-PO-RESTORE-68c661a-focused.log): 4 passed (Milestone B and structured D: 2 each), 0 failed, 0 ignored.
- [D-PO-RESTORE-68c661a-offline](../phase2-local/target/D-PO-RESTORE-68c661a-offline.log): 384 passed, 0 failed, 2 ignored. All-target Clippy and build passed. The initial format check failed; formatting was corrected and the final check passed. Rust LSP timed out.

The standalone ₹105 inclusive GST fixture at 18% remains unsupported. OPEN-008 and OPEN-005 remain in force; no actual printing, PDF creation, or delivery was tested. No accounting or phase acceptance is claimed.

#### Milestone D integrated offline handoff

**Scoped development PASS** at `8ba8334a5df43faecf3e7fe57fe117de70e33536`. The [existing structured replay](../phase2-local/tests/milestone_d_structured.rs) populated the read model with the synthetic Milestone C sale and customer receipt, the active export control, the Milestone B partially received PO and its receipts, and Owner settings. It dropped both origin fixtures after capturing typed records. All five source outputs came from that populated store. The replay saved one recovery backup, dropped the source store handle, restored into an absent destination, and regenerated all five outputs from the reopened store without reseeding, reposting, or using cached documents. Backup calls used for immutability assertions and separate negative-control restores are not additional inputs to the joined restore.

| Output | Case expectation checked in the joined run | Source / regenerated artifact |
| --- | --- | --- |
| Sales CSV (L19-03) | Date-sorted records with Mode and Pack. Cancelled invoice #1 retained both item rows at ₹354 and ₹56, with ₹410 total and ₹100 paid only on the first row. The active control retained its piece and crate quantities. | [Source](../phase2-local/target/structured-d-31601-1795273429199641000/sales.csv) / [regenerated](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-sales.csv) |
| SpreadsheetML (L19-03) | Products, Customers, Sales and Stock tabs; Sales retained stored order, omitted Mode and Pack, and preserved cancellation and first-line totals. | [Source](../phase2-local/target/structured-d-31601-1795273429199641000/workbook.xls) / [regenerated](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-workbook.xls) |
| Cancelled estimate HTML (L05-01/L18-04) | Estimate #1 dated 21-Feb-2025 retained its cancellation marker, ₹410 total, ₹100 paid, and latest balance of ₹-50 after the linked customer receipt. The reopened profile displayed escaped FSSAI and hid GSTIN as configured. | [Source](../phase2-local/target/structured-d-31601-1795273429199641000/estimate.html) / [regenerated](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-estimate.html) |
| Ordinary GST (L06-01) | Cancelled invoice #1 dated 21-Feb-2025 retained ₹410 total, ₹350 taxable, ₹30 each CGST and SGST, and HSN groups `22029920` and `21069099`. The retained artifact is a typed document serialized to JSON. | [Source](../phase2-local/target/structured-d-31601-1795273429199641000/gst.json) / [regenerated](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-gst.json) |
| Supplier PO HTML (L07-03) | `PO-001` retained original supplier quantities of 12 and 9 cases and MRPs of ₹67.50 and ₹89.50, without purchase rates, receipt progress or monetary total. Internal progress remained 12/8/4 and 9/5/4 (ordered/received/outstanding). | [Source](../phase2-local/target/structured-d-31601-1795273429199641000/purchase-order.html) / [regenerated](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-purchase-order.html) |

`cmp` confirmed that all five source/regenerated pairs matched byte for byte. Retained [backup](../phase2-local/target/structured-d-31601-1795273429199641000/backup.json), [reopened store](../phase2-local/target/structured-d-31601-1795273429199641000/restored.json), and [restored PO progress and receipt readback](../phase2-local/target/structured-d-31601-1795273429199641000/regenerated-purchase-order-readback.txt) are local synthetic artifacts. The restored reorder default was also exercised: a product saved with a blank reorder setting reopened at level 13. Manager requests received no restricted output: CSV and SpreadsheetML denials were exercised on the source store, and estimate, ordinary GST, PO and backup denials on the reopened store. CSV and SpreadsheetML Manager requests were not repeated after restore in this focused test.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured structured_exports_survive_fresh_recovery -- --exact --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-JOINED-8ba8334-focused](../phase2-local/target/D-JOINED-8ba8334-focused.log): 1 passed, 0 failed, 0 ignored, 1 filtered out.
- [D-JOINED-8ba8334-offline](../phase2-local/target/D-JOINED-8ba8334-offline.log): 384 passed, 0 failed, 2 ignored.

The separate image encoder and output-routing tests are regressions, not stages of the joined recovery. No raster PNG, image fidelity, physical print, PDF creation, or delivery is established by the joined run. The standalone ₹105 inclusive GST fixture at 18% remains unsupported. OPEN-008 gates legacy execution and OPEN-005 gates actual device handoff. No phase or accounting acceptance is claimed. `bridge.toml` was not changed.

#### Milestone D reopened-store Manager export denial

**Scoped L19-03 / R-5 development PASS** at `c7ded1e5dc5c66010db91789118acf69315743df`. In the same reopened populated store used for Owner regeneration, Manager requests for Sales CSV and SpreadsheetML each returned exactly `Err(AccessDenied)`, with no export payload. The Owner requests succeeded on that same store and retained both the pre-backup equality controls and the expected-content checks. The [original `8ba8334` run](#milestone-d-integrated-offline-handoff) still documents source-store-only Manager export checks; this follow-up does not change its evidence.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured structured_exports_survive_fresh_recovery -- --exact --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-EXPORT-DENIAL-c7ded1e-focused](../phase2-local/target/D-EXPORT-DENIAL-c7ded1e-focused.log): 1 passed, 0 failed, 0 ignored, 1 filtered out. [Retained replay artifacts](../phase2-local/target/structured-d-33987-1795273742034799000/).
- [D-EXPORT-DENIAL-c7ded1e-offline](../phase2-local/target/D-EXPORT-DENIAL-c7ded1e-offline.log): 384 passed, 0 failed, 2 ignored. Formatting and the locked offline build passed; Rust LSP timed out.

This verifies local export denial only. Hosted authorization, device isolation and phase acceptance remain open; OPEN-008 and OPEN-005 are unchanged.

#### Manual Cash Book close across month end

**L15-01 / L15-02 scoped development PASS** at `1b83700649962ea2e91888eb280b6c3f3c4d720b`. The [runnable Rust replay](../phase2-local/tests/l15_cashbook.rs) compared the existing system day projection with disposable in-memory manual entries. The [Cash Book source](../ALM_Business_Manager.html#L8432) was read as text only. Expected and observed figures matched in both tables; all amounts are rupees.

| Date and system entry kinds | System Cash / Online, exact → rounded | Manual Cash / Paytm / PhonePe before copy | Manual less rounded system Cash / Online, displayed |
| --- | --- | --- | --- |
| 28-Feb-2025: one Cash bill part-payment | 100.51 / 0.00 → 101 / 0 | 90.00 / 8.00 / 7.00 | -11 / +15 |
| 01-Mar-2025: separate Cash and Online customer receipts | 21.49 / 50.50 → 21 / 51 | 88.00 / 9.00 / 16.00 | +67 / -26 |

**Use system figures** copied the March buckets to Cash 21.00 and Paytm 51.00, cleared PhonePe from 16.00 to 0.00, and preserved Goods 40.01, Miscellaneous 50.02, Salary 60.03 and Debt 70.04. The stored row did not change until Save, which retained its ID. A blank date returned `MissingDate` without inserting a row. Declining deletion preserved the saved row; confirming removed only that row, leaving the other March 1 row and the February row intact. The posted sale, both dated receipts, and customer ledger stayed unchanged; the customer closing balance remained ₹63.50. A deliberate test control omitting the PhonePe clear failed before that clear was restored.

| Month checkpoint | Rows | Goods | Miscellaneous | Salary | Debt (separate) | Income | Expenses | Net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| February | 1 | 12.01 | 23.02 | 34.03 | 45.04 | 105.00 | 69.06 | 35.94 |
| March, copy not yet saved | 2 | 41.02 | 52.04 | 63.06 | 74.08 | 239.00 | 156.12 | 82.88 |
| March, after Save | 2 | 41.02 | 52.04 | 63.06 | 74.08 | 198.00 | 156.12 | 41.88 |
| March, after confirmed Delete | 1 | 1.01 | 2.02 | 3.03 | 4.04 | 126.00 | 6.06 | 119.94 |

Expenses include Goods, Miscellaneous and Salary only. Debt did not enter expenses or net. Monthly values are exact model readbacks, not rendered KPIs or Reports P&L totals.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l15_cashbook -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L15-1b83700-focused](../phase2-local/target/L15-1b83700-focused.log): 1 passed, 0 failed, 0 ignored.
- [L15-1b83700-offline](../phase2-local/target/L15-1b83700-offline.log): 385 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and build passed; Rust LSP timed out.

**Unsupported / not run:** Online payment made on a bill remains unsupported in the local fixture; a standalone Online receipt is not evidence for that path. This replay did not establish browser form or storage behavior, persisted manual rows across restart, tombstones, sync, hosted authorization, device isolation or accounting acceptance. Legacy execution remained gated by OPEN-008.

#### Owner bill payment mode in Cash Book

**L03-02 / L04-02 / L14-01 / L15-01 scoped development PASS** at `9fc4ad8098c2dc2222b78013a02d3bdd942a2ee5`. The [paired replay](../phase2-local/src/confirmation/tests/customer_accounts/cashbook_scope.rs) used independent synthetic fixtures identical except for the Owner-selected mode. Confirmation saved the approved mode. The payment appeared once as a bill entry on 21-Feb-2025, not as a standalone receipt. Expected and observed figures matched below; the first pair is exact paise and the second is rounded rupees.

| Fixture | Bill-date Cash / Online before cancellation | Bill-date Cash / Online after cancellation | Receipt-date Cash / Online before and after cancellation |
| --- | --- | --- | --- |
| Cash bill payment | `75 / 0` → `1 / 0` | `0 / 0` → `0 / 0` | `0 / 50` → `0 / 1` |
| Online bill payment | `0 / 75` → `0 / 1` | `0 / 0` → `0 / 0` | `0 / 50` → `0 / 1` |

Both fixtures had a separate 50-paise Online receipt dated 01-Mar-2025. Before cancellation the ledger showed a 75-paise bill credit and a closing balance of zero; after cancellation only the standalone receipt remained and the ledger closed at -50 paise. Replaying the same approval did not duplicate the posting; a mode-only change with the same request ID returned `RequestConflict`. Manager denials were checked without changing provisional draft behavior. The focused run also checked Online approval round-trip through the pending codec and SQLite client restart after a lost response. Older named snapshots still decode as Cash.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --offline --locked --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib named_ -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L15-MODE-9fc4ad8-focused](../phase2-local/target/L15-MODE-9fc4ad8-focused.log): 13 passed, 0 failed, 0 ignored, 256 filtered out.
- [L15-MODE-9fc4ad8-offline](../phase2-local/target/L15-MODE-9fc4ad8-offline.log): 388 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and build passed; Rust LSP timed out.

The historical `1b83700` replay did not exercise an Online bill payment; its original limitation stands. This follow-up covers only the synthetic Owner named-bill path. Hosted authorization, device isolation, legacy runtime (OPEN-008) and accounting acceptance remain open.

#### Cash Book signed inputs and numeric rounding

**L15-01 / L15-02 scoped development PASS** at `c32eafb258c07b8a322bd5e76f129e4c002cb513`. The [test-only replay](../phase2-local/src/confirmation/tests/l15_cashbook_numeric_replay.rs) exercised the [numeric case expectations](PHASE1_CASES.md#cash-book-day-close-comparison-not-run) using existing Rust projections and manual-row APIs. No production code or payment-entry validation changed.

| Independent fixture | Expected and observed count | Exact Cash / Online (paise) | Rounded Cash / Online (rupees) |
| --- | ---: | ---: | ---: |
| Signed history | 4 | 250 / -150 | 3 / -1 |
| Two 25-paise standalone entries per bucket | 4 | 50 / 50 | 1 / 1 |

The signed day counted the 175-paise Cash bill payment and the same-date standalone receipts of +75 paise Cash, -150 paise Online and zero Online, each once. The zero-paid bill and the seeded -125-paise bill contributed neither amount nor count. The normal APIs still rejected a negative bill-paid submission and a zero standalone receipt; only the test seeded those historical values directly.

With the 25-paise fixture fixed, these manual comparisons used the rounded system figures, summed Paytm and PhonePe before subtraction, and rounded only the resulting differences:

| Entered Cash / Paytm / PhonePe (paise) | Expected Cash / Online difference (rupees) | Observed |
| --- | ---: | ---: |
| 150 / 25 / 24 | +1 / -1 | +1 / -1 |
| 49 / 75 / 75 | -1 / +1 | -1 / +1 |
| 50 / 25 / 25 | 0 / 0 | 0 / 0 |
| 100 / 60 / 40 | 0 / 0 | 0 / 0 |

Discarding the detached draft and reopening the saved row retained the original `111 / 222 / 333` paise. Explicit Save retained the same row identity and stored `149 / 76 / 73` paise as separate fields rather than the rounded comparison values. These are numeric checks only; signs and unsigned-zero rendering, browser cancellation and durable browser storage were not exercised. Legacy execution remains gated by OPEN-008 and accounting acceptance remains open.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l15_cashbook_numeric_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L15-NUMERIC-c32eafb-focused](../phase2-local/target/L15-NUMERIC-c32eafb-focused.log): 1 passed, 0 failed, 0 ignored, 269 filtered out.
- [L15-NUMERIC-c32eafb-offline](../phase2-local/target/L15-NUMERIC-c32eafb-offline.log): 389 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and build passed; Rust LSP timed out.

#### Sales history filters and drilldown inputs

**L04-03 / L04-04: PASS (development only)** at Rust commit `1c1cf9a3f1f613dbca8fb1fe89f402906af4bbea`. One [offline replay](../phase2-local/src/confirmation/tests/l04_sales_history_replay.rs) used an in-memory `ConfirmationFixture` and a test-only Sales projection traced to [the case expectations](PHASE1_CASES.md#L149) and [drilldown expectations](PHASE1_CASES.md#L359). Fixed Today: `2025-03-15`. Bill IDs below are fixture draft IDs; the posting sequence `41, 12, 73, 36, 24, 58` stands in for creation timestamps. Document dates deliberately disagree with that sequence. Separately dated customer receipts have replay IDs assigned by append position. The projection resolves stable fixture party names by customer ID; it does not cover historical name changes.

Every observed sequence and amount matched the literal expectation. The active column applies to **both** Profit and Collected; the receipt column applies only to Collected. Amounts are exact paise, ordered as **Sales / bill paid / receipt paid / Collected / Est. Profit**.

| Filter | Sales row IDs (ordered) | Active bill IDs (ordered) | Matched receipt IDs (ordered inputs) | Expected and observed exact paise |
| --- | --- | --- | --- | --- |
| Today | 58, 24, 36 | 58, 36 | 4 | 2548 / 149 / 151 / 300 / 998 |
| This Month (no upper cutoff) | 58, 24, 36, 73, 41 | 58, 36, 73, 41 | 1, 2, 4, 5, 6, 7 | 6145 / 447 / 499 / 946 / 2445 |
| All | 58, 24, 36, 73, 12, 41 | 58, 36, 73, 12, 41 | 1, 2, 3, 4, 5, 6, 7 | 7194 / 522 / 550 / 1072 / 2894 |
| Custom Mar 1–15, inclusive | 58, 24, 36, 73 | 58, 36, 73 | 1, 2, 4, 6, 7 | 4047 / 298 / 450 / 748 / 1547 |
| Custom start only, Mar 10 | 58, 24, 36, 41 | 58, 36, 41 | 1, 2, 4, 5, 6 | 4646 / 298 / 449 / 747 / 1896 |
| Custom end only, Mar 15 | 58, 24, 36, 73, 12 | 58, 36, 73, 12 | 1, 2, 3, 4, 6, 7 | 5096 / 373 / 501 / 874 / 1996 |
| Reversed custom bounds | none | none | none | 0 / 0 / 0 / 0 / 0 |
| Mar 1–15, party `gRoVe` | 36, 73 | 36, 73 | 1, 6, 7 | 2548 / 298 / 250 / 548 / 998 |
| Mar 1–15, item `LiMe` | 58, 24, 73 | 58, 73 | none | 2998 / 149 / 0 / 149 / 1098 |
| Mar 10 only | none | none | 1, 2, 6 | 0 / 0 / 249 / 249 / 0 |
| Mar 10 only, party `gRoVe` | none | none | 1, 6 | 0 / 0 / 200 / 200 / 0 |

On Today, cancelled bill `24` remained visible with a saved 1248-paise remainder and a zero shown balance; it was excluded from the active totals and inputs. The active packet count was 2. Linked bill `36` used the current Grove ledger balance of 3849 paise, not its own 900-paise remainder. Guest bill `58` showed its own 1499-paise remainder. The Manager profit request returned `Err(AccessDenied)` without a payload at the test projection boundary; the reads did not change fixture state.

For Mar 1–15, Collected detail sorted in this order by document date then synthetic creation order: receipt `4` (151p), bill `36` (149p), receipt `6` (51p), receipt `2` (49p), receipt `1` (149p), receipt `7` (50p), bill `73` (149p). The test asserted the following fixture display strings separately from the exact amounts:

| Output | Exact paise | Expected and observed local rounded display |
| --- | --- | --- |
| Collected tile: bills / receipts / total | 298 / 450 / 748 | ₹3 / ₹5 / ₹7 |
| Collected detail: Cash / Online / total | 397 / 351 / 748 | ₹4 / ₹4 / ₹7 |
| Collected detail rows in the order above | 151 / 149 / 51 / 49 / 149 / 50 / 149 | ₹2 / ₹1 / ₹1 / ₹0 / ₹1 / ₹1 / ₹1 |
| Est. Profit: revenue / frozen cost / profit | 4047 / 2500 / 1547 | ₹40 / ₹25 / ₹15 |

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l04_sales_history_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L04-HISTORY-1c1cf9a-focused](../phase2-local/target/L04-HISTORY-1c1cf9a-focused.log): 1 replay passed, 0 failed, 0 ignored, 270 filtered out.
- [L04-HISTORY-1c1cf9a-offline](../phase2-local/target/L04-HISTORY-1c1cf9a-offline.log): 390 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and build passed; editor diagnostics timed out.

The replay is limited to synthetic regular bills with known FIFO costs and local whole-rupee formatting. It does not exercise browser controls, rendered drilldowns, production authorization, or legacy display parity. The frozen HTML remained unserved under OPEN-008. Browser parity and accounting acceptance remain open.

#### Monthly Reports and customer ranking

**L16-01: PASS (development only)** at Rust commit `8f8f1092ef3a7917f5df23f271293446d0079d3c`. The [offline replay](../phase2-local/src/confirmation/tests/l16_monthly_reports_replay.rs) reuses the L04 Sales history and extends the L15 saved Cash Book fixture, preserving its counted Cash, Paytm and PhonePe amounts. The [test-only projection](../phase2-local/src/confirmation/tests/l16_reports_projection.rs) follows the [source-derived L16-01 expectations](PHASE1_CASES.md#L246); it is not a production Reports endpoint. With Today fixed to `2025-04-15`, the report offered November 2024 through April 2025, oldest first, from the calendar rather than transaction presence. This is a six-month table, not an interactive month picker.

Expected and observed matched throughout. Bill IDs are fixture draft IDs; receipt IDs are one-based append positions. Exact figures are **Sales / frozen cost / Gross Profit / Goods (cash) / Misc / Salary / Net**, in paise. Display figures are **bill count / Sales / Gross Profit / Goods (cash) / Other Expenses / Net**; frozen cost is a reconciliation input, not a displayed P&L column.

| Offered month | Active bill IDs | Saved Cash Book IDs | Receipt IDs excluded from P&L | Exact paise | Local rounded display |
| --- | --- | --- | --- | --- | --- |
| 2024-11 | none | none | none | 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 / ₹0 / ₹0 / — / ₹0 / ₹0 |
| 2024-12 | none | none | none | 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 / ₹0 / ₹0 / — / ₹0 / ₹0 |
| 2025-01 | none | none | 8 | 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 / ₹0 / ₹0 / — / ₹0 / ₹0 |
| 2025-02 | 12 | 0 | 3 | 1049 / 600 / 449 / 1201 / 2302 / 3403 / -5256 | 1 / ₹10 / ₹4 / ₹12 / ₹57 / ₹-53 |
| 2025-03 | 36, 41, 58, 73 | none | 1, 2, 4, 5, 6, 7 | 6145 / 3700 / 2445 / 0 / 0 / 0 / 2445 | 4 / ₹61 / ₹24 / — / ₹0 / ₹24 |
| 2025-04 | 80 | 1 | none | 15025 / 5000 / 10025 / 999 / 25 / 25 / 9975 | 1 / ₹150 / ₹100 / ₹10 / ₹1 / ₹100 |

Backdated bill `12` contributed to its February document month; bill `80` dated April 25 contributed to April despite the April 15 clock. Cancelled bill `24` contributed neither revenue nor cost. January's 100-paise receipt `8` affected its customer account but left the month's P&L at zero. Bill `81` dated October 20 was outside the offered months but still contributed to all-time ranking.

April Net was calculated as `10025 - 25 - 25 = 9975` paise before rounding. Gross Profit and Other Expenses displayed as ₹100 and ₹1, but Net displayed as ₹100, not ₹99. Goods (cash) of 999 paise displayed as ₹10 separately and was not deducted again.

| All-time rank by stored name | Contributing bill IDs (customer ID) | Exact paise | Local rounded display |
| --- | --- | ---: | ---: |
| Synthetic Grove | 36, 41, 73 (`grove`); 80 (`same-name`) | 19671 | ₹197 |
| Synthetic Ancestor | 81 (`ancestor`) | 15025 | ₹150 |
| Synthetic Guest | 58 (unlinked) | 1499 | ₹15 |
| Synthetic Orchard | 12 (`orchard`) | 1049 | ₹10 |

The two Synthetic Grove accounts remained separate: ledger closing and customer due each read back as 3849 paise for `grove` and 14925 paise for `same-name`. Manager received `Err(AccessDenied)` at the test projection boundary, before any report payload was assembled. The report reads left the Sales and Cash Book fixtures unchanged.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l16_01_monthly_reports_synthetic_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L16-REPORTS-8f8f109-focused](../phase2-local/target/L16-REPORTS-8f8f109-focused.log): 1 passed, 0 failed, 0 ignored, 271 filtered out.
- [L16-REPORTS-8f8f109-offline](../phase2-local/target/L16-REPORTS-8f8f109-offline.log): 391 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser rendering, hosted role authorization, legacy parity and accounting acceptance were not established by this replay.

#### Dead stock cutoff across midnight and DST

**L16-02: PASS (development only)** at `5f3b55be739c03b4737502f752c50a57ff57b3b7`. The [test-only Rust replay](../phase2-local/tests/l16_02_replay.rs) models the [L16-02 source rule](PHASE1_CASES.md#L247) for these clock windows: derive `todayISO()` from the local clock; parse that date at UTC midnight; use local `getDate()`/`setDate()` to shift back 30 calendar days; take the date from the shifted UTC ISO string. A separate [native Date probe](../phase2-local/tests/l16_02_date_probe.mjs) agreed on all local dates and shifted instants under Node `v24.20.0`, tzdata `2026c`.

| Zone | Frozen clock (UTC) | Local `todayISO()` | Shifted instant (UTC) | Cutoff (expected = observed) | Expected eligible IDs | Observed eligible IDs |
| --- | --- | --- | --- | --- | --- | --- |
| Asia/Kolkata | 2024-11-03T18:29:59Z | 2024-11-03 | 2024-10-04T00:00:00Z | 2024-10-04 | 1, 4, 5, 8 | 1, 4, 5, 8 |
| Asia/Kolkata | 2024-11-03T18:30:00Z | 2024-11-04 | 2024-10-05T00:00:00Z | 2024-10-05 | 1, 2, 4, 5, 8 | 1, 2, 4, 5, 8 |
| America/New_York | 2024-11-03T05:59:59Z | 2024-11-03 | 2024-10-04T00:00:00Z | 2024-10-04 | 1, 4, 5, 8 | 1, 4, 5, 8 |
| America/New_York | 2024-11-03T06:00:00Z | 2024-11-03 | 2024-10-04T00:00:00Z | 2024-10-04 | 1, 4, 5, 8 | 1, 4, 5, 8 |
| America/New_York | 2024-11-04T12:00:00Z | 2024-11-04 | 2024-10-04T23:00:00Z | 2024-10-04 | 1, 4, 5, 8 | 1, 4, 5, 8 |

The Kolkata pair straddled local midnight. The New York transition pair straddled fall-back: the clock offset changed from UTC-04:00 to UTC-05:00, but the parsed date-only anchor was still UTC-04:00 in both runs. On November 4 the parsed anchor was UTC-05:00 and the shifted instant was in UTC-04:00. Subtracting 30 UTC days from that anchor instead gives `2024-10-05`, which would incorrectly admit product `2` for that run.

Stocked products `1`, `2`, `3` and `9` had last active sale document dates October 3, 4, 5 and 6, respectively. Equality was excluded: `2` did not qualify at the October 4 cutoff and `3` did not qualify at the October 5 cutoff. Product `2` also had an October 1 bill created on November 2, later than its October 4 bill; the later-created bill did not determine the last sale date. Reversing sale iteration produced the same IDs at every clock. The positive-stock controls `4` (never sold), `5` (only a cancelled November 1 sale) and `8` (only an undated active sale) qualified. Products `6` and `7`, with on-hand `0` and `-1`, did not qualify.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l16_02_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
cd /Users/apple/ALM && TZ=Asia/Kolkata node /Users/apple/ALM/phase2-local/tests/l16_02_date_probe.mjs
cd /Users/apple/ALM && TZ=America/New_York node /Users/apple/ALM/phase2-local/tests/l16_02_date_probe.mjs
```

- [L16-DEAD-5f3b55b-focused](../phase2-local/target/L16-DEAD-5f3b55b-focused.log): 1 passed, 0 failed, 0 ignored.
- [L16-DEAD-5f3b55b-offline](../phase2-local/target/L16-DEAD-5f3b55b-offline.log): 392 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and offline build passed. Rust editor diagnostics timed out; no JavaScript language server was installed.
- Native Date cross-checks passed: [Kolkata](../phase2-local/target/L16-DEAD-5f3b55b-node-kolkata.log) and [New York](../phase2-local/target/L16-DEAD-5f3b55b-node-new-york.log).

The replay used seeded on-hand values and checked selection, not stock derivation, valuation, report rendering, or cross-platform timezone parity. The frozen HTML remained unchanged and unserved under OPEN-008; legacy browser parity and accounting acceptance were not established.

#### Dead stock cancellation, valuation, and identity

**L16-03 / L16-04 / L16-05: PASS (development only)** at `c6ede1f9875e6d800b8eaa1e7a37c8e013d0d5ed`. One [offline replay target](../phase2-local/tests/l16_dead_stock_replay.rs) ran three [case assertions](../phase2-local/tests/l16_dead_stock_replay/cases.rs) against a shared [test-only report model](../phase2-local/tests/l16_dead_stock_replay/model.rs). It reused the verified L16-02 cutoff `2024-10-04` as a fixture input and reran the original cutoff test separately in the same focused command. The [source-derived cases](PHASE1_CASES.md#L248) describe the frozen report branches; no app path was run. Quantities below are packets; amounts are rupees.

**L16-03: cancellation and undo.** Product `p` opened at 5 packets with current buy rate ₹4. `old-bill` was dated `2024-10-03` for 1 packet; `recent-bill` was dated `2024-10-05` for 2 packets. Undo toggled the cancellation flag on the same seeded bill. It did not invoke an operator action or posting API.

| History | Expected last active / on hand / visible IDs | Observed last active / on hand / visible IDs | Observed raw candidate / displayed Locked Value |
| --- | --- | --- | --- |
| Old active + recent cancelled | 2024-10-03 / 4 / `p` | 2024-10-03 / 4 / `p` | ₹16 / ₹16 |
| Recent cancelled only | never / 5 / `p` | never / 5 / `p` | ₹20 / ₹20 |
| Old active + recent restored | 2024-10-05 / 2 / none | 2024-10-05 / 2 / none | ₹8 / no row |

**L16-04: valuation.** All 16 ranking candidates had positive seeded opening stock and no active sale date. FIFO is a separate contrast using seeded layers, not a report input. Raw values preserve fractional paise for the ranking check.

| Product(s) | On hand / current buy rate | Expected raw / displayed / FIFO | Observed raw / displayed / FIFO |
| --- | --- | --- | --- |
| `h01` through `h12`, each | 1 / ₹20 | ₹20 / ₹20 / ₹1 | ₹20 / ₹20 / ₹1 |
| `higher` | 1 / ₹10.75 | ₹10.75 / ₹11 / ₹2 | ₹10.75 / ₹11 / ₹2 |
| `fraction` | 2.5 / ₹4.25 | ₹10.625 / ₹11 / ₹65 | ₹10.625 / ₹11 / ₹65 |
| `missing` | 1 / absent | ₹0 / ₹0 / ₹7 | ₹0 / ₹0 / ₹7 |
| `zero` | 1 / ₹0 | ₹0 / ₹0 / ₹9 | ₹0 / ₹0 / ₹9 |

The first 15 IDs matched exactly: forward `h01` through `h12` ascending, then `higher`, `fraction`, `missing`; with input reversed, `h12` through `h01` descending, then `higher`, `fraction`, `zero`. `higher` outranked `fraction` on raw value despite equal display and opposite FIFO order. Exact ties retained input order, including at the cap. The nonempty-input control excluded `no-stock` (0 on hand, never sold) and `boundary` (1 on hand, last active `2024-10-04`): expected and observed no IDs and the modeled empty message `No dead stock 👍`.

**L16-05: identity.** All seeded sales were active and dated `2024-10-05`. The direct and crate scenarios each started from the same baseline. The crate contained the same renamed product with its retained ID, not a separate product. No ambiguous reference was relinked by name alone.

| Line variant | Expected key / packets / matched current ID | Observed key / packets / matched current ID |
| --- | --- | --- |
| Direct line with old name and retained ID | `renamed` / 1 / `renamed` | `renamed` / 1 / `renamed` |
| Crate with retained-ID component | `renamed` / 1 (2 × 0.5) / `renamed` | `renamed` / 1 (2 × 0.5) / `renamed` |
| Stale stored ID with matching current name | `stale` / 0.5 (5 pieces, pack size 10) / none | `stale` / 0.5 / none |
| No ID and no matching name | `n:Unmatched` / 1 / none | `n:Unmatched` / 1 / none |
| No ID, unique exact-name control | `control` / 1 / `control` | `control` / 1 / `control` |

Expected and observed baseline rows were `renamed`, `current`, each with 3 on hand, no last active date, and ₹3 Locked Value. Either matching variant moved `renamed` to 2 on hand with last date `2024-10-05`, excluding it; only `current` remained visible at 3 packets and ₹3. The unique-name control `control` had 2 on hand and last date `2024-10-05`, so was excluded in all variants. The stale key used a name lookup for packet size only; the lookup did not transfer sales activity or quantities to `current`.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l16_dead_stock_replay --test l16_02_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L16-DEAD-FOLLOWUP-c6ede1f-focused](../phase2-local/target/L16-DEAD-FOLLOWUP-c6ede1f-focused.log): 4 passed (three follow-up cases and the L16-02 regression), 0 failed, 0 ignored.
- [L16-DEAD-FOLLOWUP-c6ede1f-offline](../phase2-local/target/L16-DEAD-FOLLOWUP-c6ede1f-offline.log): 395 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unchanged and unserved under OPEN-008. This source-model replay did not test browser rendering, duplicate-name migration, or accounting acceptance.

#### Customer opening balance, deletion guard, and ledger views

**L14-04 / L14-02 with scoped L02-04 and L05-01 controls: PASS (development only)** at `e1f531acbd6788c5bd5d321857833abe5c59e04c`. The [offline replay](../phase2-local/tests/l14_opening_balance_replay.rs) separates [legacy source modeling](../phase2-local/tests/l14_opening_balance_replay/modeled_cases.rs) from [existing Rust application APIs](../phase2-local/tests/l14_opening_balance_replay/rust_seams.rs). The modeled branches trace to the [case inventory](PHASE1_CASES.md#L360) and frozen stamp, balance, ageing and customer editor/ledger source (HTML L961–965, L1114–1173, L2516–2572, L7269–7312, L7367–7411).

**Modeled save, `acct-a`.** Each independent copy started at 1200 paise opening. The fixed `bill-a` dated `2025-02-10` contributed 10000 debit less 2000 paid; `receipt-a` dated `2025-02-12` credited 3000. `cancelled-a` dated `2025-02-11` was excluded. The signed ledger opening row was dated `2025-02-01` UTC and carried zero credit. Ageing used `2025-02-25`; the age-15 bill was in `d15`.

| Input | Expected and observed saved opening / exact due (paise) | Expected and observed rounded due (rupees) | Expected and observed running ledger (opening → bill → receipt, paise) | Expected and observed ageing |
| --- | --- | --- | --- | --- |
| Positive | 4500 / 9500 | 95 | 4500 → 12500 → 9500 | 1500 paise undated opening (`old`, age 999); 8000 paise bill (`d15`) |
| Negative | -7500 / -2500 | -25 | -7500 → 500 → -2500 | No debt rows |
| Zero | 0 / 5000 | 50 | 0 → 8000 → 5000 | 5000 paise bill (`d15`) |
| Blank | 0 / 5000 | 50 | 0 → 8000 → 5000 | 5000 paise bill (`d15`) |

Each successful modeled save replaced the opening value and changed `_v` from 7 to 8, `_m` from `1738368000000` to `1740441600000` (2025-02-25 UTC), and `_d` from `old-device` to `synthetic-device`. The `created` value stayed `1738368000000` (2025-02-01 UTC). No payment was created, and the complete bill and receipt records stayed equal. `acct-b` kept its 1100-paise opening, original stamp, and 7600-paise due (`bill-b`: 8000 total / 1000 paid; `receipt-b`: 500). Its ledger and ageing also stayed unchanged.

**Same-name control.** The frozen editor rejects an existing duplicate normalized name before saving. With both IDs named `Synthetic`, all four opening-save attempts were expected and observed to be rejected without changing either account, its stamp, bills, or receipts. Due remained **6200 paise for `acct-a`** and **7600 paise for `acct-b`**; their linked ledgers and ageing were separate. The successful variants above therefore use a distinct name on the control account rather than bypassing the editor guard.

**Modeled deletion guard, `acct-a`.** `OfferConfirmation` means not blocked by the guard, not deleted. No confirmation action, contact deletion, or tombstone was tested.

| Exact balance (expected = observed, paise) | Guard balance (expected = observed, rupees) | Expected and observed outcome |
| --- | --- | --- |
| 9500 | 95 | Blocked: debt |
| -2500 | -25 | Blocked: advance |
| 0 | 0 | Confirmation offered |
| 25 | 0 | Confirmation offered despite nonzero debt |
| 50 | 1 | Blocked: debt |
| -50 | 0 | Confirmation offered; JavaScript negative zero compares equal to zero |

**Existing Rust path.** Used `ConfirmationFixture` to register, review and confirm named bills, record standalone receipts, read exact due and linked ledgers, and request both bill-preview modes. No opening save, customer stamp, ageing or deletion operation used the Rust application path.

| Customer / scenario | Expected and observed API result (paise) |
| --- | --- |
| `acct-a`, invoice 1 dated Feb 10, after Feb 12 receipt of 3000 | Bill due 8000; bill-date preview 8000; latest and current due 5000 |
| `acct-a`, after a further 1000-paise receipt posted later but dated Feb 9 | Document-order running balances: Feb 9 -1000, Feb 10 7000, Feb 12 4000. Bill-date preview stayed 8000; latest and exact due were 4000 |
| `acct-b`, invoice 2, same name as `acct-a` | Own separate ledger with one fully paid bill and zero closing/current due |
| Unlinked `Local - Neighbor`, invoice 3 | Total and paid 10000; zero due and zero balance in both preview modes, with no toggle. Linked-ledger lookup returned `InvalidDraft`; rereading both named ledgers after posting returned unchanged rows |

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_opening_balance_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-ACCOUNT-e1f531a-focused](../phase2-local/target/L14-ACCOUNT-e1f531a-focused.log): 4 passed, 0 failed, 0 ignored.
- [L14-ACCOUNT-e1f531a-offline](../phase2-local/target/L14-ACCOUNT-e1f531a-offline.log): 399 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unchanged and unserved under OPEN-008. No legacy editor, browser persistence or deletion was executed. Browser parity and accounting acceptance remain open.

#### Customer rename snapshot and local delta merge

**L14-05: PASS (development only)** at `dba9932ec1763fa56c1945a280c13401d2419885`. The [offline replay](../phase2-local/tests/l14_05_customer_rename_replay.rs) exercised a [typed source model](../phase2-local/tests/l14_05_customer_rename_replay/model.rs), traced to the [rename cases](PHASE1_CASES.md#rename-cascades-and-other-source-order-cases) and frozen HTML L717–734, L776–783, L961–965, L7292–7313 and L9472–9562. No Rust application rename or sync path ran. The snapshot check used an in-memory JSON serialization and independent decode, not a browser storage write or reload. The payload was serialized and decoded separately and never sent.

**Save and decoded snapshot.** Expected and observed matched by ID. Stamps below are `(_m, _v, _d)`. A duplicate-name control (` shared `) was rejected without mutation; the permitted input `  Renamed A  ` was trimmed and saved on `c-a`.

| Collection and ID | Link | Name before → after (expected = observed) | Stamp before → after (expected = observed) |
| --- | --- | --- | --- |
| customers `c-a` | target | `Shared` → `Renamed A` | `(92, 4, seed)` → `(200, 5, sender)` |
| customers `c-b` | same-name control | `Shared` → `Shared` | `(91, 3, seed)` unchanged |
| sales `s-a`, `s-cancel`; payments `p-a` | `c-a` | `Shared` → `Renamed A` | `(_m, 4, seed)` unchanged, with `_m` seeded to 99, 100, or 101 per copy |
| sales `s-b`; payments `p-b` | `c-b` | `Shared` → `Shared` | `(80, 2, seed)`; `(82, 2, seed)` unchanged |
| sales `s-none`; payments `p-none` | no ID | `Shared` → `Shared` | `(81, 2, seed)`; `(83, 2, seed)` unchanged |

The cancelled flag on `s-cancel` remained set. Snapshot `meta.rev` advanced from 7 to 8 while the meta stamp `(90, 2, seed)` remained unchanged. Only `c-a` acquired a new modification stamp.

**Unsent payload.** Each independent copy used fixed `pushedM=100`. All three linked labels changed in every snapshot, but payload IDs matched the strict `_m > pushedM` expectation:

| Linked `_m` | Expected payload IDs | Observed payload IDs |
| --- | --- | --- |
| 99 (below) | `c-a` | `c-a` |
| 100 (equal) | `c-a` | `c-a` |
| 101 (above) | `s-a`, `s-cancel`, `p-a`, `c-a` | `s-a`, `s-cancel`, `p-a`, `c-a` |

The above-watermark dependents were already eligible before Save with `Shared`. After Save, they carried `Renamed A` with the same `(101, 4, seed)` stamps and corresponding outer envelope fields. Control records were omitted in all copies.

**Local receiver.** On every independent receiver `c-a` became `Renamed A` at `(200, 5, sender)`. The linked rows `s-a`, `s-cancel`, and `p-a` were evaluated separately; same-name and unlinked controls remained unchanged.

| Input to receiver | Expected and observed linked names and stamps |
| --- | --- |
| Customer-only payload | All stayed `Shared`, `(99, 4, seed)`; customer arrival did not rewrite history |
| Dependent version 4 vs receiver version 4; incoming `_m=101` vs receiver `_m=50` | All stayed `Shared`, `(50, 4, receiver)`; newer `_m` did not break the version tie |
| Dependent version 4 vs receiver version 3; incoming `_m=101` vs receiver `_m=300` | All became `Renamed A`, `(101, 4, seed)`; higher version applied despite the older `_m` |

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_05_customer_rename_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-RENAME-dba9932-focused](../phase2-local/target/L14-RENAME-dba9932-focused.log): 1 passed, 0 failed, 0 ignored.
- [L14-RENAME-dba9932-offline](../phase2-local/target/L14-RENAME-dba9932-offline.log): 400 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unchanged and unserved under OPEN-008. No sync request, hosted merge, browser persistence, or cloud rebuild was run. This does not establish browser parity, deployed convergence, or accounting acceptance.

#### Supplier rename snapshot and local delta merge

**L09-05: PASS (development only)** at `0e3f55b890c4f079817d7503e85fe7201a357ae9`. One [offline replay](../phase2-local/tests/l09_05_supplier_rename_replay/assertions.rs) exercised the [test-only source model](../phase2-local/tests/l09_05_supplier_rename_replay/model.rs), traced to [L09-05](PHASE1_CASES.md#L364) and frozen HTML L757–774, L10483–10498 and L9451–9562. The frozen supplier Save branch rejects blank input but has no duplicate-name guard. The model rejected blank input without mutation and accepted ` Target ` even though `s2` already had that name. Zero, null and duplicate-column states were seeded inputs, not created through an editor. No legacy editor or Rust application rename or sync path ran.

**Save and decoded full fixture snapshot.** Stamps are `(_m, _v, _d)`. Each copy's snapshot was serialized and decoded independently of its unsent payload. Expected and observed matched throughout.

| Record ID | Expected and observed name before → after | Expected and observed stamp before → after |
| --- | --- | --- |
| Supplier `s1` | `Old` → `Target` | `(90, 2, seed)` → `(200, 3, sender)` |
| Supplier `s2` | `Target` unchanged | `(91, 1, seed)` unchanged |
| Purchase `p1` and PO `o1` | `Old` → `Target`; `supId=other` unchanged | `(_m, 3, seed)` unchanged; `_m` is 99, 100 or 101 per copy |
| Purchase `p2` and PO `o2` | `old` unchanged despite `supId=s1` | `(99, 3, seed)` unchanged |

Only `s1` received a new stamp. `meta.rev` advanced from 7 to 8 while its stamp `(90, 2, seed)` stayed unchanged. The exact-name rewrite and first-occurrence deduplication changed raw `priceSuppliers` from `[Target, Old, Other, Old, other, Other]` to `[Target, Other, other]`; the `Other`/`other` case distinction remained.

| Product ID | Quotes before (`Old` / `Target`) | Expected quotes after | Observed quotes after |
| --- | --- | --- | --- |
| `missing-target` | 11 / absent | `Target: 11`, no `Old` | `Target: 11`, no `Old` |
| `null-target` | 12 / null | `Target: 12`, no `Old` | `Target: 12`, no `Old` |
| `existing-target` | 13 / 44 | `Target: 44`, no `Old` | `Target: 44`, no `Old` |
| `zero-target` | 14 / 0 | `Target: 0`, no `Old` | `Target: 0`, no `Old` |
| `null-old` | null / 55 | `Old: null`, `Target: 55` | `Old: null`, `Target: 55` |

Product stamps were unchanged: `missing-target` kept `(_m, 3, seed)` at the tested cutoff position, and the other products kept `(99, 3, seed)`.

**Unsent payload.** `pushedM=100` on independent copies. `priceSuppliers` was absent from every payload; the revision increase alone did not include `meta` either.

| `_m` of `o1`, `p1` and `missing-target` | Expected and observed ordered payload collection/IDs |
| --- | --- |
| 99 | `suppliers/s1` |
| 100 | `suppliers/s1` |
| 101 | `purchaseOrders/o1`, `purchases/p1`, `products/missing-target`, `suppliers/s1` |

At 101, the three dependents were already eligible before Save with the old labels and quote. After Save they carried the updated values with unchanged nested stamps `(101, 3, seed)` and matching outer envelope fields.

**Local model receiver.** On every fresh receiver, `s1` became `Target` at `(200, 3, sender)`. The raw list remained `[Target, Old, Other, Old, other, Other]`, and controls and untransmitted quote maps stayed at receiver seed values.

| Decoded input | Expected and observed dependent result for `o1`, `p1`, `missing-target` |
| --- | --- |
| Supplier only, with local dependents `(50, 3, seed)` or `(300, 2, seed)` | No replacement: labels stayed `Old`, quote stayed `Old: 11`, and local stamps stayed unchanged |
| Full payload; incoming version 3 equals local version 3 at `_m=50` | All three rejected despite incoming `_m=101`: labels and quote stayed old at `(50, 3, seed)` |
| Full payload; incoming version 3 exceeds local version 2 at `_m=300` | All three replaced despite incoming `_m=101` being older: labels became `Target`, quote became `Target: 11` with no `Old`, and stamps became `(101, 3, seed)` |

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l09_05_supplier_rename_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L09-RENAME-0e3f55b-focused](../phase2-local/target/L09-RENAME-0e3f55b-focused.log): 1 passed, 0 failed, 0 ignored.
- [L09-RENAME-0e3f55b-offline](../phase2-local/target/L09-RENAME-0e3f55b-offline.log): 401 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and offline build passed; Rust editor diagnostics timed out.

No sync request was sent. The frozen HTML remained unchanged and unserved under OPEN-008. This replay does not establish browser persistence, deployed convergence, or accounting acceptance.

#### Supplier rename on a partially received PO

**L09-05: PASS (local Rust application workflow, development only)** at `e96c4fe32be36013882cbbf498f281a890eaf8bc`. The [workflow](../phase2-local/src/confirmation/purchase_orders/supplier_prices/rename_test.rs) used the existing Rust purchase Save, receiving, supplier directory, Receive Goods, Stock Lots and price-comparison paths in a synthetic `ConfirmationFixture`, then called the [Owner-gated supplier rename](../phase2-local/src/confirmation/purchase_orders/supplier_prices/rename.rs). This is separate from the [source-model replay](#supplier-rename-snapshot-and-local-delta-merge) at `0e3f55b`. Money below is in paise.

| Readback by stable ID | Before Save | Expected and observed after Save |
| --- | --- | --- |
| Supplier `2`; controls `0`, `1` | `2=Old`, `0=New`, `1=Other` | `2=New`, `0=New`, `1=Other`; all IDs retained |
| PO `0`, dated `2025-01-02`, supplier ID `2`, product ID `1` | Stored label `Old`; Receive Goods also read `Old`. Ordered 4, received 2, outstanding 2 | Stored label rewritten to `New`; Receive Goods read `New`. Same IDs and quantities; the complete PO equaled the prior record with only `supplier_name` changed |
| PO `0`, receipt history | One receipt dated `2025-01-03`, quantity 2 at 120 per unit | Same receipt, unchanged; no receipt added by Rename |
| PO `1`, supplier ID `0` (seeded historical label) | Stored `Old`, even though supplier `0` resolves to `New` | Stored `New`; supplier ID still `0` |
| PO `2`, supplier ID `2` (seeded case variant) | Stored `old` | Entire PO unchanged: stored `old` even though supplier `2` now resolves to `New` |
| Product `1`, billing stock and FIFO | 12 on hand; opening 10 at 100 (value 1000), dated receipt 2 at 120 (value 240); total FIFO value 1240 | Same stock, both lots and total FIFO value; purchase-cost records unchanged |
| Product `1`, target quote present | `Old:120, New:90, Other:110` | `New:90, Other:110`; target wins and old key is removed |
| Product `2`, target quote missing | `Old:70, Other:75` | `Other:75, New:70`; old quote moved and old key is removed |
| Product `3`, target zero seeded directly | `Old:80, New:0` | `New:0`; stored zero wins and old key is removed |

The Manager rename returned `AccessDenied`; the entire fixture remained equal before the Owner call. After the rename, price comparison showed columns `0=New` and `1=Other` for product `1`, with quotes 90 and 110. Deduplicating the displayed name did not merge the two `New` supplier records. The stored PO label assertions, including the mismatched and case-variant controls, distinguish a write from a display name resolved from a supplier ID.

Ordinary quotes were entered through the Rust price-comparison API or generated by receiving. The zero was seeded because the edit API clears zero, although the quote store can represent it. Explicit null quotes and the legacy raw `priceSuppliers` list have no equivalent in this Rust model; they are not application passes. Legacy stamps, snapshot/delta and receiver cases remain source-model evidence only. No sync request was sent, and the frozen HTML remained unchanged and unserved under OPEN-008.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l09_05_rename -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L09-RENAME-APP-e96c4fe-focused](../phase2-local/target/L09-RENAME-APP-e96c4fe-focused.log): 1 passed, 0 failed, 0 ignored, 272 filtered out.
- [L09-RENAME-APP-e96c4fe-offline](../phase2-local/target/L09-RENAME-APP-e96c4fe-offline.log): 402 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

#### Receive Payment customer selection and ledger readback

**L14-03 with scoped L14-01/02 ledger checks: PASS (development only)** at `b81c731179ab0e03b5aecd861d24db2ace516f9f`. The [offline replay](../phase2-local/tests/l14_receive_payment_replay.rs) models [focused input, suggestions, keyboard and pointer selection, and the missing-customer Save guard](../phase2-local/tests/l14_receive_payment_replay/picker.rs) from [L14-03](PHASE1_CASES.md#L393) and frozen HTML L1604–1665 and L5860–5959. With a valid amount and date, modeled Save passes the chosen typed ID directly to the existing Rust `record_customer_receipt`; the readback uses `customer_ledger` and `customer_due` on the synthetic fixture. No production code changed.

| Modeled input and Save | Expected and observed selection and outcome |
| --- | --- |
| Padded `Solo Market` | Trimmed unique exact match resolved `solo` without a pick. A valid Cash request for 310 paise submitted as Manager returned `AccessDenied` from Rust without recording a receipt or changing the ledgers. |
| Wrong-case `solo market` then Enter | `solo` was suggested, but no ID was selected. Save warned `Pick a customer`; zero receipt-writer calls. |
| Exact duplicate `Twin Market` then Enter with no highlight | `twin-east` and `twin-west` were suggested, but neither was selected. Save warned `Pick a customer`; zero receipt-writer calls. |
| Previously picked `twin-west`, then retyped padded duplicate and pressed Enter with no highlight | Typing cleared the stale selection; the same suggestions remained. Save warned `Pick a customer`; zero receipt-writer calls. |

The two successful Owner paths started from independent copies. The existing bills were dated `2025-03-14`; the standalone receipts were dated `2025-03-17`. Amounts are exact paise. Each successful Save added one receipt row with zero debit and the shown credit, without editing the existing bill row.

| Modeled pick | Expected ID / mode / credit | Observed Rust ID / mode / credit | Expected and observed closing before → after (delta) | Existing bill paid before → after |
| --- | --- | --- | --- | --- |
| Arrow Down then Enter on the first duplicate | `twin-east` / Online / 725 | `twin-east` / Online / 725 | 1675 → 950 (-725) | Invoice `1`: 325 → 325 |
| Pointer on the second duplicate | `twin-west` / Cash / 430 | `twin-west` / Cash / 430 | 1490 → 1060 (-430) | Invoice `2`: 510 → 510 |

The other duplicate and `solo` retained their complete ledger rows and closing balances in each Owner copy. All three rejected selection saves and the Manager attempt left all fixture ledger readbacks and posted bill totals unchanged.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_receive_payment_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-PAYMENT-b81c731-focused](../phase2-local/target/L14-PAYMENT-b81c731-focused.log): 3 passed, 0 failed, 0 ignored.
- [L14-PAYMENT-b81c731-offline](../phase2-local/target/L14-PAYMENT-b81c731-offline.log): 405 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unchanged and unserved under OPEN-008. Browser keyboard, pointer and toast behavior were not run; the Rust receipt writes are local fixture evidence, not hosted authorization or browser parity.

#### Customer ledger UTC opening date and estimate ordering

**L14-01 / L14-04 with L05-01 estimate comparison: PASS (development only)** at `067559bf436cae0a9026c7c7fe69913e5bddd8c7`. The [offline replay](../phase2-local/tests/l14_ledger_ordering_replay.rs) follows the [ledger-ordering case](PHASE1_CASES.md#L487) and frozen HTML L4322–4334 and L7367–7411. The test-only timestamp model converted the synthetic Asia/Kolkata creation instant `2025-02-11T00:15:00+05:30` to `2025-02-10T18:45:00Z`; the opening row was dated `2025-02-10`. With `created` absent on a separate copy, the opening date was blank. No browser timezone or host clock was used.

The synthetic account had a modeled opening debit of 2000 paise. B1 and B2 each debited 10000 paise and credited their original paid amounts of 2000 and 1000. Standalone receipts R1, R2 and R3 credited 3000, 400 and 1000 paise respectively. Transaction creation order was B1, R1, B2, R2, R3, at successive UTC minutes from `2025-02-10T19:00:00Z` through `19:04:00Z`. Model input order was B2, B1, R3, R2, R1. R3 was posted last but dated February 9, before the dated opening.

| Path | Expected and observed rows in order (`ID / document date / running paise`) |
| --- | --- |
| Source model, UTC-dated opening | `R3 / 2025-02-09 / -1000`, `opening / 2025-02-10 / 1000`, `B1 / 2025-02-10 / 9000`, `R1 / 2025-02-10 / 6000`, `B2 / 2025-02-10 / 15000`, `R2 / 2025-02-10 / 14600` |
| Source model, absent `created` | `opening / blank / 2000`, `R3 / 2025-02-09 / 1000`, `B1 / 2025-02-10 / 9000`, `R1 / 2025-02-10 / 6000`, `B2 / 2025-02-10 / 15000`, `R2 / 2025-02-10 / 14600` |
| Existing Rust `customer_ledger`, zero-start fixture | `R3 / 2025-02-09 / -1000`, `B1 / 2025-02-10 / 7000`, `R1 / 2025-02-10 / 4000`, `B2 / 2025-02-10 / 13000`, `R2 / 2025-02-10 / 12600` |

The dated opening was not pinned ahead of R3. On February 10 the modeled rows followed creation order, not input order or bill/receipt grouping. The Rust fixture posted the same transactions for `l14-customer` through existing bill confirmation and receipt APIs. B1 was draft `20` / invoice `1`; B2 was draft `10` / invoice `2`. R1/R2/R3 are fixture labels identified in Rust readback by date, mode and amount, not returned receipt IDs. Rust uses invoice and receipt posting order as tie-breakers and has no opening-row or legacy creation-timestamp input on this path.

| Reopened B2 estimate | Expected bill-date / B2 running row / latest (paise) | Observed |
| --- | --- | --- |
| Source model with opening debit, strict creation cutoff | 16000 / 15000 / 14600 | 16000 / 15000 / 14600 |
| Rust `bill_preview` modes, zero-start fixture | 14000 / 13000 / 12600 | 14000 / 13000 / 12600 |

R3 reduced the running balance before B2's row by document date but did not enter B2's bill-date estimate because it was created or posted later. Latest included both R2 and R3. The Rust readback exercised the estimate preview API, not a rendered estimate window.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_ledger_ordering_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-ORDER-067559b-focused](../phase2-local/target/L14-ORDER-067559b-focused.log): 3 passed, 0 failed, 0 ignored.
- [L14-ORDER-067559b-offline](../phase2-local/target/L14-ORDER-067559b-offline.log): 408 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unchanged and unserved under OPEN-008. Browser ordering and estimate rendering remain not run; this is not browser parity or accounting acceptance.

#### Owner opening balance through Rust account and estimate APIs

**L14-04 / L14-01 with L05-01 preview comparison: PASS (local development only)** at `fffd3ce87e87a1c60762d688d8a1761122cde4e4`. The [application replay](../phase2-local/tests/l14_opening_balance_replay/opening_actual.rs) calls `save_customer_opening`, `customer_ledger_with_opening`, `customer_due` and `bill_preview` on the `067559b` transaction fixture. The [zero-opening regression](../phase2-local/tests/l14_ledger_ordering_replay.rs) retained its prior balance assertions. The creation instant `2025-02-11T00:15:00+05:30` to `2025-02-10T18:45:00Z` conversion remains test-modeled; the application accepts an already-normalized UTC `BillDate` or no date. It uses invoice and receipt posting order, not imported `created` timestamps.

The Owner saved successive replacements by stable ID `l14-customer` with the same posted transactions. Each opening row had zero credit. The dates, IDs, debits and credits of the existing transaction rows stayed equal; no receipt was added, B2's original paid amount of 1000 paise was unchanged, and `same-name-other` retained its ledger and due. Manager saves for both a known and a missing ID returned `AccessDenied`; readbacks did not change. An overflowing Owner replacement was also rejected without changing the opening-aware ledger or Latest preview.

| Save input | Expected and observed opening debit (paise) | Expected and observed due / ledger closing (paise) |
| --- | ---: | ---: |
| Positive | 2000 | 14600 |
| Negative | -2000 | 10600 |
| Explicit zero | 0 | 12600 |
| Blank amount | 0 | 12600 |

On fresh copies with a 2000-paise opening, these ordered application rows matched the literal expectations. R1/R2/R3 are fixture receipt labels distinguished by date, mode and amount. B1 is draft `20` / invoice `1`; B2 is draft `10` / invoice `2`. All dated rows other than R3 use `2025-02-10`; R3 uses `2025-02-09`.

| Variant | Expected and observed order with running paise |
| --- | --- |
| UTC-dated opening, `2025-02-10` | `R3:-1000`, `opening:1000`, `B1:9000`, `R1:6000`, `B2:15000`, `R2:14600` |
| Opening date absent | `opening (blank):2000`, `R3:1000`, `B1:9000`, `R1:6000`, `B2:15000`, `R2:14600` |
| No opening configured (regression) | `R3:-1000`, `B1:7000`, `R1:4000`, `B2:13000`, `R2:12600` |

| B2 readback | Expected bill-date / B2 running / Latest (paise) | Observed |
| --- | --- | --- |
| 2000-paise dated or blank-date opening | 16000 / 15000 / 14600 | 16000 / 15000 / 14600 |
| No opening configured | 14000 / 13000 / 12600 | 14000 / 13000 / 12600 |

With an opening, current due and ledger closing also independently read 14600 paise. The date on the opening did not become the estimate's previous transaction date. The replay covers in-memory Rust saves and projections, not browser timestamp handling, editor stamps, ageing, durable persistence, hosted authorization or accounting acceptance. The frozen HTML remained unchanged and unserved under OPEN-008.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_ledger_ordering_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-OPENING-APP-fffd3ce-focused](../phase2-local/target/L14-OPENING-APP-fffd3ce-focused.log): 8 passed, 0 failed, 0 ignored.
- [L14-OPENING-APP-fffd3ce-offline](../phase2-local/target/L14-OPENING-APP-fffd3ce-offline.log): 413 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

#### Customer ageing with saved opening balances

**L02-04 / L14-04: PASS (local Rust development)** at `22cc57bc37dc1fa59f001eed31f9a7e3d6cea6ae`. The Owner-only [ageing report](../phase2-local/src/confirmation/customer_ageing.rs) reads the existing opening-aware ledger. The [replay](../phase2-local/src/confirmation/tests/customer_ageing.rs) traces to [L02-04](PHASE1_CASES.md#L356) and frozen HTML L2455–2462 and L2516–2572. It used the explicit report date `2024-03-31`, calendar-day differences, and the local ledger's posting-order tie-breaker; browser time-zone behavior was not exercised. All amounts below are exact paise.

Customer `a` kept the same bills, receipts, and cancellation state across every opening change. Standalone receipts `+200` and `-50` provided net credit 150. Active draft/invoice `8` (total 100, paid 200) provided another 100 of credit. That overpayment was seeded in private test history, including its effect on the balance index; ordinary payment validation was unchanged and rejected an overpaid review. Cancelled draft/invoice `9` contributed no debt or credit. Same-name accounts `b` and `c` retained separate pools by ID.

| Saved opening for `a` | Expected and observed allocation | Expected and observed visible rows' sum / ledger due |
| --- | --- | ---: |
| Not configured | Pool 250 applied to oldest bill `1`, leaving 750 | 5750 / 5800 |
| +500, saved date `2024-03-30` | Pool 250 applied first to undated opening; remaining 250 visible in `Old` at age 999. Bill `1` unchanged at 1000 | 6250 / 6300 |
| +300, no saved date | Pool 250 applied first to undated opening; remaining 50 omitted. Bill `1` unchanged at 1000 | 6000 / 6100 |
| -100, no saved date | Opening increased the pool to 350; all applied to bill `1`, leaving 650 | 5650 / 5700 |

| Customer `a` draft/invoice | Document date | Expected and observed age / bucket | Remaining paise (no opening / +500 / +300 / -100) |
| --- | --- | --- | --- |
| `1` | `2024-01-30` | 61 / `Old` | 750 / 1000 / 1000 / 650 |
| `2` | `2024-01-31` | 60 / `D60` | 1000 in each variant |
| `3` | `2024-02-29` | 31 / `D60` | 1000 in each variant |
| `4` | `2024-03-01` | 30 / `D30` | 1000 in each variant |
| `5` | `2024-03-15` | 16 / `D30` | 1000 in each variant |
| `6` | `2024-03-16` | 15 / `D15` | 1000 in each variant |

Draft/invoice `7` for `a` started at 50 and never entered the debt list. Customer `b`'s draft/invoice `10` started at 100 and received 50 credit: the residual 50 stayed in ledger due but was not visible. Customer `c`'s draft/invoice `11` started at 101 and received 50 credit: the residual 51 remained visible in `D15` at age 15, and ledger due was also 51. Report-wide `D15 / D30 / D60` totals were `1051 / 2000 / 2000` throughout, with `Old` at `750 / 1250 / 1000 / 650` in opening-variant order. A separate `negative` account control showed a signed receipt pool of -60 increasing bill `1` from 100 to 160 in `D15`; ledger due was 160.

Owner report reads left the complete fixture unchanged. The Manager request returned only `AccessDenied`. The projection inherits the local opening-aware ledger's supported transaction scope. It does not establish browser parity, hosted authorization or accounting acceptance. The frozen HTML stayed unchanged and unserved under OPEN-008.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib customer_ageing -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-AGEING-22cc57b-focused](../phase2-local/target/L02-AGEING-22cc57b-focused.log): 2 passed, 0 failed, 0 ignored.
- [L02-AGEING-22cc57b-offline](../phase2-local/target/L02-AGEING-22cc57b-offline.log): 415 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

#### Opening balances and ageing without sales

**L02-04 / L14-04: PASS (local Rust development)** at `a03f4daac8e745a7414b0c105ab37ec96d124d81`. The [application replay](../phase2-local/src/confirmation/tests/customer_ageing_checks.rs) registered `registered-target` and `registered-other` through the Owner API, with neither bills nor receipts. The opening save and ledger read now recognize those IDs without a sale. Ageing includes registered IDs alongside IDs already known from sales, counting an ID in both sources only once. This extends the earlier [sold-customer replay](#customer-ageing-with-saved-opening-balances) without changing its scope or the ageing threshold.

| Owner opening replacement for `registered-target` (paise) | Expected and observed exact ledger closing and current due (paise) | Expected and observed ageing |
| --- | ---: | --- |
| +51 | 51 | Exactly one undated `Old` row for `registered-target`, age 999, due 51, with no draft or invoice ID |
| +50 | 50 | No visible rows; all bucket totals zero |
| -125 | -125 (credit) | No visible rows; all bucket totals zero |

The opening-aware ledger retained the saved date `2024-03-01` and the signed debit with zero receipt credit. Ageing on `2024-03-31` treated the opening as undated. The transaction-only ledger had no entries. `registered-other` stayed at zero with an empty ledger; the saves created no sale or receipt and left billing stock and the invoice counter unchanged.

A known empty ID returned a zero ledger; the unknown ID `unknown` returned `InvalidDraft` on both ledger reads and on opening save. Manager registration, opening save, ledger and ageing calls returned `AccessDenied`, with no payload or mutation. Registration alone still did not authorize a standalone receipt. The sold-ID deduplication control returned a single row, and the previous ageing tests passed unchanged.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib customer_ageing -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-NOSALE-a03f4da-focused](../phase2-local/target/L02-NOSALE-a03f4da-focused.log): 5 passed, 0 failed, 0 ignored, 273 filtered out.
- [L02-NOSALE-a03f4da-offline](../phase2-local/target/L02-NOSALE-a03f4da-offline.log): 418 passed, 0 failed, 2 ignored. Formatting, all-target Clippy and locked offline build passed; Rust editor diagnostics timed out.

This is an in-memory, ID-only application replay, not a customer editor, durable store or hosted authorization check. The frozen HTML remained unchanged and unserved under OPEN-008; browser parity and accounting acceptance remain open.

#### Standalone Owner receipts without sales

**L02-04 / L14-01 / L15-01: PASS (local Rust development, milestone C)** at `fa8572bc72c3bcb6510aeaba896ca955d6658b1d`. The [application API replay](../phase2-local/src/confirmation/tests/customer_receipts.rs) started with two registered synthetic customers and no bills or receipts. `receipt-target` owed an undated opening debit of 1,000 paise; `receipt-control` had an empty ledger and zero balance. Before the fix the first Owner receipt reproduced `InvalidDraft`. The new guard recognizes registered IDs independently of the existing qualifying-sale path; the `a03f4da` entry above describes the prior limitation, not this build.

| Owner receipt and document date | Expected and observed ledger closing and current due (paise) | Expected and observed ageing on 2024-04-01 | Expected and observed Cash Book for receipt date (paise) |
| --- | ---: | --- | --- |
| Cash 375 on 2024-03-30 | 625 due | One undated `Old` row, age 999, due 625, no draft or invoice ID; other buckets zero | One `Receipt`; Cash 375, Online 0 |
| Online 800 on 2024-03-31 | -175 (175 credit) | No rows; all bucket totals zero | One `Receipt`; Cash 0, Online 800 |

The opening-aware ledger retained the 1,000-paise debit and showed dated credits of 375 and 800 paise in order; the transaction-only ledger showed only those receipts, with no bill entries. Cash Book counts were read after both receipts; 2024-04-01 still had zero entries and zero in both buckets. The control retained its empty ledger and zero due. Stock and FIFO catalog state were unchanged (5 packets on hand), the next invoice stayed at 1, and no sale or invoice was created. In the separate rejection replay, an unknown ID with 375 paise returned `InvalidDraft`, zero paise on a registered ID returned `InvalidAmount`, and a Manager attempt for 375 paise on a registered ID returned `AccessDenied`; the entire fixture was unchanged after each attempt.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib standalone_receipts -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-STANDALONE-fa8572b-focused](../phase2-local/target/L14-STANDALONE-fa8572b-focused.log): 2 passed, 0 failed, 0 ignored, 278 filtered out.
- [L14-STANDALONE-fa8572b-offline](../phase2-local/target/L14-STANDALONE-fa8572b-offline.log): 420 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and the locked offline build passed; Rust editor diagnostics timed out.

This evidence covers only the local in-memory application API. The frozen HTML was unchanged and remained unserved under OPEN-008. Hosted authorization (OPEN-004/006), browser parity, and accounting acceptance remain open.

#### Dashboard month-end Sales and Collected replay

**L02-02 / L02-03: PASS (local Rust development)** at `c7db74d75e0e898efd050ac7a8e3465f18963675`. The [replay](../phase2-local/src/confirmation/tests/l02_dashboard_replay.rs) froze the reporting date at 2024-02-01 and used the legacy [range and aggregate source](../ALM_Business_Manager.html#L2466) and [Collected detail source](../ALM_Business_Manager.html#L3151) as static expectations. Dashboard range selection, totals and drilldown rows are **test-only projections**, not application Dashboard output. The [Rust readbacks](../phase2-local/src/confirmation/tests/l02_dashboard_replay/readback.rs) used current sale, bill preview, customer ledger, and Cash Book day-close APIs. Dates below are inclusive document dates; amounts are exact paise. Expected and observed results matched in each row. Display rounding was not compared.

| Selection / side | Inclusive window | Sales (bill count) | Bill paid | Standalone receipts | Collected | Cash / Online |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Today / current | 2024-02-01 to 2024-02-01 | 3,603 (2) | 450 | 249 | 699 | 448 / 251 |
| Today / comparison | 2024-01-31 to 2024-01-31 | 2,402 (1) | 449 | 0 | 449 | 0 / 449 |
| 7 days / current | 2024-01-26 to 2024-02-01 | 7,206 (4) | 1,248 | 400 | 1,648 | 797 / 851 |
| 7 days / comparison | 2024-01-19 to 2024-01-25 | 3,603 (2) | 478 | 51 | 529 | 278 / 251 |
| This month / current | 2024-02-01 to 2024-02-01 | 3,603 (2) | 450 | 249 | 699 | 448 / 251 |
| This month / comparison | 2024-01-01 to 2024-01-31 | 9,608 (6) | 1,584 | 202 | 1,786 | 786 / 1,000 |

`B(draft, invoice)` identifies an actual local bill. The `r-*` IDs are replay labels bound to unique dated, mode-and-amount-matched ledger credits; neither ledger nor Cash Book provides a standalone receipt ID. All active bills in these windows are part-paid, so each Sales bill contributes exactly one bill-paid row to Collected. The comparison columns below list aggregate inputs, not a comparison modal. Expected and observed IDs matched.

| Selection / side | Sales bill inputs | Collected rows (current detail order), or comparison inputs |
| --- | --- | --- |
| Today and This month / current | `B(5,8)`, `B(81,7)` | `B(5,8)` Online 251; `r-feb01` Cash 249; `B(81,7)` Cash 199 |
| Today / comparison | `B(42,10)` | `B(42,10)` Online 449; no receipts |
| 7 days / current | `B(42,10)`, `B(5,8)`, `B(81,7)`, `B(26,6)` | `B(5,8)` Online 251; `r-feb01` Cash 249; `B(81,7)` Cash 199; `B(42,10)` Online 449; `r-jan30` Online 151; `B(26,6)` Cash 349 |
| 7 days / comparison | `B(25,5)`, `B(19,4)` | `B(25,5)` Online 251; `B(19,4)` Cash 227; `r-jan25` Cash 51 |
| This month / comparison | `B(42,10)`, `B(26,6)`, `B(25,5)`, `B(19,4)`, `B(83,3)`, `B(31,2)` | Bill-paid rows above plus `B(83,3)` Cash 159 and `B(31,2)` Online 149; receipts `r-jan30` Online 151 and `r-jan25` Cash 51 |

Boundary controls: Cash Book read back Cash 111 on 2023-12-31 outside the prior month and Online 149 on 2024-01-01 inside it. Cash 159 on Jan 18 was outside the prior week; Cash 227 on Jan 19 and Cash 51 / Online 251 on Jan 25 were inside. Cash 349 on Jan 26 started the current week. The Jan 31 bill was posted after the Feb 1 activity, but its Online 449 stayed on Jan 31; the Feb 2 Cash 811 was outside all six windows, even though it was in February. The Feb 1 detail order uses reverse document date, then reverse fixture posting order as a stand-in for the legacy `created` timestamp. No browser timestamps were exercised.

The reused `receipt-target` identity had no bill and closed at 549 paise from its 1,000-paise opening debit. Jan 30 alone showed 0 Sales and 151 Collected from `r-jan30`, all Online. The app preview exposed cancelled `B(92,9)` on Feb 1 with 1,201 total and 701 paid; neither amount leaked into active Sales or Collected. Cash Book rows matched the ledger and bill facts one for one, and reporting left the fixture unchanged.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l02_02_l02_03_dashboard_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-DASH-c7db74d-focused](../phase2-local/target/L02-DASH-c7db74d-focused.log): 1 passed, 0 failed, 0 ignored, 280 filtered out.
- [L02-DASH-c7db74d-offline](../phase2-local/target/L02-DASH-c7db74d-offline.log): 421 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML was read as text only and remained unserved under OPEN-008. Browser parity, hosted authorization, and accounting acceptance remain open.

#### Owner-gated local Dashboard API

**L02-02 / L02-03: PASS (local Rust development)** at `2c4510de39ca3b1c4afdb1838813eacb6bbb9af8`. The [Dashboard API](../phase2-local/src/confirmation/dashboard.rs) takes a reporting date and period and reads local sale, cancellation and receipt records directly. The [February 1 replay](../phase2-local/src/confirmation/tests/l02_dashboard_replay.rs) now calls this API and retains the independent window, amount and source-ID expectations documented above. The earlier `c7db74d` run remains test-only evidence.

| Period / side | Inclusive window | Sales / Collected (paise) | Cash / Online (paise) |
| --- | --- | ---: | ---: |
| Today / current | 2024-02-01 | 3,603 / 699 | 448 / 251 |
| Today / comparison | 2024-01-31 | 2,402 / 449 | 0 / 449 |
| 7 days / current | 2024-01-26 to 2024-02-01 | 7,206 / 1,648 | 797 / 851 |
| 7 days / comparison | 2024-01-19 to 2024-01-25 | 3,603 / 529 | 278 / 251 |
| This month / current | 2024-02-01 | 3,603 / 699 | 448 / 251 |
| This month / comparison | 2024-01-01 to 2024-01-31 | 9,608 / 1,786 | 786 / 1,000 |

Expected and observed values matched in all rows. Sales IDs, comparison audit inputs, and ordered current Collected rows matched the preceding replay's ID table, with current row dates also asserted. The API exposes only current drilldown rows, not comparison drilldowns. Receipt IDs are fixture-local append slots: `0` maps to `r-jan25`, `1` to `r-jan30`, and `2` to `r-feb01` in this replay. They are not persistent identifiers. The current windows exclude Feb 2, the cancelled bill contributes neither Sales nor Collected, and the Jan 30 receipt alone contributes 151 paise Online with zero Sales. Exact Cash and Online totals for both periods reconciled with independent Cash Book day-close readbacks. Manager returned `AccessDenied` with no report, and the fixture matched its snapshot after all reads.

Focused checks also covered a full leap February comparison, a checked lower date bound after the role gate, distinct identical signed receipts, and an active zero-paid bill omitted from Collected detail. This local scope returns `UnsupportedRecord` for unsupported legacy approvals, checkout or crate sales, or missing reporting facts rather than inferring amounts or modes.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib dashboard -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-DASH-API-2c4510d-focused](../phase2-local/target/L02-DASH-API-2c4510d-focused.log): 4 passed, 0 failed, 0 ignored, 280 filtered out.
- [L02-DASH-API-2c4510d-offline](../phase2-local/target/L02-DASH-API-2c4510d-offline.log): 424 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Display rounding, browser parity, hosted authorization, and accounting acceptance were not established.

#### Counter recent bills and debtor shortcut

**L02-01: PASS (local Rust development)** at `6fd9187b0d118091517503f3f4efa0be363ae353`. The [offline replay](../phase2-local/src/confirmation/tests/l02_counter_replay.rs) froze the reporting date at 2025-03-14 and posted eight synthetic bills through the Owner API, then cancelled one. Rust sale and preview readbacks supplied draft and invoice IDs, document dates, and cancellation state. Counter filtering, creation times, and navigation were modeled in the replay from the [frozen Counter source](../ALM_Business_Manager.html#L2319); no browser behavior was observed.

| Check | Expected and observed | Evidence type |
| --- | --- | --- |
| Billed today | 4 active bills by document date: `(51,1)`, `(84,3)`, `(63,5)`, `(72,7)` | Test-only count over Rust sale facts |
| Recent bills | 6 of 7 active bills, in descending modeled creation order: `(14,2)`, `(63,5)`, `(72,7)`, `(84,3)`, `(38,6)`, `(27,4)` | Test-only list over Rust sale facts |
| Recents controls | Today-dated `(51,1)` had no modeled creation time and was trimmed. Cancelled `(95,8)` had the newest modeled creation key (80) and was excluded. Backdated `(14,2)` had the newest active key (75) and led the list without counting as billed today. | Modeled creation order, not invoice order or wall-clock observations |
| Open first recent row | Modeled selection of draft `14` called the existing Owner `bill_preview`, which returned invoice `2`, dated 2025-03-12, not cancelled. Fixture unchanged; no Print, Save PDF, or WhatsApp action invoked in the local replay. | Modeled navigation; actual Rust preview readback |
| Debtor shortcut | The modeled `customers/__due` route showed 1 ID: `counter-due`. Rust ledger balances in paise: `counter-due` +1,251, `counter-zero` 0, `counter-credit` −850, `counter-sub-rupee` +49. Source-derived rounding before the positive filter produced 13, 0, −8, and 0 rupees, respectively. | Actual exact ledger readbacks; modeled rounding, route, and filter |

Bill pairs are `(draft ID, invoice number)`. Manager requests for posted invoice preview and confirmation of draft `96` both returned `AccessDenied` without mutation. The existing Manager CLI control saved and reopened provisional draft `1` in separate launches, rejected the posting command, observed no committed writes during reopen/review, and found 0 pending Owner confirmations. No access was broadened.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l02_counter_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test draft_demo -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-COUNTER-6fd9187-focused](../phase2-local/target/L02-COUNTER-6fd9187-focused.log): 1 passed, 0 failed, 0 ignored, 284 filtered out.
- [L02-COUNTER-6fd9187-manager](../phase2-local/target/L02-COUNTER-6fd9187-manager.log): 2 passed, 0 failed, 0 ignored.
- [L02-COUNTER-6fd9187-offline](../phase2-local/target/L02-COUNTER-6fd9187-offline.log): 425 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and the locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser behavior, including output dispatch from an actual Counter click, and hosted authorization remain unverified.

#### Owner-gated local Counter API

**L02-01: PASS (local Rust development)** at `8b76011defe3111c5e964ddf5d825bf5532965ba`. The [Counter API](../phase2-local/src/confirmation/counter.rs) reads the posted fixture directly. The [replay](../phase2-local/src/confirmation/tests/l02_counter_replay.rs) reused the literal expectations from `6fd9187`, which remains test-only evidence. Synthetic creation keys were supplied when posting and stored on first success; they are not wall-clock readings or persistent metadata across restart. No prefiltered rows were passed to the API.

| Check | Expected and observed |
| --- | --- |
| Billed today, 2025-03-14 | 4 active bills by document date: `(51,1)`, `(84,3)`, `(63,5)`, `(72,7)` |
| Recent bills | 6 of 7 active bills, by stored key descending: `(14,2)`, `(63,5)`, `(72,7)`, `(84,3)`, `(38,6)`, `(27,4)`. Keyless `(51,1)` was trimmed; cancelled `(95,8)` was excluded despite key 80. |
| Recent-row preview | Modeled selection used the API's first returned draft `14`. The existing Rust preview returned invoice `2`, dated 2025-03-12, not cancelled. The fixture was unchanged; no print, PDF, or share request was invoked. |
| Debtor IDs | Only `counter-due` qualified. Independent ledger balances and rounded rupees: `counter-due` +1,251 paise → 13; `counter-zero` 0 → 0; `counter-credit` −850 → −8; `counter-sub-rupee` +49 → 0. |
| Manager | Counter returned `AccessDenied` with no payload. Posted preview and confirmation of draft `96` were also denied without mutation. The existing CLI saved and reopened a provisional draft and found 0 pending Owner confirmations. |

Bill pairs are `(draft ID, invoice number)`. The acknowledged retry returned the original acknowledgment without replacing the stored key. After retry and cancellation/undo, recent draft IDs were `[3, 8, 7, 2]`: equal keys retained newest-first posting order, with explicit zero and missing keys tied. An undated active fixture row was eligible for recents but did not count as billed today. The legacy edit-time fallback that assigns a timestamp to a missing or zero creation value was not implemented or verified in this reporting slice.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l02_counter_replay -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test draft_demo -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-COUNTER-API-8b76011-focused](../phase2-local/target/L02-COUNTER-API-8b76011-focused.log): 3 passed, 0 failed, 0 ignored, 284 filtered out.
- [L02-COUNTER-API-8b76011-manager](../phase2-local/target/L02-COUNTER-API-8b76011-manager.log): 2 passed, 0 failed, 0 ignored.
- [L02-COUNTER-API-8b76011-offline](../phase2-local/target/L02-COUNTER-API-8b76011-offline.log): 427 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and the locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser navigation and hosted authorization remain unverified.

#### Counter creation keys after Owner bill edit

**L02-01 / L04-01: PASS (local Rust development)** at `f971b9977167776ea19ae7b1619be90021c4bfd5`. The [edit replay](../phase2-local/src/confirmation/tests/edit/creation_key.rs) called the controlled fixture entry point `save_edit_with_creation_key` directly; it did not exercise normal `BillEntryState::save_edit`. The supplied key stood in for `Date.now()` in the [frozen Save rule](../ALM_Business_Manager.html#L4116). It read the stored key and called the Counter API after each case. At reporting date 2024-02-29, the initial Recent bills order was `[4, 3, 5, 2, 1]` by invoice number. All expected keys and orders below matched the readbacks.

| Case | Invoice | Key before → after | Recent invoice order |
| --- | ---: | --- | --- |
| Opened and abandoned | 1 | Missing → missing | `[4, 3, 5, 2, 1]` |
| Manager save denied | 1 | Missing → missing | `[4, 3, 5, 2, 1]` |
| Owner save at controlled key 20 | 1 | Missing → 20 | `[4, 1, 3, 5, 2]` |
| Stale edit save at key 99 | 1 | 20 → 20 | `[4, 1, 3, 5, 2]` |
| Owner save at controlled key 20 | 2 | 0 → 20 | `[4, 2, 1, 3, 5]` |
| Owner save at controlled key 99 | 3 | 10 → 10 | `[4, 2, 1, 3, 5]` |
| Untouched control | 5 | Missing → missing | `[4, 2, 1, 3, 5]` |

In every row above, **Billed today** stayed `[1, 3, 4, 5]` (4 bills); invoice 2 stayed dated 2024-02-28, the others stayed dated 2024-02-29, and no invoice identity changed. Abandonment and Manager denial left the entire fixture unchanged. On fresh fixtures, a stock-shortage save left invoice 1 keyless with the initial order and no mutation, as did the no-time save for the key and order only. A deliberately missing first-post slot returned `InvalidDraft` without mutation. The separate acknowledged confirmation retry returned the original acknowledgment despite a different supplied key; after its cancellation/undo control, the recent draft order remained `[3, 8, 7, 2]`. That retry did not exercise bill edit.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::edit -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib counter_keeps_first_post_key_on_retry -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-EDIT-KEY-f971b99-focused](../phase2-local/target/L02-EDIT-KEY-f971b99-focused.log): 10 passed, 0 failed, 0 ignored, 281 filtered out.
- [L02-EDIT-KEY-f971b99-retry](../phase2-local/target/L02-EDIT-KEY-f971b99-retry.log): 1 passed, 0 failed, 0 ignored, 290 filtered out.
- [L02-EDIT-KEY-f971b99-offline](../phase2-local/target/L02-EDIT-KEY-f971b99-offline.log): 431 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and the locked offline build passed; Rust editor diagnostics timed out.

This is in-memory Rust evidence, not durable timestamp or browser parity evidence. The frozen HTML remained unserved under OPEN-008.

#### Creation keys on normal Owner bill Save

**L02-01 / L04-01: PASS (local Rust development)** at `cbc6cdb91f41a57d84e3d88189fea5ba5a3bd180`. The [normal-path replay](../phase2-local/src/confirmation/tests/edit/normal_creation_key.rs) called `BillEntryState::open_edit` then `BillEntryState::save_edit`, overriding only the clock. Default Save supplies Unix epoch milliseconds to the same transaction. The fixture consults the clock only for a missing or zero stored key, after validation and before mutation.

| Normal Save at controlled key 20 | Stored key before → after | Returned / refreshed preview invoice | Recent invoice order |
| --- | --- | --- | --- |
| Invoice 1 | Missing → 20 | 1 / 1 | `[4, 1, 3, 5, 2]` |
| Invoice 2 | 0 → 20 | 2 / 2 | `[4, 2, 1, 3, 5]` |
| Invoice 3 | 10 → 10 | 3 / 3 | `[4, 2, 1, 3, 5]` |

Expected and observed results matched. On the fixed reporting date 2024-02-29, Billed today stayed `[1, 3, 4, 5]` (4 bills). In the rejection replay, Manager denial, abandonment, stock shortage, and a failing clock each left the fixture unchanged and Recents at `[4, 3, 5, 2, 1]`; no preview was published. A nonzero key bypassed the failing clock and Save succeeded. The separate acknowledged confirmation retry control passed without exercising edit Save.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib confirmation::tests::edit -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib counter_keeps_first_post_key_on_retry -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L04-EDIT-SAVE-cbc6cdb-focused](../phase2-local/target/L04-EDIT-SAVE-cbc6cdb-focused.log): 13 passed, 0 failed, 0 ignored, 281 filtered out.
- [L04-EDIT-SAVE-cbc6cdb-retry](../phase2-local/target/L04-EDIT-SAVE-cbc6cdb-retry.log): 1 passed, 0 failed, 0 ignored, 293 filtered out.
- [L04-EDIT-SAVE-cbc6cdb-offline](../phase2-local/target/L04-EDIT-SAVE-cbc6cdb-offline.log): 434 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The evidence covers local in-memory Rust behavior, not durable timestamps or browser parity. The frozen HTML remained unserved under OPEN-008.
