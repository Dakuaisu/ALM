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
