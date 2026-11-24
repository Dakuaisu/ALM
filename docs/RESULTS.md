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

#### Owner Counter attention counts

**L02-01: PASS (local Rust development)** at `200f5821afe73658959f7e0ff96d34c6c427062f`. The [purchasing replay](../phase2-local/src/confirmation/counter/attention_tests.rs) called the existing read-only Owner Counter on 2025-03-14. Its attention summary contains counts only. It uses the catalog's effective reorder levels, the purchasing status classifier, and the existing positive rounded-balance rule. Contributing IDs below were checked separately through stock, PO, and ledger readbacks.

| Checkpoint | Expected counts (low stock, open POs, debtors) | Observed counts | Contributing IDs and readbacks |
| --- | --- | --- | --- |
| Before receipt | `(1, 1, 2)` | `(1, 1, 2)` | Product `41`: 5 on hand at reorder level 5. `PO-001`: Pending, 0 received / 3 remaining. Debtors: `due-a`, `due-b`. |
| After partial receipt | `(0, 1, 2)` | `(0, 1, 2)` | Product `41`: 6 on hand, above level 5. `PO-001`: Partially Received, 1 received / 2 remaining; still open. Debtors: `due-a`, `due-b`. |
| After completion | `(0, 0, 2)` | `(0, 0, 2)` | Product `41`: 8 on hand, above level 5. `PO-001`: Completed, 3 received / 0 remaining; no longer open. Debtors: `due-a`, `due-b`. |
| All-zero control | `(0, 0, 0)` | `(0, 0, 0)` | Product `41`: 6 on hand at level 5; no contributing product, PO, or debtor IDs. |

The ledger controls were `due-a` at 50 paise and `due-b` at 151 paise (both qualify after rounding), and `under-half` at 49 paise (excluded). Manager received `AccessDenied` with no payload on the populated fixture. The fixture was unchanged after each read.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib attention_purchasing_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L02-ATTENTION-200f582-focused](../phase2-local/target/L02-ATTENTION-200f582-focused.log): 1 passed, 0 failed, 0 ignored, 294 filtered out.
- [L02-ATTENTION-200f582-offline](../phase2-local/target/L02-ATTENTION-200f582-offline.log): 435 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

This is local API evidence, not evidence of flag rendering, the “nothing needs attention” text, or browser navigation. The frozen HTML remained unserved under OPEN-008.

#### Owner Reports API: six-month P&L and all-time customers

**L16-01: PASS (local Rust development)** at `4a283d63950054355bf950c9c22979f1098ad9f8`. The [replay](../phase2-local/src/confirmation/tests/l16_monthly_reports_replay.rs) calls the read-only [Reports API](../phase2-local/src/confirmation/reporting/monthly.rs) with reporting date 2025-04-15 and the saved manual Cash Book, not filtered sales or computed totals. Expected and observed matched the literal `8f8f109` test-only expectations. The API supplies exact figures and contributing sales and Cash Book IDs; fixture receipt IDs and rupee formatting are replay-only evidence. Bill IDs below are draft IDs; receipt IDs are one-based fixture append positions.

| Month | Active bills | Saved Cash Book IDs | Receipts excluded | Sales / frozen cost / Gross Profit / Goods / Misc / Salary / Net (paise) |
| --- | --- | --- | --- | --- |
| 2024-11 | None | None | None | 0 / 0 / 0 / 0 / 0 / 0 / 0 |
| 2024-12 | None | None | None | 0 / 0 / 0 / 0 / 0 / 0 / 0 |
| 2025-01 | None | None | 8 | 0 / 0 / 0 / 0 / 0 / 0 / 0 |
| 2025-02 | 12 | 0 | 3 | 1,049 / 600 / 449 / 1,201 / 2,302 / 3,403 / −5,256 |
| 2025-03 | 36, 41, 58, 73 | None | 1, 2, 4, 5, 6, 7 | 6,145 / 3,700 / 2,445 / 0 / 0 / 0 / 2,445 |
| 2025-04 | 80 | 1 | None | 15,025 / 5,000 / 10,025 / 999 / 25 / 25 / 9,975 |

Receipt-only January stayed at zero, backdated bill 12 went to its February document month, and cancelled bill 24 did not contribute. April includes bill 80 from later in the reporting month. Net was **9,975 paise** before display rounding: `10,025 − 25 − 25`. Goods of 999 paise is separate, not deducted again. The replay formatter displays Net as ₹100; no browser formatting was observed.

| All-time customer group | Contributing draft IDs (account) | Exact total (paise) |
| --- | --- | ---: |
| Synthetic Grove | 36, 41, 73 (`grove`); 80 (`same-name`) | 19,671 |
| Synthetic Ancestor | 81 (`ancestor`, October control outside the table) | 15,025 |
| Synthetic Guest | 58 (unlinked) | 1,499 |
| Synthetic Orchard | 12 (`orchard`) | 1,049 |

The same-name group did not merge accounts: `grove` and `same-name` retained ledger/due balances of 3,849 and 14,925 paise respectively. Manager received `AccessDenied` without a payload from either read, and the sales fixture and saved Cash Book rows were unchanged. The local P&L rejects an unknown frozen cost explicitly rather than inferring one from live prices; customer ranking remains independently available.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l16_01 -- --test-threads=1 --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L16-REPORTS-API-4a283d6-focused](../phase2-local/target/L16-REPORTS-API-4a283d6-focused.log): 5 passed, 0 failed, 0 ignored, 294 filtered out.
- [L16-REPORTS-API-4a283d6-offline](../phase2-local/target/L16-REPORTS-API-4a283d6-offline.log): 439 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser parity and accounting acceptance remain open.

#### Owner Reports API: Dead Stock

**L16-02 through L16-05: PASS for the representable local Rust cases** at `d48980e61e1f0a503d2d081bdd4e79a33aebd8e2`. The [read-only API](../phase2-local/src/confirmation/reporting/dead_stock.rs) accepts a resolved calendar cutoff and derives the report from current catalog stock and active posted sales. The [application replay](../phase2-local/src/confirmation/reporting/dead_stock_tests.rs) uses independent expectations from the `5f3b55b` and `c6ede1f` model runs. Quantities below are whole packets; raw values are paise, before display rounding.

| Check | Expected and observed application result |
| --- | --- |
| Strict cutoff `2024-10-04` | Ordered product IDs `[4, 5, 8, 1]`: `4`, `5`, and `8` each had 4 packets and raw value 400; `1` had 3 packets and raw value 300, last sold `2024-10-03`. Product `2` was excluded at equality despite a later-posted backdated bill. Products `6` and `7` had zero stock and were excluded. |
| Cutoff `2024-10-05` | Ordered IDs `[4, 5, 8, 1, 2]`. Product `2` entered with 2 packets, raw value 200 and last sale `2024-10-04`. Products `3` and `9`, last sold October 5 and 6, stayed out. |
| Cancel and undo, cutoff `2024-10-04` | Product `1`, buy rate 400: with old bill `1` active and recent bill `2` cancelled, 4 packets, raw value 1,600 and last sale October 3. Undo restored recent activity and suppressed the row even with 2 packets still on hand (raw candidate 800). In the cancelled-only control, product `1` had 5 packets, raw value 2,000 and no last sale. |
| Retained direct and crate identities | With cutoff `2024-10-04`, each separate October 5 sale followed stored product ID `1` through renaming and suppressed it with 2 packets still on hand. In both runs only product `2` appeared, with 3 packets and raw value 300. |
| Ranking and cap | From 16 eligible products at cutoff `2024-10-04`, returned 15: IDs `1`–`12` (1 packet and 2,000 paise each), then `14` (1 / 1,075), `13` (2 / 850), `15` (1 / 0). Reversing catalog insertion reversed the first tie group and placed `16` (1 / 0) in the final slot instead of `15`. This is a distinct whole-packet fixture, not the rounded fractional model. |
| Current rate, not FIFO | Product `1`: conflicting catalog and purchasing rates of 950 and 720 used catalog 950, yielding 1,900 for 2 packets. In the price-edit control, 2 × 845 yielded 1,690 versus FIFO 200. In the goods-receipt control, 3 × 1,235 yielded 3,705 versus FIFO 1,435. |
| Empty result and permissions | An Owner got no rows with zero stock on product `1` and a cutoff-date sale on product `2` (1 packet remaining). Manager got `AccessDenied` without a payload. The read-only checks preserved fixture state, including the checked-value overflow control. |

**Model-only / not run through the application:** fractional packets and sub-paise values, negative stock, undated posting, and unresolved or name-only historical sale lines. The API tests supplied October 4 and October 5 as already-resolved `BillDate` values; they did not exercise the model's local clock, time-zone or DST conversion. Product `8` was never sold in the API cutoff fixture, not posted with a missing date. The historical helper tests were rerun separately and do not count as API passes.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib dead_stock_tests -- --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l16_02_replay --test l16_dead_stock_replay
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L16-DEAD-API-d48980e-focused](../phase2-local/target/L16-DEAD-API-d48980e-focused.log): 9 passed, 0 failed, 0 ignored, 299 filtered out.
- [L16-DEAD-API-d48980e-model](../phase2-local/target/L16-DEAD-API-d48980e-model.log): 4 passed, 0 failed, 0 ignored (historical helpers only).
- [L16-DEAD-API-d48980e-offline](../phase2-local/target/L16-DEAD-API-d48980e-offline.log): 448 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser parity and accounting acceptance remain open.

#### Owner Sales history and drilldown API

**L04-03 / L04-04: PASS (local Rust development)** at `4ee9a1731f524c121a4b4b2d47d493a17102a2e6`. The [replay](../phase2-local/src/confirmation/tests/l04_sales_history_replay.rs) passes the reporting date `2025-03-15`, period, and search text to the [read-only Owner API](../phase2-local/src/confirmation/sales_history.rs). The literal expectations at `1c1cf9a` are unchanged and remain separate test-only evidence. Bill IDs below are draft IDs; receipt IDs are one-based fixture append positions. All exact amounts are paise.

| Filter | Ordered Sales rows | Active Profit and Collected bills | Matched receipts | Expected and observed Sales / bill paid / receipt paid / Collected / Est. Profit |
| --- | --- | --- | --- | --- |
| Today | 58, 24, 36 | 58, 36 | 4 | 2,548 / 149 / 151 / 300 / 998 |
| This Month | 58, 24, 36, 73, 41 | 58, 36, 73, 41 | 1, 2, 4, 5, 6, 7 | 6,145 / 447 / 499 / 946 / 2,445 |
| Custom March 1–15 | 58, 24, 36, 73 | 58, 36, 73 | 1, 2, 4, 6, 7 | 4,047 / 298 / 450 / 748 / 1,547 |
| Custom March 1–15, item `LiMe` | 58, 24, 73 | 58, 73 | None | 2,998 / 149 / 0 / 149 / 1,098 |
| March 10 only | None | None | 1, 2, 6 | 0 / 0 / 249 / 249 / 0 |

Unlike Dashboard's month-to-date window, Sales history's This Month has no upper bound: it included bill `41` dated March 25, while the inclusive March 1–15 range excluded it. Matched cancelled bill `24` stayed visible but did not enter active totals or drilldowns. A separate stored-key control produced order `[36, 41, 73, 58, 24, 12]`, independent of invoice or document-date order, with equal keys in newest-post-first order and missing keys treated as zero.

For March 1–15, the API returned Collected detail in this order: receipt `4` (151), bill `36` (149), receipts `6` (51), `2` (49), `1` (149), `7` (50), then bill `73` (149). Exact Cash / Online / Collected were `397 / 351 / 748`. Est. Profit reconciled `4,047` line revenue less `2,500` frozen cost to `1,547`. The replay's whole-rupee formatting is test-only. Manager received `AccessDenied` with no payload, and reads left the fixture unchanged.

The local projection handles dated ordinary bills; in-range checkout or crate records return `UnsupportedSalesRecord`. Unknown frozen cost makes only Est. Profit unavailable. Standalone receipts have no stored name snapshot or creation timestamp in the fixture: search resolves names from bills linked by customer ID, and detail ties use the local posting anchor and receipt slot, not browser timestamps.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l04_sales_history_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L04-HISTORY-API-4ee9a17-focused](../phase2-local/target/L04-HISTORY-API-4ee9a17-focused.log): 9 passed, 0 failed, 0 ignored, 307 filtered out.
- [L04-HISTORY-API-4ee9a17-offline](../phase2-local/target/L04-HISTORY-API-4ee9a17-offline.log): 456 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. Browser parity and accounting acceptance remain open.

#### Saved receipt names and Sales detail order

**L14-01 / L04-04: PASS (local Rust development)** at `e9c852fb601eda1416316f06641d7678ff8693ec`. The [receipt-only replay](../phase2-local/tests/receipt_metadata.rs) used the Owner receipt-save path to store the selected label `Shared Name` and a controlled creation key, while the ledger continued to use customer ID. Both registered accounts had no bills. The fixture receipt IDs below are one-based append positions, not durable identifiers.

| Document date | Receipt slot | Customer ID | Stored creation key | Mode | Exact paise |
| --- | ---: | --- | ---: | --- | ---: |
| 2024-03-30 | 1 | `first` | 20 | Cash | 375 |
| 2024-03-30 | 2 | `second` | 10 | Online | 800 |
| 2024-03-30 | 3 | `second` | 30 | Cash | 125 |

The party search `shared` matched slots `[1, 2, 3]`; Collected detail returned them in key order `[3, 1, 2]`, not append order. The item-only search `LiMe` matched no receipts and returned zero Collected. Sales and bill-paid amounts stayed at zero. Sales and Cash Book agreed exactly: 500 Cash and 800 Online, for 1,300 paise Collected. Ledger closing balances were −375 paise for `first` and −925 paise for `second`. Manager Save and Sales read both returned `AccessDenied`; the Sales read had no payload, and the fixture equality control verified no mutation.

The [mixed-order regression](../phase2-local/src/confirmation/tests/l04_sales_history_replay/controls.rs) verified saved-name precedence over bill names and stable input order for equal nonzero creation keys. Older records retain absent metadata and their prior bill-name lookup and synthetic anchor/slot order; no names or timestamps were backfilled. The earlier `4ee9a17` limitation remains historical evidence. This change only uses creation keys for Sales detail, not ledger or as-of-preview chronology.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test receipt_metadata -- --nocapture
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l04_sales_history_replay
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L04-RECEIPT-META-e9c852f-focused](../phase2-local/target/L04-RECEIPT-META-e9c852f-focused.log): 1 passed, 0 failed, 0 ignored.
- [L04-RECEIPT-META-e9c852f-l04](../phase2-local/target/L04-RECEIPT-META-e9c852f-l04.log): 10 passed, 0 failed, 0 ignored, 307 filtered out.
- [L04-RECEIPT-META-e9c852f-offline](../phase2-local/target/L04-RECEIPT-META-e9c852f-offline.log): 458 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

This is local in-memory development evidence. The frozen HTML remained unserved under OPEN-008.

#### Normal Receive Payment Save

**L14-03 / L04-04: PASS (local Rust development)** at `673e0bb357104c3230f63d330597d9f62de8fae2`. The [replay](../phase2-local/tests/l14_receive_payment_replay/receipt.rs) used modeled keyboard and pointer events, then called the library's normal [`ReceivePayment::save`](../phase2-local/src/receive_payment.rs). The replay supplied a per-instance clock, not receipt metadata. Save took the ID and name from the same selected record and captured the creation key at Save. Both registered accounts had no bill in the receipt-only replay.

| Receipt date | Fixture slot | Selection | Customer ID | Creation key | Mode | Paise |
| --- | ---: | --- | --- | ---: | --- | ---: |
| 2025-03-17 | 1 | Historical control | `twin-east` | Absent | Cash | 90 |
| 2025-03-17 | 2 | Keyboard | `twin-east` | 1700000000020 | Cash | 375 |
| 2025-03-17 | 3 | Pointer | `twin-west` | 1700000000010 | Online | 800 |
| 2025-03-17 | 4 | Pointer | `twin-west` | 1700000000030 | Cash | 125 |

Each Save credited only the chosen ID. The `tWiN` party search found the three new `Twin Market` receipts without a prior bill and returned detail order `[4, 2, 3]`, not append order. No Sales rows appeared. Searched collections were 1,300 paise; the unfiltered total was 1,390, matching Cash Book (590 Cash and 800 Online). Closing balances were −465 paise for `twin-east` and −925 for `twin-west`. Slot 1's ledger entry remained unchanged and the receipt appeared only in the unfiltered read; it was not backfilled. Slot IDs are fixture append positions, not durable identifiers.

The selection replay also verified unresolved wrong-case, ambiguous duplicate, and cleared stale picks, plus a valid Manager attempt. None sampled the clock or added a receipt. Zero amount was rejected before clock sampling; a clock error left the ledger and Sales readback unchanged. A separate smoke check bounded the default Save's Unix-millisecond key by clock readings around Save. Picker events and suggestion ordering are modeled Rust behavior, not observed browser events or collation. Equal-key Sales ordering was not changed or verified against the browser in this slice.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_receive_payment_replay --test receipt_metadata -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-SAVE-673e0bb-focused](../phase2-local/target/L14-SAVE-673e0bb-focused.log): 7 passed, 0 failed, 0 ignored (6 normal-workflow replay tests and 1 direct metadata regression).
- [L14-SAVE-673e0bb-offline](../phase2-local/target/L14-SAVE-673e0bb-offline.log): 461 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; Rust editor diagnostics timed out.

This is local in-memory development evidence. The frozen HTML remained unserved under OPEN-008.

#### Saved creation keys in the ledger and estimate

**L14-01 / L05-01: PASS (local Rust development)** at `0dab1ebe62a1604ff0aadf6f5debe6fe7709f449`. The [keyed replay](../phase2-local/tests/l14_opening_balance_replay/ledger_keyed.rs) posted drafts `10`, `20`, `30` in that order, then saved receipts with keys `40`, `30`, and `20` for `keyed-a`, a separate receipt for same-name `keyed-b`, and the metadata-free receipt for `keyed-a`. It asserts the ledger and the estimates independently against the [frozen ledger](../ALM_Business_Manager.html#L7367) and [estimate cutoff](../ALM_Business_Manager.html#L4322) rules. Observed ledger rows for `keyed-a` matched the following expected order. Amounts are exact paise.

| Document date | Ledger row | Saved key | Debit | Credit | Running balance |
| --- | --- | --- | ---: | ---: | ---: |
| 2025-02-09 | Online receipt | 20 | 0 | 1,001 | −1,001 |
| 2025-02-10 | Opening | Not applicable | 2,000 | 0 | 999 |
| 2025-02-10 | Draft 30, invoice 3 | Absent | 10,000 | 0 | 10,999 |
| 2025-02-10 | Cash receipt | Absent | 0 | 444 | 10,555 |
| 2025-02-10 | Draft 10, invoice 1 (selected) | 30 | 10,000 | 1,000 | 19,555 |
| 2025-02-10 | Cash receipt | 30 | 0 | 222 | 19,333 |
| 2025-02-10 | Online receipt | 40 | 0 | 333 | 19,000 |
| 2025-02-11 | Draft 20, invoice 2 | 10 | 10,000 | 2,000 | 27,000 |

Invoice 1's bill-date estimate included invoice 2 despite its later document date and posting position, and included the key-20 receipt saved after invoice 1. It excluded both the equal-key 222-paise receipt and the later-key 333-paise receipt. Prior balance was 18,555 paise (display date 2025-02-11); adding invoice 1's own 9,000-paise due gave **27,555 paise**. **Latest was 27,000 paise**, matching ledger closing and including both excluded receipts. The same-name account `keyed-b` closed separately at −777 paise. The estimate output was captured in memory only. Manager ledger, preview and receipt Save were denied, with the application readbacks unchanged afterward.

Missing metadata is not backfilled. For a keyed ledger the source comparison treats missing or zero transaction keys as one and preserves source tie order; for a keyed estimate missing peer keys compare as zero. A wholly metadata-free ledger and an unkeyed estimate target retain their prior posting-anchor fallback. The earlier source-model checks remain separate from this application replay.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_ledger_ordering_replay --test l14_receive_payment_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L14-CHRONOLOGY-0dab1eb-focused](../phase2-local/target/L14-CHRONOLOGY-0dab1eb-focused.log): 17 passed (11 ledger/estimate target, 6 Receive Payment target), 0 failed, 0 ignored.
- [L14-CHRONOLOGY-0dab1eb-offline](../phase2-local/target/L14-CHRONOLOGY-0dab1eb-offline.log): 464 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. This is local in-memory evidence, not browser parity or accounting acceptance.

#### Estimate prior-balance rounding correction

**L05-01 / L14-01: PASS (local Rust development)** at `edb0b0bbefafff650c3f34438eaf9763eaf71a7d`. The separate source-derived expectation is that [`custBalanceBefore`](../ALM_Business_Manager.html#L4322) rounds the qualifying prior balance to whole rupees, then [`invBalanceMode`](../ALM_Business_Manager.html#L4226) adds the selected bill total and subtracts its payment. The [keyed replay](../phase2-local/tests/l14_opening_balance_replay/ledger_keyed.rs) preserves the chronology fixture and corrects only the historical estimate projection. Amounts below are exact paise.

| Invoice 1 check | Original `0dab1eb` observation | Corrected expected and observed |
| --- | ---: | ---: |
| Exact qualifying prior before rounding | 18,555 | 18,555 |
| Prior used in the estimate | 18,555 | 18,600 |
| Selected bill remainder | 9,000 | 9,000 |
| Bill-date estimate | 27,555 | 27,600 |
| Running ledger at the selected bill row | 19,555 | 19,555 |
| Latest and closing ledger | 27,000 | 27,000 |

The strict creation-key cutoff still excludes the equal-key 222-paise receipt from the bill-date estimate and includes it in Latest. The same-name account still closes separately at −777 paise. Manager denials and unchanged readbacks remain covered. The inert estimate capture shows `₹186.00` prior and `₹276.00` due; no browser rendered it.

The [fractional-bill control](../phase2-local/tests/l14_opening_balance_replay/ledger_rounding.rs) initially failed because the prior remained 155 rather than 200 paise. The corrected run rounded 155 to 200 and added the exact 125-paise remainder, giving **325 paise**, not the 300 paise produced by rounding the final exact sum. Latest and ledger closing remained 280 paise. Signed half-rupee ties and checked overflow were also exercised.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_ledger_ordering_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L05-ROUND-edb0b0b-focused](../phase2-local/target/L05-ROUND-edb0b0b-focused.log): 14 passed, 0 failed, 0 ignored.
- [L05-ROUND-edb0b0b-offline](../phase2-local/target/L05-ROUND-edb0b0b-offline.log): 467 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. This verifies the Rust projection against a source-derived expectation, not observed browser parity or accounting acceptance.

#### Latest estimate aggregate rounding correction

**L05-01 / L14-01: PASS (local Rust development)** at `4b5d3e51ccd408969357df5c968ebd43974cea1b`. The source-derived expectation is that [`invBalanceMode`](../ALM_Business_Manager.html#L4226) uses [`custBalance`](../ALM_Business_Manager.html#L1158) for linked Latest and rounds the completed customer balance to whole rupees. The [Rust replay](../phase2-local/tests/l14_opening_balance_replay/ledger_rounding.rs) verifies the corrected projection independently of the ledger. The `edb0b0b` observation above remains unchanged.

| Fractional fixture | Original observed (paise) | Corrected expected and observed (paise) |
| --- | ---: | ---: |
| Bill-date estimate (rounded prior plus exact remainder) | 325 | 325 |
| Latest estimate | 280 | 300 |
| Exact ledger closing | 280 | 280 |

The signed controls used a 125-paise bill and a 50-paise standalone receipt. Exact ledger balances of +150 and −150 paise rounded in Latest preview to +200 and −100 paise respectively, matching the inert print and Save PDF captures (`₹2.00` and `₹-1.00`). The receipt credit stayed 50 paise. The existing chronology and walk-in fallback regressions passed; this change does not round transactions or change the historical cutoff.

```sh
cd /Users/apple/ALM && env TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test l14_ledger_ordering_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [L05-LATEST-4b5d3e5-focused](../phase2-local/target/L05-LATEST-4b5d3e5-focused.log): 15 passed, 0 failed, 0 ignored.
- [L05-LATEST-4b5d3e5-offline](../phase2-local/target/L05-LATEST-4b5d3e5-offline.log): 468 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. This is a source-derived expectation checked against local Rust output, not observed browser parity.

#### Recovered cancelled estimate Latest rounding

**L05-01 / L19-01: PASS (local Rust development)** at `250e083e69ffc416168aeae8f9e4a8ef336460c4`. The [signed recovery replay](../phase2-local/tests/milestone_d_structured/signed_recovery.rs) first reproduced the prior `InvalidArchive` rejection for a fractional balance. `Store::estimate_html` now uses the same checked rounding rule as the live estimate, applied once to the linked aggregate after summing the exact saved transactions. The [earlier recovery limitation](#milestone-d-same-store-estimate-recovery) remains unchanged as historical evidence.

| Exact linked balance (paise) | Source estimate Latest | Freshly restored and reopened estimate Latest |
| ---: | ---: | ---: |
| +150 (950 due − 800 receipt) | ₹2.00 | ₹2.00 |
| −150 (650 due − 800 receipt) | ₹-1.00 | ₹-1.00 |
| +50 (849 due − 799 receipt) | ₹1.00 | ₹1.00 |

The 849/799 control distinguishes rounding after summation from rounding each transaction. Each store was independently checked against the expected display and typed input records; the cancelled bill still displayed ₹410.00 total and ₹100.00 paid. The replay dropped the source handle, restored the saved backup to a fresh path, reopened it and regenerated from typed records without reseeding or loading cached HTML. Manager denial, invalid-link and overflow rejection, read-only disk and backup checks, and the whole-rupee ₹-50.00 recovery regression remained covered.

Retained artifacts: [positive source HTML](../phase2-local/target/structured-d-76885-1795327935313401000/positive-150-source.html), [positive regenerated HTML](../phase2-local/target/structured-d-76885-1795327935313401000/positive-150-restored.html), [positive backup](../phase2-local/target/structured-d-76885-1795327935313401000/positive-150-backup.json), [positive restored store](../phase2-local/target/structured-d-76885-1795327935313401000/positive-150-restored.json); [negative source HTML](../phase2-local/target/structured-d-76885-1795327935313401000/negative-150-source.html), [negative regenerated HTML](../phase2-local/target/structured-d-76885-1795327935313401000/negative-150-restored.html), [negative backup](../phase2-local/target/structured-d-76885-1795327935313401000/negative-150-backup.json), [negative restored store](../phase2-local/target/structured-d-76885-1795327935313401000/negative-150-restored.json). `cmp` confirmed byte equality for each HTML pair and each backup/restored JSON pair.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG ALM_STRUCTURED_RETAIN=1 TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_d_structured --test l14_ledger_ordering_replay -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --quiet
```

- [D-ESTIMATE-ROUND-250e083-focused](../phase2-local/target/D-ESTIMATE-ROUND-250e083-focused.log): 17 passed (2 structured target, 15 ledger/estimate target), 0 failed, 0 ignored.
- [D-ESTIMATE-ROUND-250e083-offline](../phase2-local/target/D-ESTIMATE-ROUND-250e083-offline.log): 468 passed, 0 failed, 2 ignored. Formatting, all-target Clippy with warnings denied, and locked offline build passed; editor diagnostics timed out.

The frozen HTML remained unserved under OPEN-008. These are local output checks, not observed browser parity or accounting acceptance.

## PC-003 replan checklist and Batch 1 billing reconciliation

**Handoff for one consolidated Supervisor review, not a completed review or acceptance.** Plan `82b15ba` ([PC-003 / DEC-026](DECISIONS.md#dec-026-pc-003-deliver-remaining-local-work-in-three-connected-workflow-batches)); tested `0347ccb`. All results below are local synthetic development. Batch 2 and Batch 3 are routed, not delivered by this handoff.

### D1: Deliver connected workflows by batch

- [x] The [replan](PRD.md#local-delivery-alongside-discovery) defines three dependency-based batches with reusable tests, integrated checks and one handoff and review per batch: billing/receipts/books; supplier order/stock/dispatch/valuation; documents/settings/export/recovery. Supplier-to-stock can start with synthetic opening stock; document recovery can use the existing fixtures. A blocked dependency doesn't stop independent work. Optional demo feedback isn't a gate.
- [x] The Batch 1 kickoff covers provisional drafts, explicit Owner simulation and retries, dated sales and receipts, estimates/GST, edit/cancel, history, ledger and Cash Book, with structured archive readback. The tests below reuse the prior controls rather than replace them. The CLI-to-Owner fixture is separate from the bill-to-books fixture; they do not prove one draft ID traveled through both. The same dated billing fixture supplies the cancelled sale and receipt to structured recovery.
- [x] Historical evidence and owner gates are preserved; the unit submitted for Supervisor evaluation is the whole batch. This finishes the owner's **replan checklist**, not the Supervisor review of Batch 1. That review remains pending.

#### Reproduce and count the saved runs

Run from the tested checkout with the existing `phase2-local/target/tmp` directory. These commands keep optional raster and artifact retention paths off.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_c_replay --test milestone_d_structured --test owner_simulation --test sqlite_restart -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target/tmp cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml
```

| Run ID at `0347ccb` | Actual Cargo summaries |
| --- | --- |
| [B1-0347ccb-focused](../phase2-local/target/B1-0347ccb-focused.log) | **11 passed, 0 failed, 1 ignored**: C 1, structured D 2 (recovery and the partial-PO setup), Owner simulation 1, SQLite restart 7 plus its ignored worker. |
| [B1-0347ccb-offline](../phase2-local/target/B1-0347ccb-offline.log) | **468 passed, 0 failed, 2 ignored**, summed across 41 result summaries, including the zero-test doc target. This includes the focused tests; don't add the totals together. |

The ignored entries are `sqlite_restart_worker` (both runs; dispatched by SQLite restart tests) and `confirmation::tests::restart::client_worker` (offline only; dispatched by confirmation restart tests). These are child-process entry points, not independent top-level passes. The [SQLite restart suite](../phase2-local/tests/sqlite_restart.rs) covers provisional draft persistence and reconnect without posting until explicit confirmation. The [Owner simulation](../phase2-local/tests/owner_simulation.rs) launches the real local `alm-draft-demo` subprocess to save the SQLite draft; the lost response and retry exercise the separate simulated authority, not a hosted call or a restarted posting server. The structured archive test drops and reopens store handles; it isn't a posting-authority restart. Formatting, all-target Clippy with warnings denied, and the locked offline build passed for the tested checkpoint; LSP diagnostics timed out, so there is no LSP pass.

#### Fixed controls and observed results

**L03/L14, provisional save and explicit retry.** Fixed control: the Manager can save and reopen but cannot post, and an explicit fresh Owner approval applies exactly once even if its first reply is lost. Observed: the SQLite draft reopened unchanged at the end as `DraftId(1)`, product `ProductId(1)` ×3, typed `Synthetic Customer`, no bill date or crate. Only the detached simulation copy was linked to the selected customer and dated `2025-02-21`. Unit price changed from 11,800 to 14,160 paise (35,400 to 42,480 total). Stale Owner review returned `StaleReview`; Manager confirm returned `AccessDenied`. Both left seven units/47,000 FIFO value, zero posted, zero sales/receipts/due and no profit record. Owner `RequestId(1)` applied with no reply: one posted invoice #1, four units/28,000 FIFO value, 42,480 sales, 20,000 received, 22,480 due and 23,480 gross profit. Identical retry returned that same receipt without another effect; reuse with 20,001 paid returned `RequestConflict` with readbacks unchanged.

**Later local follow-ups, separate from the `0347ccb` timeline.** [R56-SWITCH](#r-5r-6-development-synthetic-manager-account-switch-and-restart) checked account-scoped SQLite readbacks after switching and restarting under trusted synthetic selectors. Its separate in-memory Owner control showed no posting changes. [R56-CLI-EDIT](#r56-cli-edit-explicit-save-after-a-scoped-quantity-edit) checked that an edit stayed provisional and survived a fresh CLI launch only after explicit Save. [R56-PRICED-LINES](#r56-priced-lines-edit-numbers-in-the-priced-manager-preview) also verified that the displayed line number selected the intended line when the product appeared twice.

[B1-QTY-RECONNECT](#b1-qty-reconnect-persisted-quantity-change-invalidates-owner-review) saved the quantity edit through the scoped SQLite API, not through the CLI edit run. A fresh SQLite handle reopened the revised draft. Separately, the in-memory Owner simulation registered that draft without posting, rejected the retained review as `StaleReview` without posting effects, and posted only after fresh review and explicit Owner confirmation. Posting did not alter the SQLite draft. These runs do not form one CLI-to-posting execution and do not establish hosted authorization or posting. The later [B1-SAME-RECORD run](#b1-same-record-cli-saved-named-draft-through-simulated-owner-posting-and-books) used the CLI-saved draft as input to the Owner simulation in one local replay.

**L03-02 / L04-01 / L04-02 / L05-01 / L05-02 / L06-01 / L08-01 / L11-01 / L14-01 / L15-01 / L19-03, dated bill-to-books.** [Milestone C](../phase2-local/tests/milestone_c_replay.rs) asserts the fixed controls below; actual values matched them in the focused log. This distinct fixture uses `milestone-c-buyer` (`Synthetic Buyer`), `DraftId(21)`, invoice #1, document date `2025-02-21`. Amounts are exact paise unless shown with ₹. Cash Book cells give cash/online and entry count for the named day.

| Checkpoint | Fixed control | Observed (matched) |
| --- | --- | --- |
| Receipts and pre-posting | Backdated receipt keeps its own date/rate; no bill or cash effects before Owner confirm. | Entered `2025-02-20` first: product 1, 3 × 7,000; product 2, 2 × 2,500; `PartiallyReceived`. Entered product 1, 1 × 6,000 dated `2025-02-19` second: `Completed`. Stored receipt order stayed as entered; FIFO put Feb 19 first, with 4 and 2 units worth 32,000. Balance zero, ledger `InvalidDraft`, both days empty. Manager confirm `AccessDenied`, posted count zero. |
| Reopen and confirm the named draft | Manager saves draft 21 in SQLite. Reopen on a fresh handle; stale terms must not post. | The reopened draft supplied the Owner review: 9,440 → 11,800 per unit, 18,880 → 23,600 total. Old review rejected. After explicit confirmation the reply was discarded; identical retry returned the original receipt and changing paid to 10,001 under the same request returned `RequestConflict`. Neither retry changed the posted effects or Manager draft. |
| Owner post, Feb 21 | Two units at 11,800, 10,000 Cash on bill; deduct oldest lots, post on bill date only. | Bill debit 23,600, credit 10,000, stored due/current balance 13,600. FIFO 19,000 (2 × 7,000 plus 2 × 2,500). Feb 21 Cash Book 10,000/0 (1); Mar 1 0/0 (0). Sales CSV recorded product 1, qty 2, total 23,600, paid 10,000. Estimate AsOnBillDate/Latest both ₹136.00 due. |
| Separate Online receipt, Mar 1 | Apply 5,000 against live balance only; don't rewrite bill-paid, bill-date due, CSV, GST or stock. | Manager receipt `AccessDenied`, Owner readbacks unchanged. Owner's `2025-03-01` receipt credited 5,000 on its own ledger row; closing 8,600. Feb 21 stayed 10,000/0 (1); Mar 1 became 0/5,000 (1), displayed as ₹0/₹50. Stored bill total/paid/due/balance remained 23,600/10,000/13,600/13,600; estimate AsOnBillDate ₹136.00 vs Latest ₹86.00. FIFO stayed 19,000. |
| Edit same bill | Keep ID, number, date, original Cash payment and independent receipt; recalculate lines, stock and ledger. | Product 1: 2 × 17,700; product 2: 1 × 5,600. Total 41,000, paid 10,000, bill-date due 31,000, current/stored-at-edit balance 26,000. Estimate ₹310.00 AsOnBillDate vs ₹260.00 Latest. Both Cash Book days unchanged; FIFO 16,500 (2 × 7,000 plus 1 × 2,500). CSV put total/paid on the first line only. |
| Cancel same bill | Retain edited record/number and historical output, remove bill from active books and stock, retain standalone receipt. | Both CSV rows `CANCELLED`; saved total/paid/due/balance remained 41,000/10,000/31,000/26,000. Feb 21 Cash Book 0/0 (0); Mar 1 stayed 0/5,000 (1). Ledger contained only the Mar 1 Online credit, closing −5,000. FIFO restored to 32,000. Cancelled estimate retained ₹410.00 total and ₹100.00 paid; AsOnBillDate ₹310.00 vs Latest ₹-50.00. Undo not part of this joined timeline. |

The ordinary GST control at post and after receipt was HSN `22029920` at 18%: taxable 20,000, tax 3,600 (CGST/SGST 1,800 each, IGST 0), gross 23,600. The edit matched taxable 35,000 and tax 6,000 (CGST/SGST 3,000 each, IGST 0), gross 41,000: `22029920` 30,000 taxable/5,400 tax and `21069099` at 12%, 5,000 taxable/600 tax. Cancel retained those amounts and set the flag. This is ordinary-sale renderer/readback evidence, not a standalone GST lifecycle or operator action. Estimate routes matched `_print` `OnLoad` and `_pdf` `AfterLoad(350ms)` at each stage, not an actual print or saved PDF.

**L05-01 / L06-01 / L18-01 / L18-04 / L19-01 / L19-03, linked archive readback.** [Structured D](../phase2-local/tests/milestone_d_structured.rs) linked `sale-cancelled-id` (Feb 21, #1) and `receipt-standalone-id` (Mar 1, Online 5,000) to the buyer account; the active `sale-active-id` control (Jan 1, #99; 1,100 total, 300 paid) was linked elsewhere. Expected and asserted on source and freshly restored/reopened typed records: estimate #1 dated Feb 21, `CANCELLED`, ₹410.00/₹100.00/₹-50.00 (total/paid/Latest as of Mar 1); escaped FSSAI shown and GSTIN hidden. Receipt of 7,000 on an independent copy gave ₹-70.00; linking the active control to the buyer instead gave ₹-42.00. Reorder default 5 to 13 persisted. Backup and regenerated estimate, ordinary GST, PO documents, Sales CSV and SpreadsheetML matched originals and the fixture assertions; no cached rendering or reseed. Bad JSON, wrong version, invalid date and existing-target restore failed closed. Manager couldn't write the setting, back up or restore, or obtain restricted estimate/GST/PO documents; reopened CSV and workbook returned `AccessDenied`. The scratch outputs weren't retained by this run. This is read-model recovery only.

The [existing signed recovery test](../phase2-local/tests/milestone_d_structured/signed_recovery.rs) ran inside `structured_exports_survive_fresh_recovery`, not as another counted test. Its independent expected displays matched on source and restored data: +150 paise (950−800) → ₹2.00; −150 (650−800) → ₹-1.00; +50 (849−799) → ₹1.00. The last control distinguishes summing exact transactions before rounding from rounding each transaction; missing links and overflow fail closed. This reuses the `250e083` rule and tests; the preceding historical result and artifacts remain intact.

**Related PO fixture, not Batch 2 completion.** The focused setup asserted Feb 10 planned quantities/rates 12 × 100 and 9 × 180 (total 2,820), no stock, receipts or register rows on Save. Feb 12 receipts 5 × 130 and 4 × 210 left 7/5 outstanding and FIFO 1,490; the Feb 11 backdated receipts 3 × 110 and 1 × 190 left 4/4 and FIFO 2,010. Both stages were `PartiallyReceived`; this setup stops before Close Remaining and doesn't join crate dispatch or physical count.

**Sales drilldown and saved closes (same C timeline).** The bill collection was keyed by draft 21, the independent Online collection by fixture-local `SalesReceiptId(1)` (not a persistent receipt identifier). Exact-day queries separated the two. Combined collections went 10,000 → 15,000 → 15,000 → 5,000 paise. Owner copied system figures into separate dated manual closes before cancellation. After cancellation the saved Feb 21 close still held ₹100 Cash while live Cash was ₹0 (₹100 difference); both saved and live Mar 1 still held ₹50 Online (zero difference). Manager ledger, Sales history, live close, copy, save, saved-row read and comparison were denied. No manual close was rewritten to hide the difference.

#### Inventory routing and remaining actions

Use the [source inventory](PHASE1_DISCOVERY.md#source-inventory-and-parity-cases), [case matrix and source-order branches](PHASE1_CASES.md), and [existing evidence by case](MILESTONE_D_COVERAGE.md). Routing includes secondary controls, guard branches and helper-only variants; it isn't a pass. Preserve named historical results and offline test outcomes at their own scope. Other action variants below are not run in the joined billing timeline.

| Group / action | Route and disposition |
| --- | --- |
| L01 shell, startup, arbitration, recovery and upgrades | Batch 3 for synthetic restore/storage analogues; frozen startup, shortcuts, resources, mirror/IndexedDB and schema-upgrade walkthroughs wait for OPEN-008 (L01 first), deployment dependencies OPEN-001; real migration OPEN-002/003/007. |
| L02 Counter/Dashboard, L03 entry, L04 history | Batch 1: navigation, sales versus collections ranges/drilldowns and ageing; draft guards, resume/discard, walk-in/mixed-unit and numbering controls; history search/filter, edit isolation, cancel reason/undo and KPI branches. C covers only the joined checkpoints above. |
| L05 estimate, L06 GST | Batch 1 for saved-vs-live due and ordinary GST before/after edit/cancel. Batch 3 for remaining estimate output/routing, preference controls, ordinary renderer variants and standalone save/view/edit/cancel/delete, GSTIN/state, popup and pagination. Actual output and legacy operator paths wait for OPEN-005/008. |
| L07 PO actions, L08 receiving | Batch 2 for paste/save/validation, receipt progress, rate/quantity guards, close/decline, edit/delete, fallback/quote and receiving-form variants; Batch 3 for supplier-safe documents, print/share and recipient routing. Invalid calendar paste divergence L07-04 waits for OPEN-009; `BillDate` and both mismatch thresholds stay unchanged. |
| L09 catalog/suppliers, L10 crates, L11 stock/FIFO, L12 Godown, L13 reorder/planner | Batch 2: identity, quote and historical reference changes, templates/component dispatch, lot ordering/shortage/conversion/cutoffs, adjustment and fridge, manual Godown versus derived register, threshold/add/copy/checkout. Batch 3 for supplier contact intent and clipboard sink handling. Don't count the partial-PO setup as the connected stock reconciliation. |
| L14 customers/udhaar, L15 Cash Book, L16 reports | Batch 1: receipt/identity/opening/delete and manual close/copy/delete/expense branches; sales/P&L, collections and customer rankings. Batch 2: supplier buys, dead-stock cutoffs, cancellation, buy-rate valuation and identity. Batch 3 for reminder output intents. Joined C doesn't establish manual Cash Book or Reports parity. |
| L17 rate list, L18 settings, L19 backup/restore/export, L20 sync | Batch 3: selection/filter and outputs, business identity/preferences/privacy/modal/activity/clear, folder handle/permissions and save fan-out, export projections and restore checks, local-only cloud/Sheets guards, merges, cursors, snapshots, tombstones, scheduling, readback/conflicts and navigation stubs. Batch 1 uses only the needed sales projections and archive checks above. Real integrations wait for OPEN-002/006; browser/device actions wait for OPEN-005/008. |

### D2: Cadence only; permissions and evidence gates unchanged

- [x] `OPEN.md` and `CLAUDE.md` are unchanged. No model, effort or Bridge configuration change in PC-003. Local SHA-256 of `bridge.toml` is `e2a53f33d79e120f6676d6db11e8df95d6fb7d24407149bc4070d47850625b4b`, matching [DEC-025](DECISIONS.md#dec-025-engines-changed-by-the-owner).
- [x] Reversible local synthetic work only. Manager stays provisional: no invoice, payment, stock reservation/dispatch or posting on reconnect; deny restricted operations and data at the boundary, not just in UI. Owner posting here is explicit simulation, not approval of the production role/data or stale-term authority (OPEN-004).
- [ ] Legacy runtime **not run**: frozen HTML remains unserved under OPEN-008 until the positive control and OS-level deny evidence pass; baseline dependency capture OPEN-001 remains open. No inference of browser parity from renderer or sink results.
- [ ] Protected metadata/real records and hosted tests **blocked** pending separate OPEN-002 and OPEN-006 authorization. Source precedence, invoice and GST/accounting reconciliation remain OPEN-003; no hosted grants, cross-device cache/sync, account or concurrency acceptance from these subprocesses.
- [ ] Installed Windows/macOS/Android workflows, actual print/PDF/clipboard/WhatsApp and device parity **not run**, pending OPEN-005. Migration rehearsal, rollback, accounting/owner sign-off and production cutover remain separately gated by OPEN-002/003/007, including invoice continuity, preservation of post-switch sales, one writable path and distinct cutover approval. No formal phase exit is claimed.

## PC-003 Batch 2: Supplier order through stock, dispatch and valuation reconciliation

**One handoff for Supervisor review.** [PC-003 / DEC-026](DECISIONS.md#dec-026-pc-003-deliver-remaining-local-work-in-three-connected-workflow-batches), plan `82b15ba`; tested `657cc7f` (purchasing `cbbbce0`). Local synthetic development evidence only. The [joined replay](../phase2-local/tests/milestone_b_replay.rs) reuses the original shared [plan](../phase2-local/tests/common/partial_po/plan.rs) and [partial receipt](../phase2-local/tests/common/partial_po/receipts.rs) controls via [purchasing](../phase2-local/tests/milestone_b_replay/purchasing.rs).

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test milestone_b_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

| Saved run | Actual Cargo results |
| --- | --- |
| [B2-657cc7f-focused](../phase2-local/target/B2-657cc7f-focused.log) | **3 passed, 0 failed, 0 ignored**: joined reconciliation, Close Remaining control, L07-03 supplier output control. |
| [B2-657cc7f-offline](../phase2-local/target/B2-657cc7f-offline.log) | **469 passed, 0 failed, 2 ignored** across 41 summaries including the zero-test doc target; includes the focused tests. |

Ignored: `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`, both subprocess entry points dispatched by restart tests, not independent passes. Formatting, all-target Clippy with warnings denied, and locked offline build passed. LSP diagnostics timed out; no LSP pass.

Fixed assertions matched the replay below. A/B are Synthetic A/B (`ProductId(1/2)`); money is in minor units and stock is in packets unless noted. Dates are February 2025 unless specified.

| Inventory route / checkpoint | Fixed values and matched result |
| --- | --- |
| L09/L13/L07, supplier quote to Feb 10 PO Save | Winning supplier quotes A 200 → 100 and B 280 → 180 beat the other supplier's 160/260. Live plan A 12 × 100, B 9 × 180 totaled 2,820. `PO-001` saved Pending at stock 0/0 and FIFO 0, with no receipts or Feb 10 register rows. |
| L08/L11, backdated partial receipts and close | Feb 12 posted first: A 5 × 130, B 4 × 210; stock 5/4, outstanding 7/5, FIFO 1,490. Feb 11 posted second: A 3 × 110, B 1 × 190; stock 8/5, outstanding 4/4, FIFO 2,010 (980/1,030), Feb 11 layers first. Both were PartiallyReceived. Manager close was denied and Owner decline preserved progress; Owner Close Remaining completed the PO with `(received, cancelled, remaining)` A `(8,4,0)`, B `(5,4,0)` without changing receipts, movements, stock, FIFO or ledgers. |
| L10/L11/L12, Feb 13 crate dispatch | Two mixed crates expanded to A 4 and B 2 packets. One Owner post recorded six packets and 1,000 in both sales and received ledgers. Oldest-first depletion was 860; only Feb 12 layers remained, A 4 × 130 = 520 and B 3 × 210 = 630, stock 4/3 and FIFO 1,150. Register closings Feb 11/12/13/14 were 3/1, 8/5, 4/3, 4/3; receipts fell on their own dates and dispatch only on Feb 13. |
| L12, manual Godown | Owner saved and reopened A at 31 cases, edited and reopened the single row at 47 (`back rack`). Manager read/save denied. Billing, stock, FIFO, register and closed PO stayed unchanged; the later physical count also left the 47-case row intact. |
| L11/L12/L13, Feb 15 physical count and reorder | No reorder rows before dispatch; after dispatch both met their thresholds (A 4, suggest 8; B 3, suggest 6). Owner counted B 3 → 5, fridge 0, reason `CountCorrection`, note `two loose packets found`; A stayed 4 and only A remained on reorder. Feb 15 B register was opening 3, received 0, dispatched 0, adjustment +2, displayed received 2, closing 5; Feb 16 carried 5 forward without movement. B kept 630 known FIFO value plus **2 uncosted packets** dated Feb 15, so B total was unknown, not zero; A stayed 520. |
| L16, valuation and cutoffs | [Dead Stock](../phase2-local/tests/milestone_b_replay/count_adjustment.rs) uses last sale strictly before cutoff, not stock as of cutoff: Feb 12/13 empty, Feb 14 and 16 matched current stock at each replay stage. Feb 14 valuation went 1,010 → 1,390 (A 4 × 110 = 440, B 3 → 5 × 190 = 570 → 950), not FIFO. At the Jan 31 report anchor all six months had no bills or revenue/cost; February held the one bill at revenue 1,000, frozen cost 860 and gross 140, unchanged between Feb 14 and 16 report anchors. |

**Crate valuation scope.** The mixed-crate FIFO result above stands as local development evidence for that fixture. The separate [repeated-product L10-01 replay](#pc-003-batch-2-addendum-crate-fifo-and-dead-stock-valuation) matched FIFO depletion but recorded a report valuation parity **MISS** against the source-derived calculation, not a frozen-runtime observation. [OPEN-010](OPEN.md#open-010-repeated-product-crate-valuation-l10-01) remains owner-blocked; [DEC-027](DECISIONS.md#dec-027-defer-repeated-product-crate-valuation-l10-01) defers release costing and report rounding. Neither costing rule is approved for release.

**Lot identity and Manager boundary.** Stock Lots asserts dated active layer order, remaining quantity, unit cost and value; rows are not stable lot IDs, receipt links or original quantities. The local Manager draft check is separate from the inventory fixture: reopened draft 2, dated Feb 16, had A ×1/B ×2, unassigned customer and no crate. Its Rust debug payload was checked for absence of cost, profit, buy-rate, locked-value, supplier and money markers. This is not serialized client, device cache or hosted isolation evidence. The later [B2-MANAGER-SQLITE run](#b2-manager-sqlite-scoped-draft-readback-from-the-batch-2-inventory-replay) inspected the scoped SQLite store directly; the original check did not inspect its stored rows. Manager reads of the posted sale, Stock Lots, register, reorder, physical count, product entry, purchase cost, Dead Stock and monthly/total profit were denied, as were count submit and crate loading.

**Disposition.** [L07-03](../phase2-local/tests/milestone_b_replay/l07_03.rs) ran separately: Pending/partial/closed views and inert print/handoff controls preserved stock, FIFO and ledgers; Manager was denied before sinks. Synthetic routing bytes do not prove PNG rendering or delivery. No test failures in either saved run. L07-04 invalid calendar parity remains blocked on OPEN-009; protected data/hosted tests remain blocked on OPEN-002/006. Frozen browser walkthrough and isolation verification (OPEN-008), installed-device and actual output actions (OPEN-005) were not run. Batch 3 documents/settings/export/recovery was pending when this handoff was written; see the subsequent [Batch 3 development handoff](#pc-003-batch-3-documents-settings-export-and-recovery). Original instructions and gates remain unchanged; no acceptance or phase exit claimed.

## PC-003 Batch 3: Documents, settings, export and recovery

**One Supervisor handoff, local synthetic development only.** [PC-003 / DEC-026](DECISIONS.md#dec-026-pc-003-deliver-remaining-local-work-in-three-connected-workflow-batches), tested revision `eb3e5b7`.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target ALM_STRUCTURED_RETAIN=1 cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test milestone_d_structured --test milestone_d_replay --test preference_replay --test rate_list_replay --test folder_backup_replay --test clear_restore_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B3-eb3e5b7-focused](../phase2-local/target/B3-eb3e5b7-focused.log): **29 passed, 0 failed, 0 ignored** — structured D 2, replay D 9, preferences 1, rate list 4, folder backup 11, clear/restore 2.
- [B3-eb3e5b7-offline](../phase2-local/target/B3-eb3e5b7-offline.log): **469 passed, 0 failed, 2 ignored** across 41 result summaries including doctests; includes the focused tests. `confirmation::tests::restart::client_worker` and `sqlite_restart_worker` are dispatched subprocess entry points, not independent passes. Formatting, all-target Clippy with warnings denied, and locked offline build passed; LSP diagnostics timed out.

### Retained artifact manifest

The retained file links below are from `structured-d-3511-1795333227465565000`, the directory printed by the original **focused** run. Local byte comparisons matched all 12 output pairs and the archive triplet. The [structured replay](../phase2-local/tests/milestone_d_structured.rs) dropped the source fixture/handle, restored the saved JSON to a fresh path, reopened it, and regenerated from stored records, not saved markup. This is a typed read model, not a full legacy database or posting-authority restore. Later local checks link to their own saved results and logs, not to additional files in this retained directory. HTML files and captured markup are not printed output or PDFs.

| Case / artifact | Source | Restored or regenerated |
| --- | --- | --- |
| L19-01 typed archive | [source.json](../phase2-local/target/structured-d-3511-1795333227465565000/source.json) · [backup.json](../phase2-local/target/structured-d-3511-1795333227465565000/backup.json) | [restored.json](../phase2-local/target/structured-d-3511-1795333227465565000/restored.json) |
| L05-01 estimate markup | [estimate.html](../phase2-local/target/structured-d-3511-1795333227465565000/estimate.html) | [regenerated-estimate.html](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-estimate.html) |
| L05 bill SVG model | [bill.svg](../phase2-local/target/structured-d-3511-1795333227465565000/bill.svg) | [regenerated-bill.svg](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-bill.svg) |
| L06-01 ordinary GST projection | [gst.json](../phase2-local/target/structured-d-3511-1795333227465565000/gst.json) | [regenerated-gst.json](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-gst.json) |
| L06-02 standalone GST recovery | [L06 rounding fixture](#pc-003-batch-3-addendum-standalone-gst-rounding-and-initial-backup-write-rejection); source store and backup JSON written to scratch disk | [L06-STANDALONE-RECOVERY](#l06-standalone-recovery-regenerate-the-same-standalone-gst-bill-after-restore): restored JSON written to a fresh path and reopened; typed invoices matched fixed expectations and originals. Inert Print and Save PDF substitutes captured matching HTML, not physical prints or PDF files. |
| L06-02 / L18-04 standalone GST archive guards and profile recovery | Valid standalone archive and Owner profile stored in scratch JSON | [L06-DUPLICATE-ID](#l06-duplicate-id-reject-duplicate-standalone-gst-ids-on-restore): no fresh file and occupied store unchanged; original bills readable after reopen. [L06-PROFILE-RESTORE](#l06-profile-restore-preserve-standalone-gst-bills-after-an-owner-profile-save): reopened the rewritten source before backup and reopened the fresh restore; changed profile and original bills matched. |
| L07-03 supplier print-route markup | [purchase-order.html](../phase2-local/target/structured-d-3511-1795333227465565000/purchase-order.html) | [regenerated-purchase-order.html](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-purchase-order.html) |
| L07-03 internal PO progress | [purchase-order-readback.txt](../phase2-local/target/structured-d-3511-1795333227465565000/purchase-order-readback.txt) | [regenerated-purchase-order-readback.txt](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-purchase-order-readback.txt) |
| L17 preview | [rate-list-preview.html](../phase2-local/target/structured-d-3511-1795333227465565000/rate-list-preview.html) | [regenerated-rate-list-preview.html](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-rate-list-preview.html) |
| L17 Print | [rate-list-print.html](../phase2-local/target/structured-d-3511-1795333227465565000/rate-list-print.html) | [regenerated-rate-list-print.html](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-rate-list-print.html) |
| L17 Save PDF route | [rate-list-pdf-print.html](../phase2-local/target/structured-d-3511-1795333227465565000/rate-list-pdf-print.html) | [regenerated-rate-list-pdf-print.html](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-rate-list-pdf-print.html) |
| L17 WhatsApp text | [rate-list-whatsapp.txt](../phase2-local/target/structured-d-3511-1795333227465565000/rate-list-whatsapp.txt) | [regenerated-rate-list-whatsapp.txt](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-rate-list-whatsapp.txt) |
| L17 WhatsApp URI | [rate-list-whatsapp-uri.txt](../phase2-local/target/structured-d-3511-1795333227465565000/rate-list-whatsapp-uri.txt) | [regenerated-rate-list-whatsapp-uri.txt](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-rate-list-whatsapp-uri.txt) |
| L19-03 Sales CSV | [sales.csv](../phase2-local/target/structured-d-3511-1795333227465565000/sales.csv) | [regenerated-sales.csv](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-sales.csv) |
| L19-03 SpreadsheetML (XML, not native Excel readback) | [workbook.xls](../phase2-local/target/structured-d-3511-1795333227465565000/workbook.xls) | [regenerated-workbook.xls](../phase2-local/target/structured-d-3511-1795333227465565000/regenerated-workbook.xls) |
| L19-02 / L19-08 partial folder backup | [Initial write rejection and success control](#pc-003-batch-3-addendum-standalone-gst-rounding-and-initial-backup-write-rejection); [earlier Connect and retry](#folder-backup-scheduling-and-reconnect) | [L19-LATE-RESTORE](#l19-late-restore-recover-the-current-copy-after-a-partial-folder-backup): Manual Save reported failure after injected dated publication failure. The current JSON was written and read back, the dated copy kept its old bytes, and the current payload restored to a fresh file and reopened. The local filesystem returned `AlreadyExists` at the occupied path; neither backup changed during recovery. |

**Matched controls and limits.** The cancelled Feb 21, 2025 bill #1 is from the C fixture; the Mar 1 Online receipt of ₹50 is a separate linked record. As of Mar 1, Latest shows ₹−50.00, not the ₹310.00 bill-date due; historical total/paid remain ₹410.00/₹100.00. The active control belongs to another account. Exact signed and aggregate-first rounding controls used separate copied archives at fixed cutoffs, not arbitrary-date or live-clock evidence. CSV and SpreadsheetML have different Sales schemas/order and preserve first-line-only bill totals; the JSON retains nested crate components, links and the standalone receipt. The SVG is generated model output, not a native bill image or layout acceptance. The retained `gst.json` pair contains cancelled ordinary-sale tax projections; the standalone GST evidence comes from the separate runs linked above. The archived Feb 10 `PO-001` comes from the shared partial-PO fixture, not the completed Batch 2 order: received 8/5, outstanding 4/4. Supplier HTML shows original quantities and MRPs, not buy rates, receipt progress or total; the internal readback retains progress. PO print intent is delayed 350 ms. Rate-list print/PDF routes record two `_rl` popup attempts (`width=820,height=900`) and identical inert HTML, not PDFs. The filtered Alpha selection persisted after being hidden; picker ₹123 versus escaped preview/text ₹123.46, with no buy cost or unselected rows. The text and percent-encoded `wa.me` URI were captured, not opened or sent.

**Settings, restore and access (L18-01/04, L19-01/03).** Reorder default 5 → 13 survived reopen and restore. The primary estimate shows escaped FSSAI and hides GSTIN. A separate two-directory simulation switched explicit visibility to GSTIN on/FSSAI off; its [origin estimate](../phase2-local/target/structured-d-3511-1795333227465565000/device-recovery/origin/estimate.html) and [destination estimate](../phase2-local/target/structured-d-3511-1795333227465565000/device-recovery/destination/estimate.html) matched, and both business files matched the [scenario backup](../phase2-local/target/structured-d-3511-1795333227465565000/device-recovery/backup.json). Bill body, records and exports were unchanged. Origin dark/privacy on/Latest and destination light/privacy off/AsOnBillDate stayed local, absent from the business archive. Malformed JSON, version 2 and invalid Feb 31 did not create a fresh destination; valid restore could not overwrite an existing destination. The two-directory invalid attempts preserved both business and preference state. Manager setting/profile mutations, restricted documents (estimate/bill, GST, PO and Rate List), backup and restore were denied where exercised; CSV and SpreadsheetML were denied on both source and reopened store. These are local API checks, not device isolation.

### Companion checks and dispositions

The companion targets run their own fixtures; they do not jointly operate on the retained archive.

| Saved focused target / case | Observed scope |
| --- | --- |
| `milestone_d_replay` — L05-02–05, L06-01, L07-03, L13-03, L19-01 | Dated billing/SQLite readback with inert estimate `_print` on load and `_pdf` after 350 ms, share/clipboard requests and copied CSV (not regenerated there). Separate tests exercised active Sales-row contact routing, cancelled history reopen/text fallback and Manager denial before callbacks. PNG helper used fixture bytes for blob/data-URL branches; null did not retry the helper, estimate used its separate canvas fallback, PO used text without a second render. The logged scratch report directory is not retained; no native image or output delivery claim. |
| Shared partial-PO setup in both D targets | Each target counts its setup test once; the Feb 12/11 receipts stop at PartiallyReceived, before Close Remaining. Neither adds a Batch 2 dispatch/count run. |
| `preference_replay` — L18 presentation/privacy | Successful/failed saves and read failure checked current versus reopened theme, masking and balance mode, with business bytes unchanged. Fixed Mar 1 Latest ₹90 versus bill-date ₹150 belongs to this fixture. Privacy on/off did not change Manager denials or provisional draft save/reopen. |
| `rate_list_replay` — L17 | Independent selection/session, multi-brand sort with catalog order within groups, exact inert outputs with restricted fields excluded, and empty/blocked-popup cases. |
| `folder_backup_replay` — L19-02/05/06/07/08 | Real scratch JSON files with simulated folder handles, permissions, and failure injection: same-day preservation and rollover, burst timer replacement and execution-time data/permission, no prompting on auto/restore, manual permission and reconnect, token reopen/false-store/denied-connect, picker cancel/failure, and dated publication failure after current close with retry. See the later partial-backup restore evidence in the manifest above. No browser folder handles or installed-device durability. |
| `clear_restore_replay` — L01 arbitration / L18 clear / L19 restore | Typed subset and simulated slots: decline, both writes, either split write, highest revision, mirror on equal revisions even with different data, and imported revision 3 → 4. Mirror-only clear allowed primary revision 9 to win; follow-up wrote revision 10 to both, assuming success. No browser persistence, transport, watermarks or folder handles. |

**Not run / blocked.** Actual print/PDF generation, native image/layout checks, clipboard/share/WhatsApp actions, Excel opening and installed-device workflows were not run (OPEN-005). Frozen runtime and browser isolation verification were not run (OPEN-008). L20-01–04 guards/merge/rebuild, L20-05/06/10/14/18 Sheets push/readback/navigation, L20-07/08 pricing snapshots/deltas, and L20-09/11/12/13/15/16/17 cloud scheduling/tombstones/mirror/pull were not established by these runs: local slot arbitration and folder scheduling above are not sync transports or convergence tests. Hosted/protected-data evidence remains blocked (OPEN-002/006). See [case dispositions](MILESTONE_D_COVERAGE.md) for earlier scoped results and remaining variants. No owner acceptance or phase exit claimed.

## PC-003 Batch 3 addendum: L20 sync replay reconciliation

**Separate Supervisor follow-up at `289a1f5`, development evidence only.** The prior Batch 3 runs and artifact manifest remain valid at their recorded scope. This addendum maps the saved runs to the [L20 case matrix and source-order audit](PHASE1_CASES.md).

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test l20_cloud_replay --test l20_records_replay --test l20_mirror_replay --test l20_sheets_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B3-L20-289a1f5-focused](../phase2-local/target/B3-L20-289a1f5-focused.log): **47 passed, 0 failed, 0 ignored** — cloud 16, records 9, mirror 5, Sheets 17.
- [B3-L20-289a1f5-offline](../phase2-local/target/B3-L20-289a1f5-offline.log): **516 passed, 0 failed, 2 ignored** across 45 summaries including the zero-test doc target; includes the focused tests. Individually ignored: `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`, subprocess entry points dispatched by their restart tests, not independent passes.
- Formatting, all-target Clippy with warnings denied, and locked offline build passed at the tested checkpoint; LSP diagnostics timed out. The commands above document the saved runs, not new executions for this addendum.

**Evidence boundaries.** [Cloud](../phase2-local/tests/l20_cloud_replay.rs), [records](../phase2-local/tests/l20_records_replay.rs), [mirror](../phase2-local/tests/l20_mirror_replay.rs) and [Sheets](../phase2-local/tests/l20_sheets_replay.rs) are four source-derived Rust models under `tests/`, not shipping sync code or executions of the frozen HTML. RPC/POST/JSONP/navigation entries are captured in process, with scripted replies and clock steps; they are not network traffic. Merge, timers, callback and UI flags are modeled in memory. Where checked, reopened copies, progress/stamp checkpoints and serialized payloads are scratch files; modeled write failures are injected, not OS failure tests. A file readback is not IndexedDB, localStorage, or deployed-service durability.

| Case | Observed local disposition | Remaining scope |
| --- | --- | --- |
| L20-01 | With both integrations off, **zero requests** and the guarded save in both reopened DB copies. Separately exercised cloud disabled and each missing URL/key/token, plus missing Sheets URL. | No browser storage, OS egress positive/deny control or installed-device save. |
| L20-02 | Captured 100+1 pushes and paged pulls; sent timestamps advanced `pushed`, echoes and older/equal versions were skipped, versioned meta/stock preserved local fields. A later **push** rejection left the stale-conflict winner in RAM only: progress stayed at pushed 10/seq 70 and both files at baseline; retry resent earlier peers. A later **pull** rejection left file-backed seq 80 ahead of DB rev 40, with applied data not durable; successful comparison reloaded rev 41 at seq 80. | No deployed ordering, transaction or convergence proof. Tombstone and busy-view controls are separate below. |
| L20-03 | Scripted revoked/401/403 stopped cloud locally; ordinary failures warned on the third silent failure and the next pull recovered. Optional mirror/snapshot rejection did not undo the completed record save or advance optional success timestamps. | No actual credential revocation, reconnect or hosted permissions test. |
| L20-04 | First upload rewound the push watermark while preserving the pull cursor, then saved even when the explicit snapshot rejected. Daily snapshot used the strict time/idle gate. Confirmed rebuild split safety copy, replacement and catch-up; checked absent/malformed/migration/RPC failures and split write outcomes against reload. A success notice did not guarantee the replacement won reload. | No deployed RPC schema, real migration pipeline, interruption recovery or cross-device convergence evidence. |
| L20-05 | Captured immutable full-DB POST body with device/time stamp. Opaque POST alone did not confirm; matching stamped readback at callback advanced confirmation, mismatch/failure did not. Frozen-clock trailing checks captured the stamp at fire time and overlapping confirmations used their own stamps out of order. | No browser fetch/opaque-response behavior, Apps Script execution or live workbook readback. |
| L20-06 | New unseen foreign stamp blocked normal push; Later kept conflict, confirmed Force bypassed precheck, and Pull Latest waited for confirmation. Older/equal/same-device/unreadable prechecks, missing/failed/malformed pull, and sidebar precedence/stale click were modeled. Valid pull replacement, seen-stamp checkpoint, primary completion and reload selection were checked separately; success feedback did not prove durability. | No real dialogs, multi-device conflict handling, workbook authority or reconciliation decision (OPEN-002/003). |
| L20-07 | Shared pricing helper modeled both editor effects, contact fallback and quoted-contact column retention. Files verified raw order and quote maps for full copies; JSON restore rewound pushed watermark, rebuild set its cursor before separate catch-up, and Sheets replacement preserved its cursor. Missing/non-array raw lists migrated to empty without moving quotes. | Actual editor entry points, full legacy DB migration and service-sourced snapshots not exercised. The three replacement routes are independent typed fixtures. |
| L20-08 | Captured ordinary envelopes carried stamped contacts and product quotes but no outgoing raw `priceSuppliers`; divergent receiver kept its raw list, new contact preceded contact-only fallbacks, quote alone added no column and contact tombstone did not run local retention. File serialization/readback is scoped to the model. | Generic unsolicited inbound raw-list collection was not established; no service filtering or convergence claim. |
| L20-09 | Modeled the sync timers, **not** the folder backup timer: 3,000 ms cloud debounce, missing prerequisite checks, and no dispatch or automatic retry on disabled/busy/missing DB at fire. Separately stepped the 15,000 ms interval with strict >55,000 ms gate and hidden/visible paths. With both configured, Sheets fired before cloud. | No actual browser timer/visibility callbacks or elapsed-time measurements; no full interval-by-visibility-by-entry-guard cross-product. |
| L20-10 | Replaced 2,500 ms Sheets timer on burst; callback used current URL (added/removed), recent direct POST versus stale precheck, and silent rejected POST without extra requests or file changes. Silent conflict behavior is separately scoped under L20-06. | No live timer, browser network, or full combined scheduling/conflict/storage-failure matrix. |
| L20-11 | Repeated GST helper deletion refreshed the same marker, advanced version/time/device, and enforced strict outgoing watermark. Equal/older/newer marker replay and independent row removal checked, with sender/receiver file readback. | Second helper call is not a second UI delete; no hosted retention or replay. |
| L20-12 | Both update/delete orders ended with the ordinary row absent and marker retained, with the same-ID other collection unchanged; checked serialized final states. | Not a meta/stock merge or proof of server ordering. |
| L20-13 | Local 4,000-mark boundary retained array order; refresh did not move slot; exceeding cap kept last 3,000. Inbound append did not trim. Retained marker blocked an otherwise applicable update; trimmed marker did not, with fixture file readback. | No service retention or actual resurrection inference. |
| L20-14 | Conditional Open Sheet captured configured URL plus literal `?action=open`, target `_blank`; absent URL gave no navigation request. | No browser Settings rendering, popup, navigation, workbook access or alternate URL-shape check. |
| L20-15 | Captured five derived tabs (Day Summary, Sales, Udhaar, Stock, Payments) with 45-day cutoff and per-tab rounding; not record envelopes, a full snapshot or a stamped Sheets POST. Strict >15-minute dispatch, completion timestamp, rejection/retry and failed checkpoint file write were distinguished; separate daily snapshot gate remained independent. | No actual remote tab materialization/persistence, live timezone behavior or authority claim. |
| L20-16 | Empty terminal pull took 2 calls (seq 72), cap took 200 (seq 270); both kept DB copies at rev 40. A failed progress write did not become the next durable starting cursor. | Empty-page cap fixture is not a loaded/mixed-page service run; no browser cursor storage. |
| L20-17 | Applied pull saved even when modal, current bill/GST draft or INPUT/SELECT/TEXTAREA suppressed redraw. Idle and off-screen bill/GST drafts allowed redraw in the recorded view. | No actual DOM focus, active editing preservation or full redraw screen matrix. |
| L20-18 | Captured modeled script/callback registration and cleanup for success, load error and no callback; no DOM or script execution. | No actual script load, browser failure/timing integration or remote endpoint behavior. |

**Prerequisites still open.** The source-only gaps in the table are not local test failures. Actual frozen-browser and OS-level egress evidence was **not run**: OPEN-008 requires the positive control and OS deny evidence before serving the frozen baseline, with L01 first. Installed-device checks remain unrun (OPEN-005). Hosted schema/permissions, real data, cross-device persistence and remote acceptance remain **blocked** pending separate authorization (OPEN-002/006); source authority/reconciliation remains OPEN-003. No credentials or external services were used, no artifacts were rebuilt, and no parity, acceptance or phase exit is claimed.

## PC-003 Batch 1 addendum: Named-customer undo of an edited invoice

**Local synthetic development at `5aa66d2`.** [L04-02](PHASE1_CASES.md#counter-checkout-edit-and-outputs) and linked estimate, ledger, Cash Book and stock checks. The [joined replay](../phase2-local/tests/milestone_c_replay.rs#L200-L221) uses [undo readback](../phase2-local/tests/milestone_c_replay/undo.rs) and [dated books and saved-close checks](../phase2-local/tests/milestone_c_replay/books.rs).

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test milestone_c_replay -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B1-UNDO-5aa66d2-focused](../phase2-local/target/B1-UNDO-5aa66d2-focused.log): **1 passed, 0 failed, 0 ignored**. Undo and books are assertions within that one test.
- [B1-UNDO-5aa66d2-offline](../phase2-local/target/B1-UNDO-5aa66d2-offline.log): **516 passed, 0 failed, 2 ignored** across 45 summaries, including doctests. The ignored entries are subprocess workers `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`. Formatting, Clippy and build passed; LSP diagnostics timed out.

Amounts are paise. The fixed identity was `DraftId(21)`, invoice #1, dated 2025-02-21, for `milestone-c-buyer` (Synthetic Buyer). The fixed undo control was equality to the **edited** readback, not the pre-edit sale. Observations matched:

| Checkpoint | Fixed control and observed readback |
| --- | --- |
| Before edit, after independent receipt | Original bill A 2 × 11,800, total 23,600, Cash 10,000; bill-date due 13,600 and Latest 8,600 after the separate 2025-03-01 Online receipt of 5,000. FIFO remaining value 19,000. |
| Edited | A 2 × 17,700 plus B 1 × 5,600 = 41,000. Same Cash payment, bill-date due 31,000, Latest and ledger closing 26,000. FIFO remaining A/B 2/1 worth 16,500; February sales/cost/profit 41,000/15,500/25,500. CSV totals/paid only on the first line. |
| Cancelled | Edited record and 31,000 bill-date due retained; both CSV rows marked `CANCELLED`. Active February sales/cost/profit and live bill Cash were zero; FIFO returned to 4/2 worth 32,000. The sole ledger collection was the March Online receipt; Latest and ledger closing were −5,000. Manager undo returned `AccessDenied` with the cancelled readback unchanged. |
| Owner undo | Same edited review/receipt, posted count, lots, previews, CSV and report readback as before cancellation; no new invoice or duplicate payment. Both CSV flags cleared and ordinary GST matched the edited active projection. Bill-date/Latest estimates returned to ₹310.00/₹260.00. Exactly one Feb 21 bill ledger debit 41,000 and Cash credit 10,000, plus one Mar 1 Online credit 5,000; combined collections 15,000 and ledger closing 26,000. Sales drilldown identified the separate receipt as fixture-local `SalesReceiptId(1)`, not a durable receipt ID. |

**Read-only close comparison.** The same in-memory `ManualCashBook` BTreeMap supplied both saved rows by their original, distinct row IDs, with no resave. Cancellation left the saved Feb 21 ₹100 Cash intact against live ₹0; undo restored live ₹100 and zero difference. Mar 1 remained ₹50 Online with zero difference throughout. Manager ledger, history, live close, copy, save, row read and comparison were denied in the linked controls.

**Storage boundary.** [SQLite setup](../phase2-local/tests/milestone_c_replay/start.rs#L22-L42) persisted only the original Manager provisional draft in `synthetic-drafts.sqlite3`: draft 21, selected buyer, Feb 21 date and A ×2. A fresh handle reopened it without reseeding and supplied it to the Owner fixture; it reopened unchanged again after the initial post/retry checks. `RequestId(1)` lost-reply/exact-retry and changed-payment conflict were simulated in that in-memory fixture, not a restarted posting authority. The posted and edited invoice, payment/ledger effects, cancellation/undo, FIFO and reports, and saved manual closes were not persisted or reopened from SQLite. No undo or saved-close durability is established. Formal phase gates are unchanged; no acceptance or phase exit claimed.

## PC-003 Batch 2 addendum: Crate FIFO and Dead Stock valuation

**Local synthetic development at `137407e` (crate `46f0d0f`).** Follow-up to the crate and Dead Stock scope limits at [line 490](#L490) and [line 2148](#L2148); prior results and criteria stand.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test l10_crate_fifo_replay --test l16_dead_stock_replay -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --lib dead_stock -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B2-STOCK-137407e-focused](../phase2-local/target/B2-STOCK-137407e-focused.log): **18 passed, 0 failed, 0 ignored** (crate 2, Dead Stock model 4, filtered library 12; library filter excluded 308).
- [B2-STOCK-137407e-offline](../phase2-local/target/B2-STOCK-137407e-offline.log): **522 passed, 0 failed, 2 ignored** across 46 summaries, including the zero-test doctest target. Ignored: `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`, dispatched subprocess entry points. Formatting, all-target Clippy with warnings denied, and locked offline build passed; LSP diagnostics timed out.

**L10-01 — FIFO matched; report parity MISS.** The [crate replay](../phase2-local/tests/l10_crate_fifo_replay.rs) received Feb 12 first (5 × 275 paise), then backdated Feb 11 (3 × 125), for 8 packets worth 1,750. Two crates of the same product dispatched on Feb 13, at revenue 1,000. Review did not dispatch, and Manager review/confirm were denied. The fixed values below matched Rust lots/register/monthly report and, separately, the [source-derived calculation](../phase2-local/tests/l10_crate_fifo_replay/source.rs), not a frozen-runtime execution. Costs are paise unless marked ₹.

| Component packet demands | Rust dispatch → remaining Feb 12 layer | Rust FIFO/report cost; gross | Source-derived report cost; display | Sequential per-crate rounding / separate component rounding controls |
| --- | --- | --- | --- | --- |
| `[3,2]` | 5 → 3 × 275 = 825 | 925; 75 | 626 (₹3.13 per crate); ₹6 | 926 / 926 |
| `[3,1]` | 4 → 4 × 275 = 1,100 | 650; 350 | 500 (₹2.50 per crate); ₹5 | 650 / 652 |

The source calculation reuses the oldest cost for the second component before advancing its offset, then rounds per crate before extending and rounding the display. The controls separate allocation from rounding; passing the differential assertions does not make either case a parity pass. Rust accepted 6 pieces per crate at pack size 4 because two crates yield a whole 3-packet component demand. Two separate half-packet demands (62.5 paise each in the source model) were rejected as `InvalidAmount` even though together they make one packet; a ₹1.125 receipt rate is not representable as integer `Money`. Repeated demands exceeding stock returned `InsufficientStock`. Rejections left stock, movements, posted count and ledger unchanged.

**L16-02–05 — Dead Stock scopes remain separate.** The [library tests](../phase2-local/src/confirmation/reporting/dead_stock_tests.rs) exercise the Rust report; the [integration model](../phase2-local/tests/l16_dead_stock_replay.rs) is source-derived only.

- **Action-created Rust sale:** ID 1 sold Oct 4, 2024; Oct 4 cutoff excluded equality, and Oct 5/6/7 retained Oct 4 as last sale with 4 packets, despite a separately seeded stale ID 91 sale on Oct 6 with the same label. Direct and crate sale rename controls followed the retained ID. Cancellation/undo moved actual stock and changed report membership; positive current stock, stable ties/cap, overflow and Manager denial were checked separately.
- **Private seeded Rust history:** an undated sale still depleted stock but did not supply a last-sale date. Undated-only left 4 packets/last `None`; with an older dated sale, 3 packets remained/last Oct 3. Public posting rejected missing date (`MissingBillDate`) and unknown product (`InvalidDraft`); seeded history is not an alternate public posting route.
- **Name-only helper, not Rust posting:** exact current label resolved the unkeyed line; case mismatch fell back to a separate name key, stale explicit ID stayed stale, and retained ID survived a label change. Oct 4 equality was excluded and Oct 3 qualified. The source model also allowed fractional stock/value: 2.5 packets gave raw ₹10.625, displayed ₹11, versus modeled FIFO ₹65; raw-value order was independent of equal rounded display. No Rust ingestion of name-only history, fractional stock, or sub-paise money was shown.
- **Buy-rate estimate is not FIFO:** ID 1's 4 × 325 = 1,300 versus 500 FIFO; the mixed undated/dated seed gave 975 versus 420. Catalog buy won 950 over a registered 720; a buy edit gave 1,690 versus unchanged FIFO 200, and receipt-updated buy gave 3,705 versus FIFO 1,435. Cutoffs filter last sale against current stock, not stock as of the cutoff.

No frozen runtime, hosted or production evidence. The parity miss records a difference, not an approved change to valuation or an accounting rule. Formal gates remain unchanged; no acceptance or phase exit claimed.

## PC-003 Batch 3 addendum: Standalone GST rounding and initial backup write rejection

**Local synthetic development at `f52d549` (GST `6ee53ed`).** An interim GST test failed and its retry timed out during uncommitted parallel development; no cause was established. The final saved runs below are separate from those attempts.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test gst_preview_replay --test folder_backup_replay -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B3-GST-BACKUP-f52d549-focused](../phase2-local/target/B3-GST-BACKUP-f52d549-focused.log): **18 passed, 0 failed, 0 ignored** (GST 5, folder backup 13).
- [B3-GST-BACKUP-f52d549-offline](../phase2-local/target/B3-GST-BACKUP-f52d549-offline.log): **525 passed, 0 failed, 2 ignored** across 46 summaries, including the zero-test doctest target. The ignored entries are dispatched restart workers `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`.
- Formatting passed. Lint and build are tracked separately from these test logs; LSP diagnostics timed out.

**L06-02 — ₹105 inclusive at 18%.** The [standalone action replay](../phase2-local/tests/gst_preview_replay/rounding.rs) used Owner form, Save & View, and saved-record readback/reprint for intra-state `GstBillId(91)`, invoice #47 dated 2025-01-02. Per line, the source-derived raw taxable amount was ₹5250/59, tax ₹945/59, and each half ₹945/118. The fixed values and observed saved/readback values are in paise, ordered taxable, tax, CGST, SGST, IGST, total:

| Fixture | Fixed expected | Observed |
| --- | --- | --- |
| One ₹105 line | `[8898, 1602, 801, 801, 0, 10500]` | `[8898, 1602, 801, 801, 0, 10500]` |
| Two ₹105 lines | `[17797, 3203, 1602, 1602, 0, 21000]` | `[17797, 3203, 1602, 1602, 0, 21000]` |

The multiline summary rounds raw aggregates independently: taxable ₹177.97, tax ₹32.03, CGST and SGST ₹16.02 each, grand total ₹210.00; it neither sums the displayed ₹88.98 line values nor forces the rounded halves to equal the rounded tax. No Round Off row. Manager save/reprint denied before sinks; stock, movement, ordinary posting, ledgers and customer due remained unchanged. Print/Save PDF captured equal HTML; companion popup and 12/13/20/32/33-row pagination checks were inert. This is standalone fixture evidence, not disk restart, ordinary-sale GST, rendered PDF, or accounting approval.

**L19-08 — initial current write rejection.** The [connection replay](../phase2-local/tests/folder_backup_replay/connection.rs) seeded two occupied files with distinct raw JSON bytes: `ALM_data.json` (`old current`, `last_file_backup:41`) and `ALM_backup_2024-05-01.json` (`old dated`, `last_file_backup:37`), both with `backup_folder:"new"`. The attempted payload was `{"records":["initial connect"],"meta":{"backup_folder":"new","last_file_backup":7}}`. As fixed, the injected current-write rejection occurred **before accepting bytes**, with trace `query → current-write-failed`: both files read back their own unchanged old bytes, with no current close or dated attempt; modeled last backup stayed 7 and daily unset. “Backup folder set” still appeared and the modeled token reopened as `new`. On a fresh equivalent sandbox, the success control traced `query → current-closed → dated-staged → dated-closed`; both existing destinations read back the exact new payload, with modeled last backup 100 and daily `2024-05-01`. This is distinct from the companion later dated-publication failure after current close. It proves neither crash/OS atomicity nor browser file-handle durability.

The prior [`d6ac8d9` crate parity miss](#pc-003-batch-2-addendum-crate-fifo-and-dead-stock-valuation) and [`OPEN-010`](OPEN.md#open-010-repeated-product-crate-valuation-l10-01) / [`DEC-027`](DECISIONS.md#dec-027-defer-repeated-product-crate-valuation-l10-01) deferral at `c20d0d3` are unchanged: neither source-derived saved cost nor depleted-lot cost is approved for release. Frozen runtime and formal acceptance gates remain unchanged; no phase exit claimed.

## PC-003 Batch 3 addendum: L20 inbound pricing, cloud callbacks and mixed pull cap

**Test-only local development at `7cde982`.** Fixed assertions matched the observations below; recorded requests are in-process, not network calls. No frozen browser or hosted service was exercised.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test l20_records_replay --test l20_sheets_replay --test l20_cloud_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [B3-SYNC-7cde982-focused](../phase2-local/target/B3-SYNC-7cde982-focused.log): **46 passed, 0 failed, 0 ignored** (records 10, Sheets 19, cloud 17).
- [B3-SYNC-7cde982-offline](../phase2-local/target/B3-SYNC-7cde982-offline.log): **529 passed, 0 failed, 2 ignored** across 46 summaries. Ignored subprocess entry points: `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`. Formatting, all-target Clippy with warnings denied, and build passed; LSP diagnostics timed out.

**L20-08 — inbound is not outgoing.** The [delta replay](../phase2-local/tests/l20_records_replay/pricing_deltas.rs) captured a pull from seq 41, limit 500, for `synthetic-receiver`. Scripted input was `priceSuppliers/ps-inbound` v2, m70, from `synthetic-sender`, with name `Existing Contact` and `extra.synthetic=true`. It applied once; cursor became seq 42 with pushed watermark still 9. Reopened JSON kept the object intact ahead of the strings `Receiver Raw` and `ps-inbound`; the modeled visible headers were `[object Object]`, `Receiver Raw`, `ps-inbound`, `Existing Contact`. The ordinary outgoing envelope list contained only product `p-quote` v3/m10 with quote 630 for `Existing Contact`, not the raw list or inbound object. This is a generic inbound model branch, not evidence that a service sends or accepts it.

**L20-09 — callback, entry and dispatch are separate.** The [clock-driven matrix](../phase2-local/tests/l20_sheets_replay/cloud_schedule_cases.rs) checks cloud sync, not the folder timer:

| Trigger | Fixed and observed |
| --- | --- |
| Save | Disabled or missing URL/key/token called kick without arming. Two eligible saves 500 ms apart replaced the 3,000 ms timeout (due at +3,500 ms). The replacement fired and entered sync for ready, disabled, missing-token, busy and missing-DB states; only ready captured `sync_pull`. No self-retry. Both files retained the two saves at rev 42, separate from dispatch. |
| Interval | At the 15,000 ms tick, visible/ready at ages 54,999 or 55,000 ms did not enter; 55,001 ms entered and dispatched. Disabled, missing token or hidden stopped before entry; busy and missing DB entered without dispatch. No file writes. |
| Visibility | Hidden event did not kick; visible event kicked without an age check and replaced the pending timeout when configured. Disabled configuration did not arm. Becoming hidden before the replacement fired did not stop entry; across the same five entry states only ready dispatched. No file writes. |

**L20-16 — mixed cap and continuation.** The [cap fixture](../phase2-local/tests/l20_cloud_replay/pull_cap_tests.rs) asserted all 200 pulls (`since` 70 through 269, limit 500), all pages `more=true`, and a separate successful checkpoint per page. It applied Products/1 v3, Customers/7 v1 then v2, and Stock/20 v1 on the capped tail. It skipped local echo Products/9 v9, equal and older Products/1 v3/v2, and tombstoned Products/8 v99. Durable seq reached 270; mirror had rev 41 before the pending primary settled. After settlement both independently reopened copies matched **rev 41**: Stock/20 v1, Customers/7 v2, Products/1 v3, Deleted/8 v4. The next invocation used `since=270` then `275` (limit 500); it added Products/21 v1 and upgraded Stock/20 to v2, skipping equal Customers/7 v2 and echo Products/9 v10. At durable seq 280 both reopened copies matched **rev 42**: Products/21 v1, Stock/20 v2, Customers/7 v2, Products/1 v3, Deleted/8 v4. File readback after modeled settlement is not browser or service durability.

No network, production, parity acceptance or phase exit claim. [DEC-027](DECISIONS.md#dec-027-defer-repeated-product-crate-valuation-l10-01) and [OPEN-010](OPEN.md#open-010-repeated-product-crate-valuation-l10-01) remain unchanged.

## Development reconciliation: PO receipt identity and delta selection

**Tested `cef20dc`.** Follow-up to the [receipt identity case](PHASE1_CASES.md#bounded-receipt-cost-and-supplier-quote-helpers-not-run) and [SRC-STK / SRC-PRODUCT crosswalk](PHASE1_DISCOVERY.md#per-record-source-crosswalk). The legacy-shaped records replay and the public Rust receiving replay are independent synthetic fixtures; matching IDs or document numbers do not link them.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test l08_public_receiving_replay --test l20_records_replay -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [PO-IDENTITY-cef20dc-focused](../phase2-local/target/PO-IDENTITY-cef20dc-focused.log): **13 passed, 0 failed, 0 ignored** (public receiving 2, records 11).
- [PO-IDENTITY-cef20dc-offline](../phase2-local/target/PO-IDENTITY-cef20dc-offline.log): **532 passed, 0 failed, 2 ignored** across 47 summaries. Ignored subprocess workers: `confirmation::tests::restart::client_worker` and `sqlite_restart_worker`. Formatting, build and Clippy passed; LSP timed out.

**Legacy-shaped records model (L08-01 / L20-02).** In both [receipt controls](../phase2-local/tests/l20_records_replay/receipt_cases.rs), fixed expectations matched before, after and on reopen. Catalog order is ID `1` then `2`. Parent `po-receipt` (`PO-001`, dated 2025-02-11, `created=90`, supplier `Synthetic Supplier` / `s-receipt`, note `Synthetic receipt fixture`, not cancelled) retains those fields. Line index 0 retains PID `2`, item `Synthetic B` in the unique case or `Synthetic A` in the seeded exact-duplicate case, ordered 2 and cancelled 0. It changes from received 0, rate ₹2, no receipt array, to received 2, rate ₹3, and receipt ordinal 1 (index 0): 2025-02-12, 2 packets at ₹3. Parent stamp changes from `(1,99,synthetic-before)` to `(2,110,synthetic-receiver)`.

Money below is **rupees in legacy JSON**; value tuples are `(avgCost, buyRate, margin, sellRate, quote)`, with quote key `Synthetic Supplier` throughout. Stamp tuples are `(_v,_m,_d)`; `_m` is a synthetic clock tick, not the receipt document date. Both products start at stamp `(1,100,synthetic-before)`.

| Control / product | Exact name | Before → after values | After stamp |
| --- | --- | --- | --- |
| Unique / `1` | Synthetic A | `(1,1,4,5,1)` unchanged | `(1,100,synthetic-before)` |
| Unique / `2` | Synthetic B | `(2,2,3,5,2)` → `(2.5,3,2,5,3)` | `(2,110,synthetic-receiver)` |
| Duplicate / `1` | Synthetic A | `(1,1,4,5,1)` → `(2,3,2,5,3)` | `(1,100,synthetic-before)` |
| Duplicate / `2` | Synthetic A | `(2,2,3,5,2)` unchanged | `(2,110,synthetic-receiver)` |

The duplicate model mutates the first exact-name product but stamps the PID-resolved second product. ID-based on-hand moves from `[2,2]` to `[2,4]` in both cases; name-based reads show `[2,2]` after receipt in the duplicate case versus `[2,4]` in the unique case. Opening slots stay unchanged. The model revision goes 4 → 5 and adds `grn` activity `lg-receipt` at stamp `(1,110,synthetic-receiver)`.

**Selection versus full-copy readback.** At the same synthetic clock boundary, strict `_m > pushedM` selects nothing before receipt at `pushedM=100`. After receipt, watermarks 100 and 109 both select, in order, `purchaseOrders/po-receipt`, `products/2`, and `log/lg-receipt`, with their exact post-state payloads; 110 selects nothing. The duplicate's mutated `products/1` is excluded at equality with 100, not because its content is unchanged. The test wrote `receipt-full.json`, dropped the model and reopened it, verifying both products, the nested receipt, ID-based stock and the asserted selection at 100 and 110. Selection is not sending, acknowledgment or receiver persistence; the reopened scratch JSON is not a hosted or browser write. An absent delta proves neither prior upload nor data loss nor source authority.

**Public Rust receiving and catalog guard (L08-01 / L09-01).** The [public replay](../phase2-local/tests/l08_public_receiving_replay.rs) independently exercised unique registered purchase names and trusted registration of the same purchase name for both IDs. Public catalog creation/edit rejected an exact duplicate and the padded case-variant edit; the two catalog rows remained `Synthetic A` / `Synthetic B`. In both receiving controls, the same PO ID, supplier and note survived: `PO-001` dated 2025-02-10 went Pending → Completed. Its sole line retained PID 2 and ordered 2, with received 0 → 2, remaining 2 → 0, rate 200 → 300 **paise**, and receipt ordinal 1 dated 2025-02-12 for 2 at 300 paise. Line text followed the registered metadata. In paise, PID 1 stayed `(100,100,400,500,100)` at stock 2; PID 2 changed `(200,200,300,500,200)` → `(250,300,200,500,300)` and stock 2 → 4, with quote keyed to `Synthetic Supplier`. Trusted duplicate purchase metadata is not a bypass demonstrated through public catalog Save. This fixture exposes neither legacy stamps nor `pushedM`, and does not establish restart durability.

Development evidence only. OPEN-002/003/008 are unchanged; no protected-source comparison, frozen-runtime pass or phase exit.

## DISC-WORKFLOW-SEED: Offline fixture validation

**Development — offline assertions passed.** Fixture commit `8252d93`; artifact [phase1-workflow-seed.json](phase1-workflow-seed.json); SHA-256 `03c9c8ef6021ec352dbee0e972ba325856dcf44ae339b61a479f60c59252959a`, matching the [fixture manifest](PHASE1_DISCOVERY.md#l02l19-shared-workflow-fixture-manifest-not-run).

The prior Python check parsed the JSON with decimal arithmetic, checked the byte hash against the manifest, confirmed the blank sync URL and product ID links in stock, sales and POs, and asserted line arithmetic, subtotal plus loading, buyer balance arithmetic, date-sorted receipt quantities/rates, the two FIFO value calculations, GST taxable arithmetic and the duplicate-name receipt blend. These were offline checks of fixture data and calculations; the manifest clock, balances and readbacks are expected controls, not runtime observations. No replay was run. **OPEN-008 remains NOT RUN:** use a supported disposable guest and pass the full inert gate before serving the candidate, then run synthetic L01 first.

## DISC-L01-UPGRADE-INPUT: Offline L01-12/13 recipe check

**Development — offline input check passed.** Recipe commit `ceca28b`: [L01-12/13 differential upgrade recipes](PHASE1_DISCOVERY.md#l01-12--l01-13-differential-upgrade-recipes-not-run). The prior Python check constructed the synthetic schema variants and supplier-order controls from `baseDB` and verified JSON serialization round-trip. Migration was not executed: safety-copy contents, generated IDs and links, and post-save reload were not observed.

**OPEN-008 remains NOT RUN.** Keep the HTML unserved until the full inert gate passes in a supported disposable guest, then run synthetic L01 first.

## DISC-L01-15-PADDING: Offline cutoff measurement

**Development — in-memory Node assertions passed.** The [reproduction command](PHASE1_DISCOVERY.md#l01-15-mirror-cutoff-padding-recipe-not-run-in-browser) verified the unchanged [seed](phase1-storage-arbitration.json) SHA-256 `88abb98959688a8ea702a0146cdeec4c3179b526fbcebe9b665ae98938bdf559`. Independent copies used numeric revision 9 before padding measurement and 10 in the post-increment copy. Native `JSON.stringify(...).length` measured 644 UTF-16 units at revision 10 with the empty pad field.

| ASCII `A` count in `syntheticPad` | Measured rev 9 length | Measured rev 10 length | Expected legacy write attempts, not observed |
| ---: | ---: | ---: | --- |
| 4,499,355 | 4,499,998 | 4,499,999 | Mirror and IndexedDB |
| 4,499,356 | 4,499,999 | 4,500,000 | IndexedDB only |

No expanded payload was written, and no storage operation, mock, browser or replay ran. **OPEN-008 remains NOT RUN**; a supported guest must pass the full inert gate before serving the candidate and running synthetic L01 first.

## R-5/R-6 development: Synthetic Manager account switch and restart

**Tested `be3f1c9` (implementation `a284c35`).** One disposable device root, two trusted synthetic selectors, and the same `DraftId(21)` in both. The [switch replay](../phase2-local/tests/account_switch_restart.rs) selected A → B → A → B in one writer process, then launched independent `reopen-a` and `reopen-b` processes without reseeding. Each fresh selection initially returned no draft 21 before its first save; in particular B could not see A's saved 21. A's edit left B unchanged. Both restart reads matched their own payload, returned no draft 99, and showed no other-account marker.

| Selector / checkpoint | Fixed payload, matched by Manager and SQLite readback |
| --- | --- |
| `synthetic-manager-a`, initial | Selected `SYNTHETIC-A-buyer` / `SYNTHETIC-A First's Shop`, 2025-02-11. Ordered lines PID 7 ×2, PID 9 ×4. Two crates, components PID 7 ×3 Piece then PID 9 ×1 Case. |
| `synthetic-manager-a`, edited and reopened | Same buyer ID, `SYNTHETIC-A Revised's Shop`, 2025-02-12. Lines PID 9 ×3, PID 7 ×1. One crate, components PID 9 ×2 Case then PID 7 ×4 Piece. SQLite crate definition `1;9,2,1;7,4,0`. |
| `synthetic-manager-b`, saved and reopened | Selected `SYNTHETIC-B-buyer` / `SYNTHETIC-B Second's Shop`, 2025-03-01. Lines PID 7 ×5, PID 9 ×2. Three crates, components PID 9 ×6 Piece then PID 7 ×2 Case. SQLite crate definition `3;9,6,0;7,2,1`. |

All crates were unpriced. [Read-only SQLite inspection](../phase2-local/tests/account_switch_restart/sqlite_evidence.rs) found exactly two stores. It matched the account binding and exact table/view column allowlist: `manager_account_scope(account_hex)` and only the provisional draft ID, customer, date, ordered lines and crate definition tables. The inspected dumps lacked the other account's marker/binding and the checked cost, margin, profit, invoice, payment and stock markers. Reopen workers added no store and left both logical snapshots unchanged.

**Independent posting controls.** In the writer, both reopened Manager drafts received `AccessDenied` on reconnect review in a separate in-memory Owner fixture; draft 21 never appeared as a posted sale. The full Owner readbacks stayed equal after saves, switches and denied reviews. That nonzero baseline (paise) was one invoice #1 on draft 5, sales 3,200, bill Cash 1,100, due 2,100 and gross profit 1,800; PID 7 had 10 left worth 7,000, PID 9 had 11 with unknown cost. B's independent Mar 1 Online receipt of 400 remained, with ledger closing −400; Feb 12 Cash Book stayed empty. The separate legacy CLI probe rejected `post` as unknown and found zero pending Owner confirmations; its two-launch save/review and the SQLite restart controls are not account-switch or shared-authority tests.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test account_switch_restart --test draft_demo --test sqlite_restart -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [R56-SWITCH-be3f1c9-focused](../phase2-local/target/R56-SWITCH-be3f1c9-focused.log): **10 passed, 0 failed, 2 ignored** — account switch 1, CLI 2, SQLite restart 7. The three dispatched switch-worker successes are not additional top-level passes.
- [R56-SWITCH-be3f1c9-offline](../phase2-local/target/R56-SWITCH-be3f1c9-offline.log): **537 passed, 0 failed, 3 ignored** across 48 summaries. Ignored worker entry points: `account_switch_restart_worker` and `sqlite_restart_worker` (both runs), plus `confirmation::tests::restart::client_worker` (offline). Formatting, strict Clippy and build passed; LSP timed out.

**CLI boundary.** [Scoped launch example](../phase2-local/README.md), not a command run for this handoff, using the existing target directory as a trusted root:

```sh
cd /Users/apple/ALM && cargo run --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --bin alm-draft-demo -- --device-root /Users/apple/ALM/phase2-local/target --account synthetic-manager-a
```

Select B by changing only `--account`. The scoped API derives the SQLite pathname and checks its binding; manual DB paths were not used to select accounts. `--db` remains the legacy single-account demo, not a scoped selector. This establishes local draft separation under trusted inputs, not authentication, authorization, revocation, shared posting or OS isolation. OPEN-004/006/008 remain unchanged; no phase exit claimed.

### R56-CLI: Scoped Manager CLI launches

**Development, PASS.** At `fda0b41` (CLI code unchanged from `be3f1c9`), run `R56-CLI.eMnBZF` used one fresh device root and five independent CLI launches. [Full stdout transcript](../phase2-local/target/R56-CLI.eMnBZF/transcript.log). Reproduce with zsh; each `demo` call below starts and exits a new process:

```sh
cd /Users/apple/ALM && root=$(mktemp -d /Users/apple/ALM/phase2-local/target/R56-CLI.XXXXXX)
cd /Users/apple/ALM && demo() { cargo run --quiet --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --bin alm-draft-demo -- --device-root "$root" --account "$1"; }
cd /Users/apple/ALM && demo demo-a <<< $'new\nSynthetic Customer\n1\n3\nsave\nquit'
cd /Users/apple/ALM && demo demo-b <<< $'open\n1\nquit'
cd /Users/apple/ALM && demo demo-b <<< $'new\nSynthetic Customer\n1\n5\nsave\nquit'
cd /Users/apple/ALM && demo demo-a <<< $'open\n1\nreview\nquit'
cd /Users/apple/ALM && demo demo-b <<< $'open\n1\nreview\nquit'
```

All launches printed `Storage: account-scoped local drafts.` Both saves printed `Saved draft 1: PROVISIONAL — NOT POSTED.` Between the saves, B printed `Draft 1 was not found.` Both final reviews printed `PROVISIONAL — NOT POSTED`, `Draft ID: 1`, and `State: SAVED`. A showed `Item: Synthetic Lime | Quantity: 3` and `Inclusive total: ₹354.00`; B showed `Item: Synthetic Lime | Quantity: 5` and `Inclusive total: ₹590.00`. Neither review contained the other's quantity or total. An earlier setup attempt on a separate scratch root used an unsupported customer label and ended without saving; the measured run used the CLI's supported `Synthetic Customer` and distinguished the drafts by quantity.

This checks CLI routing and displayed provisional contents under trusted local selectors. The accepted SQLite evidence above was reused, not rerun. Authentication, hosted authorization, revocation, installed-device isolation, and formal phase exits remain open.

### R56-CLI-EDIT: Explicit save after a scoped quantity edit

**Development, PASS.** The runs tested the quantity-edit changes in the worktree based on `c20d7b4`; the base commit alone does not contain the edit. The reviewed implementation and tests were subsequently committed unchanged as `b21bc6a`. This extends the R-6 reopen/edit control using the R-5 account-scoped selector. `edit` takes a one-based ordinary line number and a positive whole quantity. A changed quantity marks the draft unsaved without writing it; `save` uses the existing provisional store and draft ID. Crate rows are outside the command's scope.

The [fresh-process regression](../phase2-local/tests/draft_demo/scoped_edit.rs) seeded draft ID `1` for each selector on one device root. A started with quantities `(2, 3)` and B with `(4, 5)`. A's edit to `(2, 7)` printed `Edited draft 1: UNSAVED.` and reviewed as `State: UNSAVED` and `PROVISIONAL — NOT POSTED`. Ending input without Save reported that the draft was not saved; a new launch reopened `(2, 3)`. Repeating the edit and issuing `save` made `(2, 7)` survive a new launch with the same ID and `State: SAVED`. B still reopened `(4, 5)`. Whole-draft readbacks matched the expected customer ID/name, bill date, and ordered lines; only the selected quantity changed. The focused run also verified that `post` remains unsupported.

A separate direct CLI smoke used one fresh root and the same draft ID for both selectors: after A edited and saved, fresh launches showed A at quantity 7 (₹826.00 preview total) and B unchanged at quantity 5 (₹590.00). A's review before Save showed `UNSAVED`; both final reviews showed `PROVISIONAL — NOT POSTED`.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --bin alm-draft-demo --test draft_demo --test account_switch_restart -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [R56-CLI-EDIT-focused](../phase2-local/target/R56-CLI-EDIT-focused.log): **6 passed, 0 failed, 1 ignored** at top level. The ignored entry is the dispatched account-switch worker.
- [R56-CLI-EDIT-offline](../phase2-local/target/R56-CLI-EDIT-offline.log): **539 passed, 0 failed, 3 ignored** at top level. The ignored entries are the existing confirmation restart, SQLite restart, and account-switch workers. Formatting, all-target Clippy with warnings denied, and the locked-offline build passed. Editor diagnostics timed out.

This is local provisional storage evidence under trusted synthetic selectors, not authentication, hosted authorization, or installed-device isolation. The frozen HTML stayed unserved, and formal gates are unchanged.

### B1-QTY-RECONNECT: Persisted quantity change invalidates Owner review

**Development, PASS (R-6/R-7).** The earlier reconnect replay covered a price change, and R56-CLI-EDIT covered explicit save and restart without an Owner review. This [test-only handoff](../phase2-local/tests/owner_simulation/quantity_reconnect.rs), run in the worktree based on `76dd3ae`, connects the missing steps without changing the CLI or posting implementation.

The scoped Manager SQLite store supplied draft ID `1` to the separate in-memory Owner simulation. The retained Owner review was for 3 units at 11,800 paise each (35,400 total). The Manager-side edit explicitly saved 4 units under the same ID, and a fresh SQLite handle reopened the revised draft, preserving customer, bill date, and crate state. Manager reconnect returned `AccessDenied`; Owner reconnect registered the reopened edit without posting. Confirming the retained review returned `StaleReview`. Stock remained at 7 units valued at 47,000 paise; there was no invoice, payment, or ledger entry, and the sales, received and customer-due totals were unchanged at zero. Sales History and Cash Book readbacks were unchanged too.

At the same unit price, the fresh Owner review showed 4 units and 47,200 paise total. Manager confirmation returned `AccessDenied` with no side effects. Explicit Owner confirmation, using the request ID from the rejected attempt, posted invoice #1 for 4 units with 20,000 paise Cash paid on the bill and 27,200 due. FIFO stock fell to 3 units worth 21,000 paise. Invoice, Sales History, Cash Book, and customer ledger readbacks matched; posting did not change the revised SQLite draft.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test owner_simulation -- --nocapture
```

[B1-QTY-RECONNECT-focused](../phase2-local/target/B1-QTY-RECONNECT-focused.log): **2 passed, 0 failed, 0 ignored**, including the existing Owner response-loss control. This run tested the worktree based on `76dd3ae`, before the reconnect test and its registration were committed. Editor diagnostics timed out.

[B1-QTY-RECONNECT-b8cfe8f-offline](../phase2-local/target/B1-QTY-RECONNECT-b8cfe8f-offline.log): **Development, PASS.** The full locked-offline suite tested a clean snapshot extracted with `git archive b8cfe8f`, excluding unrelated worktree changes. It recorded **540 passed, 0 failed, 3 ignored** across 48 result summaries, summed at the top level; the reconnect test passed. The ignored entries were the confirmation restart, account-switch, and SQLite restart subprocess workers. Command used for the saved run:

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/target/B1-QTY-RECONNECT-b8cfe8f.dizpoS/phase2-local/Cargo.toml --locked --offline
```

No separate CLI replay was run. Both results are development evidence only: SQLite stores drafts, and the posting effects are synthetic in-memory effects, not hosted authorization or a formal phase exit. The frozen HTML stayed unserved.

### R56-CRATE-REVIEW: Read-only Manager CLI crate contents

**Development, PASS (R-5/R-6; L10-01 structural readback only).** Both runs tested the crate-review changes in the worktree based on `79f2fa9`, not the base commit alone. The reviewed implementation and regression were subsequently committed unchanged as `9947cfc`. The [fresh-process regression](../phase2-local/tests/account_switch_restart/crate_review.rs) saved the existing synthetic `B_SAVED` fixture through the scoped SQLite interface, then launched the actual Manager CLI with that selector and ran `open`, `21`, `review`, `quit`.

The review matched the stored draft: `PROVISIONAL — NOT POSTED`, `State: SAVED`, ordinary line quantities 5 then 2, crate quantity 3, and components in stored order: product ID 9, quantity per crate 6, `Piece`; then product ID 7, quantity per crate 2, `Case`. The complete review contained no price, tax, partial total, or cost output and ended with `Bill preview unavailable: UnsupportedCrateRow`. The read-only SQLite snapshot and the full draft reopened after CLI exit both matched their pre-review values. Pricing and the OPEN-010 / DEC-027 valuation deferral were not changed.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --bin alm-draft-demo --test account_switch_restart --test draft_demo -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [R56-CRATE-REVIEW-focused](../phase2-local/target/R56-CRATE-REVIEW-focused.log): **10 passed, 0 failed, 1 ignored** at top level; the ignored entry is the dispatched account-switch worker.
- [R56-CRATE-REVIEW-offline](../phase2-local/target/R56-CRATE-REVIEW-offline.log): **541 passed, 0 failed, 3 ignored** at top level; the ignored entries are the existing confirmation restart, account-switch, and SQLite restart workers. Formatting, all-target Clippy with warnings denied, and the locked-offline build passed. Editor diagnostics timed out.

This checks local review under a trusted synthetic selector only. The frozen HTML stayed unserved; no hosted authorization, installed-device isolation, or formal phase exit is established.

### R56-LINE-REVIEW: Edit numbers in the unavailable Manager preview

**Development, PASS (R-5/R-6, local CLI only).** Both the focused and locked-offline runs tested the line-review changes in the worktree based on `bb12e56`, not the base commit alone. The tested renderer and regression were subsequently committed unchanged as `55a13b4`. Before the renderer change, both scoped assertions failed because the ordinary rows displayed `Unrecognized item` instead of their edit numbers and stored product IDs ([red run](../phase2-local/target/R56-LINE-REVIEW-red.log)).

On the existing scoped synthetic `B_SAVED` draft, the fresh-process review displayed `Line 1: Product ID 7 | Quantity: 5` and `Line 2: Product ID 9 | Quantity: 2` in saved order. The full transcript had no invented ordinary product names or monetary values, and the read-only SQLite snapshot and reopened draft were unchanged after review alone. The edit regression took the displayed number for product 9, entered it in `edit`, and explicitly saved quantity 8. Reopening matched the full expected draft with only product 9's quantity changed; product 7 stayed at 5.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --bin alm-draft-demo --test account_switch_restart --test draft_demo -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/R56-LINE-REVIEW-focused.log): **8 passed, 0 failed, 1 ignored** at top level. The account-switch test dispatched its ignored worker separately; all three child invocations passed.
- [Locked offline](../phase2-local/target/R56-LINE-REVIEW-offline.log): **542 passed, 0 failed, 3 ignored** at top level (the existing confirmation restart, account-switch, and SQLite restart workers). Formatting and locked-offline all-target Clippy with warnings denied passed; editor diagnostics timed out.

The closed crate check remains closed. Pricing, valuation, OPEN-010, and formal phase exits are unchanged. No frozen browser, hosted service, or installed-device acceptance check ran.

### R56-PRICED-LINES: Edit numbers in the priced Manager preview

**Development, PASS (R-5/R-6, local CLI only).** Both the focused and locked-offline runs tested the modified worktree based on `8924cbb`, not the base commit alone. The tested renderer and regression were subsequently committed unchanged as `194324c`. The existing [scoped CLI regression](../phase2-local/tests/draft_demo/scoped_edit.rs) first failed on the missing priced line number ([red run](../phase2-local/target/R56-PRICED-LINES-red.log)).

The two ordinary lines for Synthetic Lime displayed edit numbers 1 and 2 with initial quantities 2 and 3. The test fed the displayed second-line number back to `edit`: only line 2 changed to 7, and review showed `UNSAVED` and `PROVISIONAL — NOT POSTED`. Reopening after EOF without Save restored quantities 2 and 3; reopening after an explicit Save showed 2 and 7, still provisional. The independent scoped account stayed at quantities 4 and 5. Fixed amount assertions confirmed a unit price of ₹118.00 and an initial line total of ₹236.00 then ₹354.00, with taxable ₹500.00, CGST and SGST ₹45.00 each, and inclusive total ₹590.00. After the quantity edit, the line totals were ₹236.00 and ₹826.00, with taxable ₹900.00, CGST and SGST ₹81.00 each, and inclusive total ₹1062.00. Only the line label changed in production code.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --bin alm-draft-demo --test draft_demo -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/R56-PRICED-LINES-focused.log): **5 passed, 0 failed, 0 ignored**.
- [Locked offline](../phase2-local/target/R56-PRICED-LINES-offline.log): **542 passed, 0 failed, 3 ignored** at top level (the existing confirmation restart, account-switch, and SQLite restart workers). Formatting, all-target Clippy with warnings denied, and the locked-offline CLI build passed; editor diagnostics timed out.

The unavailable-preview and crate checks remain closed. No pricing, tax, valuation, OPEN-010, or formal phase exit changed.

### L19-LATE-RESTORE: Recover the current copy after a partial folder backup

**Development, PASS (L19-02/L19-08, synthetic, local Mac).** Both the focused and locked-offline runs tested the modified worktree based on `47078ec`, not the base commit alone. The tested regression was subsequently committed unchanged as `5d04242`. The earlier [D-FOLDER-d1ea4e5 run](#folder-backup-scheduling-and-reconnect) covers Connect reporting “Backup folder set” despite a late dated-publication failure, followed by successful retry. The [new regression](../phase2-local/tests/folder_backup_replay/connection.rs) adds the missing manual Save-to-restore check before retry; no production code changed.

Manual Save reported “Could not write.” The trace showed current close before dated staging and injected publication failure. `ALM_data.json` on disk contained the new synthetic archive payload; `ALM_backup_2024-05-01.json` retained its distinct old bytes, and no dated stage remained. The modeled last backup timestamp stayed at 7 and the daily marker remained unset. Using the payload extracted from the actual current file, typed restore to a fresh path succeeded and reopened with the expected records (`reorder_default: 23`). Restore to the occupied dated path instead received an actual local `AlreadyExists` error; both backup files stayed byte-for-byte unchanged by the recovery attempts.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test folder_backup_replay --test clear_restore_replay -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/L19-LATE-RESTORE-focused.log): **16 passed, 0 failed, 0 ignored** (14 folder backup and 2 clear/restore).
- [Locked offline](../phase2-local/target/L19-LATE-RESTORE-offline.log): **543 passed, 0 failed, 3 ignored** at top level (the existing confirmation restart, account-switch, and SQLite restart subprocess workers). Formatting passed; Rust editor diagnostics timed out.

The dated-publication error was injected, not an observed filesystem write failure. `AlreadyExists` was observed during restore to an intentionally occupied local path. The folder replay envelope carries the typed archive only for this test; direct legacy-format import, crash durability, browser behavior, and installed-device parity were not verified. The priced-line check remains closed, and OPEN-010 and formal phase exits are unchanged.

### L06-STANDALONE-RECOVERY: Regenerate the same standalone GST bill after restore

**Development, PASS (L06-02, synthetic local Mac).** Both completed runs tested the modified worktree based on `ea39a90`, not the base commit alone. The tested implementation and regression were subsequently committed unchanged as `61bb825`. The earlier standalone rounding replay re-emitted from the live fixture; the structured GST replay used an ordinary sale. The [extended rounding replay](../phase2-local/tests/gst_preview_replay/rounding.rs) now archives the typed record from the existing standalone Save & View action, writes the backup to disk, drops both the source store and live fixture, restores to a fresh path, drops the restore handle, then reopens and emits by the original bill ID. Neither cached markup nor an ordinary sale supplies the regenerated output.

Both isolated cases recovered bill ID 91, invoice 47 dated 2025-01-02, not cancelled, for the original buyer. Full invoice equality checked the saved record and totals; regenerated inert markup matched the original byte for byte. In paise, ordered taxable, tax, CGST, SGST, IGST, total, the fixed and observed arrays both matched `[8898, 1602, 801, 801, 0, 10500]` for one line and `[17797, 3203, 1602, 1602, 0, 21000]` for two lines. The existing standalone projector supplies the rounding. Manager and unknown-ID requests emitted nothing and returned `AccessDenied` and `NotFound`, respectively.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test gst_preview_replay --test milestone_d_structured -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/L06-STANDALONE-RECOVERY-focused.log): **7 passed, 0 failed, 0 ignored**.
- [Locked offline](../phase2-local/target/L06-STANDALONE-RECOVERY-complete.log): **543 passed, 0 failed, 3 ignored** (existing confirmation restart, account-switch, and SQLite restart workers). An earlier invocation hit the shell timeout and was not counted. Formatting passed; editor diagnostics timed out.

This covers typed standalone bill recovery and inert markup only. No browser, actual printing, PDF creation, installed-device parity, or formal phase exit was verified. The folder-backup check remains closed.

### L06-DUPLICATE-ID: Reject duplicate standalone GST IDs on restore

**Development, PASS (L06-02 archive guard, synthetic local Mac).** Both the focused and locked-offline runs tested the modified worktree based on `f77fd4e`, not the base commit alone. The tested [regression](../phase2-local/tests/gst_preview_replay/archive_restore.rs) and its registration were subsequently committed unchanged as `cc9862f`. A valid backup with internal IDs 91 and 92 and displayed invoice numbers 47 and 48 restored successfully to seed the occupied store. Changing only internal ID 92 to 91 made the archive invalid; the displayed invoice numbers remained distinct. This test establishes no rule about displayed invoice numbers.

Restore returned `InvalidArchive` at both destinations. No file was created at the fresh path, and the occupied store was byte-for-byte unchanged. Reopening the occupied store and emitting by the original internal IDs yielded both original standalone records and totals.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test gst_preview_replay duplicate_standalone -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/L06-DUPLICATE-ID-focused.log): **1 passed, 0 failed, 0 ignored** (5 filtered out).
- [Locked offline](../phase2-local/target/L06-DUPLICATE-ID-offline.log): **544 passed, 0 failed, 3 ignored** (existing confirmation restart, account-switch, and SQLite restart workers). Formatting passed; editor diagnostics timed out.

No production code, invoicing, or rounding rule changed. The successful recovery check remains closed; this result does not establish browser or installed-device parity or a formal phase exit.

### L06-PROFILE-RESTORE: Preserve standalone GST bills after an Owner profile save

**Development, PASS (L06-02 / L18-04, synthetic local Mac).** Both the focused and locked-offline runs tested the modified worktree based on `e926fc5`, not the base commit alone. The tested regression was subsequently committed unchanged as `bbcb748`. The [new regression](../phase2-local/tests/gst_preview_replay/archive_restore.rs) captured the full invoices for internal IDs 91 and 92 before saving a changed Owner business profile. It dropped and reopened the same source file before backing it up, restored to a fresh path, then dropped the restore handle and reopened the destination.

The changed profile, including its version, timestamp, and device, matched on both disk reopens. Both original bills were emitted by internal ID from the restored store; their full records and tax totals matched the pre-save invoices.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline --test gst_preview_replay owner_profile_rewrite -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --locked --offline
```

- [Focused](../phase2-local/target/L06-PROFILE-RESTORE-focused.log): **1 passed, 0 failed, 0 ignored** (6 filtered out).
- [Locked offline](../phase2-local/target/L06-PROFILE-RESTORE-offline.log): **545 passed, 0 failed, 3 ignored**. Formatting passed; Rust editor diagnostics timed out.

No production code, numbering, or rounding rule changed. The duplicate-ID check remains closed. This is local development evidence only, not installed-device parity or a formal phase exit.

### B1-SAME-RECORD: CLI-saved named draft through simulated Owner posting and books

**Development, PASS (synthetic local Mac).** Both the focused and locked-offline runs tested the modified worktree based on `4e0966f`, not the base commit alone. The tested replay was subsequently committed unchanged as `72ebd44`. The [Milestone C replay](../phase2-local/tests/milestone_c_replay.rs) now takes its posting input from the [scoped Manager CLI and SQLite handoff](../phase2-local/tests/milestone_c_replay/start.rs). The historical Batch 1 handoff describes its earlier runs at their original scope. No production code changed.

| Checkpoint | Fixed expectations matched by the joined run |
| --- | --- |
| Manager draft and fresh reopen | Seeded a provisional draft for `milestone-c-buyer` (`Synthetic Buyer`), `DraftId(21)`, dated 2025-02-21, product 1 ×1. The real scoped CLI changed the quantity to 2, displayed `UNSAVED` and `PROVISIONAL — NOT POSTED`, and explicitly saved. A fresh CLI process reopened it as `SAVED` and still provisional. The typed SQLite readback matched the expected ID, buyer, date, ordered line and no crate; only that readback went to the Owner simulation. |
| Changed terms and explicit confirmation | Simulated unit price changed from 9,440 to 11,800 paise (review total 18,880 → 23,600). Old approval returned `StaleReview` and Manager review/confirmation were denied with no posting effects. Explicit Owner confirmation applied before the reply was discarded. Identical retry returned the same invoice #1 without another effect; reusing the request with paid changed to 10,001 returned `RequestConflict` without changing readbacks. |
| Feb 21 invoice #1 | Exact total/paid/due: 23,600/10,000/13,600 paise. Feb 21 Cash Book showed 10,000 Cash, FIFO remaining value was 19,000, and ordinary GST taxable/tax was 20,000/3,600 with CGST and SGST 1,800 each and zero IGST. Estimate AsOnBillDate and Latest both displayed ₹136.00 due. |
| Mar 1 independent receipt | Owner's separate Online receipt of 5,000 paise reduced the ledger balance to 8,600 and appeared on Mar 1 only. Manager receipt was denied. Invoice #1 and its original 10,000 paid and 13,600 bill due were unchanged; estimate AsOnBillDate remained ₹136.00, while Latest showed ₹86.00. |
| Edit invoice #1 | Same invoice, buyer and bill date; total/paid/bill-date due/current balance became 41,000/10,000/31,000/26,000 paise. Estimate AsOnBillDate/Latest displayed ₹310.00/₹260.00. GST taxable/tax was 35,000/6,000, with CGST and SGST 3,000 each and zero IGST. FIFO remaining value was 16,500. Sales History, CSV, ledger and both Cash Book dates were checked against the edited bill and the separate receipt. |
| Cancel invoice #1 | Historical edited total, paid and bill-date due stayed 41,000/10,000/31,000 paise; CSV and GST were marked cancelled. Feb 21 live bill Cash fell to zero; the saved manual close kept its ₹100 Cash. Mar 1 still held the 5,000-paise receipt, leaving the ledger at −5,000 paise and estimate Latest at ₹-50.00 versus historical due ₹310.00. FIFO returned to 32,000. The existing undo control restored the edited figures after a Manager undo denial; final SQLite readback still matched the CLI-saved provisional draft. |

The focused run also reused the independent live ledger and [recovered `Latest` rounding controls](../phase2-local/tests/milestone_d_structured/signed_recovery.rs). From reopened typed records, +150 and −150 paise displayed ₹2.00 and ₹-1.00; the sum-before-rounding control `849 − 799` paise displayed ₹1.00. These are separate rounding fixtures, not extra transactions on invoice #1. No rounding rule or fixed expectation changed.

```sh
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_c_replay --test l14_opening_balance_replay --test milestone_d_structured -- --nocapture
cd /Users/apple/ALM && env -u ALM_REPLAY_QUICKLOOK_PNG -u ALM_L07_03_PNG -u ALM_STRUCTURED_RETAIN TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml
```

- [Focused](../phase2-local/target/B1-SAME-RECORD-focused.log): **7 passed, 0 failed, 0 ignored** (joined C 1, L14 4, structured D 2 including its companion PO setup).
- [Locked offline](../phase2-local/target/B1-SAME-RECORD-offline.log): **545 passed, 0 failed, 3 ignored** (existing confirmation restart, account-switch and SQLite restart workers). Formatting and focused Clippy with warnings denied passed; editor diagnostics timed out.

The Manager CLI is provisional-only; posting and reply loss were exercised only in a separate in-memory Owner simulation. No hosted authorization, frozen-browser execution, actual print or PDF output, installed-device acceptance, or formal phase exit is established.

### B2-MANAGER-SQLITE: Scoped draft readback from the Batch 2 inventory replay

**Development, PASS (synthetic local Mac).** Both the focused and locked-offline runs tested the modified worktree based on `756019c`, not the base commit alone. The tested changes were subsequently committed unchanged as `d19f5b9`. The [Batch 2 count checkpoint](../phase2-local/tests/milestone_b_replay/count_adjustment.rs) saved only the provisional draft in a disposable account-scoped SQLite store and reopened it through a fresh handle. The original Batch 2 handoff describes the earlier debug-only check; the account-switch replay's disk evidence was from a different fixture.

The reopened draft matched draft ID `2`, date `2025-02-16`, and ordered lines product `1` ×1 and product `2` ×2, with no customer or crate. The [shared read-only SQLite inspector](../phase2-local/tests/common/manager_sqlite.rs) found exactly one scoped database with the expected account binding, the exact draft-only schema allowlist, and only the expected draft header, date, and line rows. Its logical dump contained none of the checked purchase cost, FIFO, margin, profit, or Owner fixture markers.

The populated Owner fixture remained separate and supplied the existing purchase-cost, FIFO, and monthly profit comparisons; no Owner inventory or financial records were copied to the Manager store. The original valuation assertions were not changed. This inspects the local draft store's logical contents, not hosted authorization, installed-device isolation, or release costing. OPEN-010 and formal phase exits are unchanged.

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test milestone_b_replay --test account_switch_restart -- --nocapture --test-threads=1
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml
```

- [Focused](../phase2-local/target/B2-MANAGER-SQLITE-focused.log): **6 passed, 0 failed, 1 ignored** at top level; the account-switch test dispatched the ignored worker in three child invocations, which passed separately.
- [Locked offline](../phase2-local/target/B2-MANAGER-SQLITE-offline.log): **545 passed, 0 failed, 3 ignored** (existing confirmation restart, account-switch, and SQLite restart workers). Formatting and focused Clippy with warnings denied passed; editor diagnostics timed out.

### LOCAL-CLEAN-eed9f6e: Combined local regression from a committed snapshot

**Development, PASS (local macOS, synthetic fixtures).** No equivalent clean run was recorded. The earlier `b8cfe8f` archive predates these source and test changes, and the Batch 2 full run used a modified worktree. This run extracted `eed9f6e0fe63c86cbeb97d3b507f1133166f0793` with `git archive` into a new scratch directory. No unrelated working-tree files or prebuilt target artifacts were copied. Cargo ran from the extracted root with the existing offline dependency cache, a fresh build directory and a run-local `TMPDIR`; optional Quick Look PNG and artifact-retention overrides were unset.

Run `local-clean-eed9f6e.yifcL3`, started `2026-11-22T20:18:24Z`, Cargo and rustc `1.98.0`. The single `cargo test --locked --offline` invocation used the [archived manifest](../phase2-local/target/local-clean-eed9f6e.yifcL3/snapshot/phase2-local/Cargo.toml). The [saved log](../phase2-local/target/local-clean-eed9f6e.yifcL3/locked-offline.log) recorded exit status **0** and **545 passed, 0 failed, 3 ignored** across 48 top-level summaries. The ignored entries were the confirmation restart, account-switch, and SQLite restart subprocess workers, not additional top-level passes. The joined Batch 1, Batch 2 inventory and Manager SQLite, and structured recovery tests passed at this checkpoint. The frozen HTML remained unserved. This is reproducibility evidence for the committed local suite, not release acceptance or a formal phase exit.

**Consolidated disposition.** The local handoffs stand at their recorded scope. Under [D1](PRD.md#local-delivery-alongside-discovery), the builder can continue [the static record crosswalk](PHASE1_DISCOVERY.md#per-record-source-crosswalk) without assigning source precedence, and can extend the existing monthly Reports fixture for [L16-06 chart inputs](PHASE1_CASES.md#canvas-chart-inputs-and-drawing-not-run): compare the synthetic Sales and Gross Profit series with the P&L inputs at month rollover and after cancellation. Such a check would be local and source-derived, not chart rendering or browser parity. Neither task requires reopening the handoffs or changing a business rule.

The [existing owner checklist](OPEN.md#consolidated-owner-checklist) governs the remaining gates: Phase 1 deployed dependency capture and protected metadata need OPEN-001/002; frozen runtime waits for verified OPEN-008 containment. Hosted roles, sync, and posting need OPEN-004/006; actual device and output parity needs OPEN-005 and the prerequisite discovery and shared-data evidence. Migration rehearsal needs authorized protected copies and reconciled source rules (OPEN-002/003); production cutover follows rehearsal and parity sign-off and needs separate OPEN-007 authorization. OPEN-009 remains a separate unresolved paste-date decision. [OPEN-010](OPEN.md#open-010-repeated-product-crate-valuation-l10-01) remains owner-blocked under DEC-027 pending containment, frozen L10-01 evidence, and a written costing and rounding decision. Neither costing rule is approved. No checklist or phase status changed.

### L16-06-CHART: Reports chart inputs across rollover and cancellation

**Synthetic local development: focused PASS; locked-offline suite FAILED, incomplete.** Both runs tested the modified worktree based on `e7caa31` with the Reports replay subsequently committed as `abaed07`, before the GST sandbox fix. Before execution, the [test-only replay](../phase2-local/src/confirmation/tests/l16_reports_chart_replay.rs) pinned expected values from the frozen [P&L calculation](../ALM_Business_Manager.html#L8756), [chart callback and month keys](../ALM_Business_Manager.html#L8947), and [gross-profit calculation](../ALM_Business_Manager.html#L3029). The callback retains the render-time month keys but rereads active sales when invoked. Its Rust model reads sale records independently of the monthly P&L API; the existing Reports fixture supplies both.

| Checkpoint | Fixed expectation and observed result, raw paise (Sales, Gross Profit) |
| --- | --- |
| Apr 30, 2025, before cancellation | November–April chart inputs and P&L inputs agreed. Nonzero pairs: November (15,025, 10,025), February (1,049, 449), March (6,145, 2,445), April (15,025, 10,025). |
| Apr 30 callback released after the modeled May 1 rollover and cancellation of bill 80 | The callback still used November–April. Only April differed from the already-emitted P&L: the chart input became (0, 0), a delta of (−15,025, −10,025). The table was not recomputed at this checkpoint. |
| Fresh May 1 render after cancellation | Both inputs used December–May and agreed. November dropped out, April stayed (0, 0), and May entered at (0, 0). February and March were unchanged. |

```sh
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --lib l16_ -- --nocapture
cd /Users/apple/ALM && TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml
```

- [Focused](../phase2-local/target/L16-06-CHART-focused.log): **10 passed, 0 failed, 0 ignored** (317 filtered out), including L16-06. Formatting passed; Rust editor diagnostics timed out.
- [Locked offline](../phase2-local/target/L16-06-CHART-offline.log): **358 passed, 1 failed, 2 ignored** across the summaries emitted before Cargo stopped. `archive_restore::owner_profile_rewrite_preserves_standalone_gst_bills_through_backup_and_restore` failed during scratch setup at `tests/gst_preview_replay/artifacts.rs:21` with `AlreadyExists`, before its restore assertions. The ignored entries were the confirmation restart and account-switch workers; targets after the failure were not run. No retry or scratch-helper change was made.

Frozen HTML remained unserved. Browser rendering and actual canvas/timer behavior are **NOT RUN**. No production Reports logic, OPEN-010, or formal phase exit changed.

### GST-SANDBOX-RETRY: Exclusive allocation under normal parallelism

**Development, PASS (local macOS, synthetic fixtures).** The earlier L16-06 locked-offline attempt remains a failure with **358 passed, 1 failed, 2 ignored**; later targets did not run. Its log establishes that directory creation returned `AlreadyExists` before the GST restore assertions, not which process or test created the occupied path.

Both new runs tested the modified worktree based on `e7caa31`. The successful full suite included both the L16-06 replay and the [GST test sandbox fix](../phase2-local/tests/gst_preview_replay/artifacts.rs), subsequently committed as `abaed07` and `4af3888`, respectively. Allocation now uses the process ID and an atomic sequence, claims each candidate with exclusive `create_dir`, and retries only `AlreadyExists` with a different candidate. The regression verified that the occupied candidate's sentinel survived, cleanup removed only the newly claimed directory, and two live sandboxes had distinct paths. The exact `ALM_GST_PREVIEW_ARTIFACTS=1` opt-in and retention branch remain intact. No existing GST or Reports assertions were weakened.

```sh
cd /Users/apple/ALM && env -u RUST_TEST_THREADS -u ALM_GST_PREVIEW_ARTIFACTS TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml --test gst_preview_replay
cd /Users/apple/ALM && env -u RUST_TEST_THREADS -u ALM_GST_PREVIEW_ARTIFACTS TMPDIR=/Users/apple/ALM/phase2-local/target cargo test --locked --offline --manifest-path /Users/apple/ALM/phase2-local/Cargo.toml
```

- [GST-SANDBOX-RETRY-focused](../phase2-local/target/GST-SANDBOX-RETRY-focused.log): **10 passed, 0 failed, 0 ignored**.
- [GST-SANDBOX-RETRY-offline](../phase2-local/target/GST-SANDBOX-RETRY-offline.log): **549 passed, 0 failed, 3 ignored** across 48 top-level summaries. The ignored entries were the existing confirmation restart, account-switch, and SQLite restart subprocess workers. Formatting passed; editor diagnostics timed out.

Frozen HTML stayed unserved; browser rendering remains **NOT RUN**. OPEN-010 and formal phase exits are unchanged.

## Final consolidated regression at 6240541

| Result ID | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FINAL-6240541-offline | development | `cargo test --locked --offline` on `phase2-local`, synthetic fixtures, HEAD `6240541` | All tests pass | **549 passed, 0 failed, 3 ignored** (sum of per-target summaries; the ignored entries are the existing subprocess workers). `cargo fmt --check` exit 0; `cargo clippy --all-targets --locked --offline -- -D warnings` finished with no warnings | Pass; development evidence only, not owner acceptance | [log](../phase2-local/target/FINAL-6240541-offline.log), run id `FINAL-6240541-offline`, commit `6240541` |

## Baseline digest recheck

| Result ID | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| DISC-DEPLOY-RECHECK | development | Local `shasum -a 256` of the untracked `ALM_Business_Manager.html`; no browser, no serving | SHA-256 `cd2887712e8a5a7e2fb9d45299d9034614cb4f9ec640a08ab1670acc42dd0043` | `cd2887712e8a5a7e2fb9d45299d9034614cb4f9ec640a08ab1670acc42dd0043`, **match**. `git ls-files --error-unmatch` fails and `git status --porcelain` shows `??`: the file is untracked | Match; not committed (see [OPEN-011](OPEN.md#open-011-unattributed-working-tree-files)) | Command output at HEAD `40fcbff` |

## D1 embedded SQLite driver and D2 workspace split

Development evidence only, not acceptance. Synthetic fixtures, local macOS, `--locked --offline` after the single authorized `cargo fetch` for `rusqlite =0.43.0` (`bundled`). Counts are sums of the per-target `test result` lines.

| Result ID | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| D1-BASELINE | development | `cargo test --locked --offline --workspace` on `phase2-local`, HEAD `ba376f4`, sqlite3 CLI store | Baseline record | **549 passed, 0 failed, 3 ignored** across 48 summaries (the ignored entries are the confirmation restart, account-switch and SQLite restart subprocess workers) | Baseline | [summary](../phase2-local/target/D1-BASELINE-ba376f4-summary.log), commit `ba376f4` |
| D1-89f87af-offline | development | Same command, commit `89f87af` (rusqlite store, bound parameters, `TransactionBehavior::Immediate`, `foreign_keys = ON`, `synchronous = FULL`, busy timeout 0 as with the CLI) | ≥ baseline, 0 failed | **551 passed, 0 failed, 3 ignored**. The +2 are the new `sqlite::tests::legacy_cli_database_opens_and_round_trips_with_the_embedded_driver` and `sqlite::tests::scoped_cli_database_opens_for_its_bound_account_only`, which build files with the CLI path's frozen DDL and hex-cast rows. `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` exit 0. `grep -rn '"sqlite3"' src tests` returned nothing | Pass | [log](../phase2-local/target/D1-89f87af-offline.log), commit `89f87af` |
| D1-WAL-PROBE | development | Same suite with `journal_mode = WAL` added to the connection setup (not committed) | All restart and recovery tests pass | **FAILED**: `manager_account_tests::case_distinct_accounts_with_same_draft_id_use_independent_databases` found 6 entries under `manager-accounts/` instead of 2 because of the persistent `-wal`/`-shm` sidecars; the lib target stopped at 327 passed, 1 failed | WAL **not enabled**; rollback journal kept | Local probe output, not retained |
| D2-ae31789-offline | development | `cargo test --locked --offline --workspace` on the virtual workspace (`alm-core`, `alm-store`, `alm-sync`, `alm-cli`), commit `ae31789`, crate scratch `target/` directories removed first | ≥ baseline, 0 failed | **551 passed, 0 failed, 3 ignored** across 53 summaries; the same 554 test function names as D1, only module paths changed (moves to `alm-store/tests/confirmation_restart`, case-ID module renames). Each extraction commit (`ceae51c`, `2ca90b0`, `649a540`, `51cf750`) was also 551/0/3 with fmt and clippy clean. `cargo fmt --all --check` exit 0; clippy exit 0. `cargo tree -p alm-core` lists only serde/serde_json and their macro deps; `grep -rn 'std::process\|std::fs' crates/alm-core/src` returned nothing | Pass | [log](../phase2-local/target/D2-ae31789-offline.log), [clippy](../phase2-local/target/D2-ae31789-clippy.log), commit `ae31789` |

## Standalone GST bill parity (G1-G4)

| Result ID | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| GST-PARITY-4c75de4-offline | development | `cargo test --locked --offline --workspace`, synthetic fixtures, commits `7fbba84` (checked transactions, `&mut self` writes), `033e9cc` (G1-G3 core), `4c75de4` (G4 store and archive); crate scratch `target/` directories removed first | > 551 passed, 0 failed, 3 ignored | **565 passed, 0 failed, 3 ignored** across 54 summaries (+9 core tests under `confirmation::tests::standalone_gst::{parity,print}`, +5 in the new `alm-store` target `gst_bill_store`). `cargo fmt --all --check` exit 0; clippy `-D warnings` exit 0; `cargo tree -p alm-core` lists only serde/serde_json; `grep -rn 'std::process\|std::fs' crates/alm-core/src` empty | Pass; parity is **source-derived only**: the HTML runtime remains **NOT RUN** (OPEN-008). Known limitation: GST numbering is per device, so two devices can issue the same number, as the HTML can; shared numbering belongs to the later Supabase/sync work | [log](../phase2-local/target/GST-PARITY-4c75de4-offline.log), [clippy](../phase2-local/target/GST-PARITY-4c75de4-clippy.log) |

## Storage cleanup, Tauri one-screen trial and local Supabase schema

Development evidence only, not acceptance. The HTML runtime remains **NOT RUN** (OPEN-008). Toolchain: Rust 1.98.0, Tauri CLI 2.12.6 with `tauri =2.12.7` / `tauri-build =2.7.6`, OpenJDK 17.0.21.1, Android SDK platform 35, build-tools 35.0.0, NDK 28.2.13676358 (r28c), emulator image `android-35;google_apis;arm64-v8a` on a `pixel_tablet` AVD, Supabase CLI 3.3.0.

| Result ID | Kind | Environment / fixture | Expected | Observed | Disposition | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CLEANUP-b82fdc5-offline | development | `cargo test --locked --offline --workspace` after the WAL (5 s busy timeout), error move, accessor and re-export commits (`7ad32e7`..`b82fdc5`) | ≥ 565 passed, 0 failed | **565 passed, 0 failed, 3 ignored**; fmt and clippy clean; `alm-core` depends only on serde/serde_json | Pass | Local run output |
| TAURI-MACOS-bd5dcb1 | development | `cargo run -p alm-app` on macOS 27.0.1, driven through the accessibility tree | Draft create, quantity edit, save, reopen after restart; Owner review; GST invoice view; print / save as PDF | All passed: draft #1 at quantity 3 reopened SAVED (₹354.00) after a restart; Owner review ₹354.00 and Manager denied; invoice matches the ported template (₹385.00, CGST/SGST 42.50); the native print sheet showed the invoice alone, 1 page, with the PDF menu. `window.print()` is a no-op in Tauri's WKWebView, so printing calls `WebviewWindow::print()` | Pass | Screenshots, not retained |
| TAURI-ANDROID-bd5dcb1 | development | `cargo tauri android build --debug --apk --target aarch64`, installed on the Android 15 tablet emulator | Bundled SQLite cross-compiles; app-data path; the same workflow; print/PDF and share | Built with no rusqlite changes; the database is created under `/data/user/0/in.alm.trial/manager-accounts/` in WAL mode; the draft saved at quantity 4 reopened SAVED (₹472.00) after a force-stop; Owner review and invoice render. The Kotlin `PrintManager` bridge saved `GST-Invoice-25-26-1.pdf` (1 page, 106,129 bytes) to Downloads, and `ACTION_SEND` opened the share sheet. Found and fixed: synchronous Tauri commands ran on the UI thread and caused an **ANR** under SQLite fsync, so commands are now `async`. Open: content runs under the status bar (no safe-area padding) | Pass with one layout finding | Screenshots and pulled PDF, not retained |
| TAURI-WINDOWS | development | `.github/workflows/alm-app-windows.yml` (windows-latest: tests, clippy, release build) | Build passes | **NOT RUN**: the workflow runs only once pushed. Even when green it proves the build, not a launched window | Pending | — |
| SUPABASE-LOCAL | development | `supabase start` with `supabase/config.toml` | Local stack starts | **Blocked**: Docker Desktop reports a missing containerd snapshot for a base layer shared with the in-use `pgvector/pgvector:pg18` image; re-pulling the Supabase images did not help | Needs a Docker Desktop restart or image cleanup (owner decision) | Local `supabase start` output |
| SUPABASE-SQL-PG18 | development | Migration `20261123000000_local_sales_core.sql` plus pgTAP on a throwaway `pgvector/pgvector:pg18` container (Postgres 18, pgTAP from apt, Supabase roles created by hand) | Migration applies; replays pass | Migration applied; `post_sale.test.sql` **17/17** and `gst_numbers.test.sql` **8/8** passed. Concurrency checks: 100 allocations from two concurrent sessions gave 100 distinct GST numbers (1-100); the same request ID posted from two sessions at once returned identical receipts with one sale and one stock movement | Pass on plain Postgres; not yet on the Supabase image | Local run output |

## Local billing screen ports and replay client (partial batch)

| Step | Tested changes / platform | Result and limits |
|---|---|---|
| 0. Supabase restart | macOS, Docker Engine 29.7.2, Supabase CLI 3.3.0 | Restarted Desktop. Both starts failed on containerd snapshot 3203, including the single retry after removing only Supabase images. `supabase test db` got connection refused: 17 sale and 8 GST assertions **NOT RUN**. `supabase stop` completed. The saved running-container list was empty; none needed restarting. |
| 1. Android safe area | `c35005d`, four inset paddings in app.css | Linked and packaged; visual verification **NOT RUN**. |
| 2. Replay client | `83bbad6`, postgres 0.19.14; macOS locked offline workspace: 570 passed, 0 failed, 3 ignored | Four opt-in tests returned early without `ALM_PG_URL`. Live Rust replay, SQLSTATE and GST allocation checks and DB resets **NOT RUN**; local stack unavailable. |
| 3. Billing screens | `8a3d9fa` view split, `7f17c1c` Sales history and explicit simulated posting, `2559906` Receive Payment; 574 passed, 0 failed, 3 ignored | Sales has macOS window capture (`phase2-local/target/batch1-sales-smoke.png`); Receive Payment macOS smoke not verified. Separate receipt after cancellation covered in Rust. Owner posting is a local simulation, not hosted authorization. OPEN-004: Manager access to Sales or receiving payments is not approved. Estimate/udhaar/Cash Book integration and batch 2 **NOT RUN**. |
| 4. Android trial | arm64 debug APK, Android 15 `alm-trial` | Built locked/offline, installed and launched. A persistent Android **System UI** not-responding dialog prevented verification and screenshots of the two new screens. Emulator shut down via `adb emu kill`. |
| 5. Regression evidence | macOS workspace, implementation through `2559906` | fmt and clippy `-D warnings` passed; 574 passed, 0 failed, 3 ignored (baseline 566/0/3; four of the eight additions were database opt-ins without a connection). Windows and frozen HTML runtime **NOT RUN** (OPEN-008). No batch acceptance claimed. |

## Billing corrections and New Bill: live development checks

| Scope | Commit / platform | Observed result |
|---|---|---|
| Replay client | `4c113a0`, disposable PostgreSQL 17.12 on Linux ARM64, loopback only | PostgreSQL is a dev-dependency; `alm-sync` has no normal dependency edges. Four Rust replays executed with `ALM_PG_URL`; pgTAP `post_sale` 17/17 and `gst_numbers` 8/8 passed. This is **not** the Supabase CLI stack, a REST implementation, or hosted authorization. Shared `pgvector` image/volumes were not removed; unrelated filingqa remained stopped. |
| Payment entry and review | `41eab02`, `91f67b4`, macOS | Both payment forms take exact decimal rupees, including `.5`. Zero is permitted for bill payment, not a standalone receipt. Core validates the review serial and customer identity. Actual JS handler tests passed 4/4 for IDs above 2^53 through `u64::MAX`; Rust rejects malformed IPC IDs. |
| Android system bars | `c4b962d`, windowed Android 15 tablet, host GPU, cold boot | Sales and Receive Payment headings moved from y=20–60 to 68–108; footer from y=1542–1574 to 1478–1510. Both now clear the status bar (ends at 48) and navigation bar (starts at 1536); keyboard input and dismissal checked. [Measurements](../phase2-local/target/alm-android-windowed-after-bounds.txt). |
| New Bill | `0b9df46`, macOS and Android | Selected Synthetic Market and Synthetic Soda by fixture IDs; one packet saved/reopened at ₹59.00. macOS exercised explicit local Owner confirmation and Manager denial; Android retained the provisional draft after app restart, without posting. [macOS](../phase2-local/target/alm-new-bill-reopened.png), [Android](../phase2-local/target/new-bill-android-reopened.png). Native pickers replace autocomplete. Crates, rate edits, notes, MRP/pack columns, delivery charges and balance shortcuts are not ported. Payment remains at Owner review. |
| Regression / remaining work | Through `0b9df46`, locked offline workspace on macOS **with a live database** | **586 passed, 0 failed, 3 ignored** (574/0/3 previously); all four PostgreSQL tests exercised the migrated DB in this run. fmt and clippy clean. [Workspace log](../phase2-local/target/new-bill-workspace-live.log). Counter, Customers, Cash Book/expenses, dashboard, full batch-1 reconciliation and batch 2 remain outstanding. OPEN-004 unchanged; Manager provisional-only. Frozen HTML runtime **NOT RUN** (OPEN-008); Windows **NOT RUN**. |

## Review fixes, batch 1 book screens, batch 1 integrated check and partial batch 2

Development evidence only, synthetic fixture (`alm-core` feature `demo-fixture`), Owner posting a local simulation, Manager provisional-only. Every commit passed `cargo fmt --check`, `cargo clippy --locked --offline --workspace --all-targets -D warnings` and `cargo test --locked --offline --workspace` (no live database: the four PostgreSQL replays return early). Baseline **586/0/3**; final **604 passed, 0 failed, 3 ignored**. JS transport tests 4/4 (`node --test tests/glue-transport.test.cjs`).

| Step | Commits / tests | Platforms | Result, unported sub-actions and limits |
|---|---|---|---|
| 1a. Review arithmetic | `edfe7dd`; 588/0/3 | macOS (Owner review in smoke) | Review lines/total come from core `owner_billing::priced_draft`; test ties rendered rupees to core and to `Review::amounts`. |
| 1b. Rupee parser | `55b2e4f`; 588/0/3 | — | `Money::parse_rupees` (+ `RupeesError`), boundary tests moved. Manual cash book `from_rupees` kept: it converts rounded whole-rupee i64 system figures, not typed text. |
| 1c. trial_billing split | `6657ffa`; 588/0/3 | macOS, Android (device date shown) | Rules in `confirmation::owner_billing` (+ `BillingDirectory`); seed in `confirmation::demo_fixture` behind `demo-fixture` (alm-app only). `DAY` removed: `start` command carries the device's local date; tests use 2025-04-30. Receive Payment now picks a directory customer. |
| 1d. README | `032571e` | — | Platform notes: safe-area/print/share live in `gen/android/.../MainActivity.kt`; reapply after `tauri android init`. |
| 2. Counter | `1f1d995`; 590/0/3 | macOS smoke (AX), Android | Core `counter_recents` (Cash/Online/Credit tag, no amounts). Unported: greeting by hour, Rate List action, Stock/PO tiles, clickable low-stock/open-PO flags, bill time, invoice reopen (links to Sales), dDisp dates. |
| 2. Customers | `8dec556`; 595/0/3 | macOS smoke, Android | Core `customer_book` (L14 closing balances, receivable/advances, Counter debtor rule, filter, `BalanceSide`), opening balance via `save_customer_opening`. Unported: add/edit/delete customer, WhatsApp remind, mobile search, prefilled New Bill, invoice link, negative opening entry, mobile/address columns. Departure: Only Due uses the rounded-rupee debtor rule. |
| 2. Cash Book | `28f9d8a`, `423a91f`; 597→598/0/3 | macOS smoke, Android | Month KPIs, rows, inline Add/Edit with L15 day close, differences, Use system figures, Check, confirmed Delete. Core `ManualCashBook::rows/find`, `ManualCashBookId::value`, `Money::to_rupee_text`. In memory only. Unported: goods/misc note text, live recompute on typing (Check button), modal. Smoke fixed "+₹20" → "₹20". |
| 2. Dashboard | `05a2024`; 598/0/3 | macOS smoke (Today/7 days/This month), Android | Hero, Profit (sales_history window), Collected, Receivables, payment mix, top debtors (core `top_debtors`), ageing bands. Unported: delta chips, average/collection-rate/margin %, Packets Sold, 14-day chart, donut, top products/customers, bar widths and drill-downs, breakdown modals, WhatsApp, onboarding card. |
| 3. Batch 1 integrated check | `f6b3454`; 599/0/3 | Rust (`bill_entry::batch1`) | Draft saved/reopened after restart (0 posted) → Owner review ₹241.00 → Manager denied → confirm ₹50 cash, lost-response retry returns identical receipt → Sales → 2025-05-02 ₹30.50 online receipt → balance 16,050 paise (₹161) → Cash Book closed for both dates → cancel: receipt survives (-3,050 paise, ₹31 adv), 04-30 close empties, saved entry shows +₹50 → Dashboard prior/current windows → Counter → Manager denied on all book screens. Not covered: estimate `Latest` balance, edited-bill flow, conflicting request reuse. |
| 4. Batch 2 (partial) | `a9d7484` Reorder, `2daed25` Stock, `af92847` Godown; 600→604/0/3 | macOS smoke each; Android | Reorder from `open_reorder_planner`/`copy_reorder_list`; Stock from new core `stock_list` + `open_stock_lots`; Godown from new core `manual_godown_list`. Manager tests assert no ₹/cost/FIFO value. Unported: clipboard write, New PO buttons, Purchased/Sold/Fridge columns, Adjust and Adjust Log, lot source/bought qty, Movement Register, godown Copy List/edit/remove/Updated. **NOT DONE:** products, suppliers, order, purchases, crates, ratelist. OPEN-009/OPEN-010 untouched. |
| 5. Android | Debug APK `-- --locked --offline`, aarch64; AVD `alm-trial` (Android 15 tablet), windowed, `-gpu host -no-snapshot` | Android | Installed and launched twice (before and after Godown); Counter, Cash Book, Customers, Stock, Reorder, Dashboard, Godown each opened by tap and captured with `adb exec-out screencap` (`phase2-local/target/android-batch/`). No ANR. Only empty fixture states were viewed; no Android posting. Emulator stopped with `adb emu kill`. |
| macOS evidence | `cargo run -p alm-app`, AX tree + AXPress, window-only captures in `phase2-local/target/batch1-smoke/` | macOS | Fresh draft → Owner review ₹120.50 → confirmed ₹20 cash on 2026-11-23 → Counter/Customers/Cash Book/Dashboard values checked. Customers/Cash Book smokes were blocked while the login session was locked and run after unlock. |
| NOT RUN | — | — | Windows; Supabase CLI stack; hosted REST; frozen HTML runtime (OPEN-008); live PostgreSQL replays in this pass. |

OPEN-004 questions recorded, not decided (core currently denies the Manager on each): Counter (HTML shows it to all, no amounts); Customers balances and ledger; Cash Book; Dashboard; Reorder (HTML shows costs to all); Stock quantities without values; manual Godown list; Receive Payment and Sales (unchanged).

## Batch 1 completion, GST Sales, batch 2 screens and check, Reports

Development evidence only on the synthetic fixture (`demo-fixture`; batch 2 screens also use the app-only `seed_purchasing` quotes). Owner posting is a local simulation; Manager stays provisional-only and is denied every new screen. Every commit passed `cargo fmt --check`, clippy `-D warnings` and `cargo test --locked --offline --workspace` (PostgreSQL replays returned early without `ALM_PG_URL`). Baseline **604/0/3**; final **626 passed, 0 failed, 3 ignored**. JS transport tests 4/4.

| Step | Commits / tests | Platforms | Result, unported sub-actions, departures |
|---|---|---|---|
| 1. Leftovers | `8a06713`; 607/0/3 | — | Core `parse_cash_book_amount` (blank = 0, as `+input.value \|\| 0`), `CashBookAmounts::ZERO`, `Money::is_zero`, `SalesRow::is_credit`; Sales balance uses `BalanceSide` (no negation in alm-app). |
| 2. Batch 1 check, second half | `2bade42`; 608/0/3 | Rust (`bill_entry::batch1_edit`) | Same request ID with different terms → `RequestConflict`, no effects; Manager denied `begin_edit`; Owner edit (milestone C shape) → 42,850 / due 37,849, stock 48/37, Sales ₹429/₹378, 04-30 close unchanged; ₹30.99 receipt: AsOnBillDate 42,850/5,001/37,849, Latest = ledger 34,750 summed then rounded once → 34,800 (₹348, signed_recovery aggregate-first); second edit refused. **Departure:** core edits append a line rather than change a line's quantity. |
| 3. GST Sales | `9e51d07`; 611/0/3 | macOS smoke, Android | List (newest created first, cancelled dimmed), all G3 fields, item lines, Fill from items, Cancel, Delete, View/print/PDF/share via the existing invoice view; save/cancel/delete persist through `GstBillDatabase`. Core `GstBillFields/GstLineFields`, `gst_bill_list`, `next_gst_bill_id`, `gst_line_input`, `parse_gst_percent`, `fill_gst_lines`, `gst_line_amount`. Tests: numbering, cancel keeps number, deletion marker survives restart, `without_gst_bills` unchanged, Manager denied. Unported: customer/item autocomplete, live GSTIN/e-way warnings and state inference, live totals. |
| 4. Products | `bf5235c`; 614/0/3 | macOS smoke, Android | Core `product_list` (margin % as toFixed(1)), `next_product_id`, `ProductEntry::apply_text`. Unported: Compare from row, pack/MRP/avg-cost, GST/cess/HSN overrides, delete. **Departure:** buy and sell submit together; typed sell wins over live "buy keeps margin". |
| 4. Suppliers | `d4ecf65`; 616/0/3 | macOS smoke, Android | Core `supplier_book`, `parse_quote`, `PurchaseSupplierId::index`; comparison matrix with highlight and best; demo `seed_purchasing`. Unported: mobile/note, edit/delete supplier, WhatsApp. |
| 4. Order Planner | `b1c5ae3`; 617/0/3 | macOS smoke, Android | Planner rows, Low/out, cart, checkout draft, sheet paste with warnings, save via `save_order_plan` (no stock). OPEN-009 unchanged. Unported: Copy Plan, Add All by brand, search, PO line edits, PO print/WhatsApp. |
| 4. Purchase Orders | `2d31058`; 619/0/3 | macOS smoke (run later), Android | Core `purchase_order_list`, `ReceiveFormRow::line_from_text`, `PurchaseOrderId::index`; dated receipts, close remaining. Unported: New PO outside planner, edit/cancel PO, PO outputs. Close-remaining disclosure not reachable via macOS AX (Rust-tested). |
| 4. Crates | `26ab8f5`; 621/0/3 | macOS smoke (run later), Android | Core `crate_template_list`, `crate_component_from_text`, `CrateTemplateId::value`. Unported: Piece MRP entry, loading a crate into New Bill (crate checkout exercised in step 5). OPEN-010 unchanged. |
| 4. Rate List | `adccfd0`; 623/0/3 | macOS smoke, Android | New core `confirmation::rate_list` ported from the L17 replay model (first-seen ticked, filters, group toggle, WhatsApp text/link). Unported: print/PDF, opening WhatsApp, pack/MRP columns. |
| 5. Batch 2 integrated check | `439b382`; 624/0/3 | Rust (`alm-app::batch2`) | Quote ₹37→₹35 makes Wholesale best; plan → PO-001, no stock; receipts 05-01 (Lime 4@69, Soda 3@35) and 05-03 (Soda 4@37.50); close remainder → Completed 16/11/0; 2 crates on 05-04 draw FIFO (Soda 3@37.50 = 11,250; Lime 2@69 = 13,800), movement register 4/0/2/2 and 7/0/4/3; count to 2 Soda → 7,500; reorder Lime, Soda; godown 9 cases separate from stock 2; Stock ₹213; Manager denied on all nine batch 2 screens with no ₹/rates/supplier names. |
| 6. Reports (batch 3 start) | `fbcf3d3`; 626/0/3 | macOS smoke, Android | P&L by month and dead stock with core `dead_stock_cutoff` (today − 30). **Departure:** Misc and Salary columns instead of a summed "Other Expenses". Unported: Sales CSV, charts, supplier-wise/outstanding/customer cards. **Settings NOT DONE** this round. |
| 7. Android | Debug APK `-- --locked --offline` aarch64; AVD `alm-trial`, windowed, `-gpu host -no-snapshot` | Android 15 tablet | Opened GST Sales (seed bill ₹385), Products, Suppliers, Order Planner, Purchase Orders (PO-001 created and received in-app: Completed 1/1 ₹36), Crates (Party Crate saved via native select), Godown (12 cases saved), Rate List (text), Reports (dead stock), Stock (₹5,056); captures in `phase2-local/target/android-batch2/`; no ANR; `adb emu kill`. |
| macOS evidence | AX tree + AXPress, window-only captures in `phase2-local/target/batch1-smoke/` | macOS | The session locked during Purchase Orders and Crates; both were smoked after unlock (PO received 4/10, stock 44; Party Crate ₹400 / ₹241). |
| NOT RUN | — | — | Windows; Supabase CLI stack; hosted REST; live PostgreSQL replays; frozen HTML runtime (OPEN-008); Settings, backup/restore and export UI. |

OPEN-004 questions recorded, not decided (core denies the Manager on each): GST Sales; Products (HTML shows buy rates and margins to all); Suppliers names without rates; Order Planner; Purchase Orders and receiving; Crates; Rate List (sell rates only); Reports.

## OPEN-013 fixes, generic glue, bill image, Settings, backup/restore, exports and batch 3 check

Development evidence only, synthetic fixture. Owner posting is a local simulation; Manager stays provisional-only. Every commit passed `cargo fmt --check`, clippy `-D warnings` and `cargo test --locked --offline --workspace` (PostgreSQL replays returned early without `ALM_PG_URL`). Baseline **626/0/3**; final **651 passed, 0 failed, 3 ignored**. JS glue tests **9/9** (`node --test tests/glue-*.test.cjs`). New dependencies (pre-approved, one `cargo fetch`, exact workspace pins): `tiny-skia =0.12.0`, `ab_glyph =0.2.32` (lock adds ab_glyph_rasterizer, owned_ttf_parser, ttf-parser, arrayref, arrayvec, strict-num, tiny-skia-path); bundled font Noto Sans Regular with `assets/fonts/OFL.txt`.

| Step | Commits / tests | Platforms | Result, unported sub-actions, departures |
|---|---|---|---|
| 1a. In-place bill edit | `e30d689`; 630/0/3 | Rust; Android (Edit Bill modal opened, save not completed) | Core `edit_posted_lines`: each line's qty and rate replaced in place; stock/lots follow (extra oldest lot first, fewer back to the line's newest lots), balance, ledger and Sales follow, paid / Cash Book unchanged, frozen unit cost (`Math.round(cost/qty,2)`) × new qty; repeat edits allowed. Batch 1 check extended (2 → 3 at ₹118: stock 47, 35,400, due 30,399). Append-only core edit kept. |
| 1b. Reports Other Expenses | `d14e643`; 631/0/3 | Rust | Core `MonthlyProfit::other_expenses` (misc + salary); one column, Goods "—" when zero. |
| 1c. Products margin preview | `d360914`; 633/0/3 | Rust, JS | Core `ProductEntry::preview_prices` (buy/margin → sell, sell → margin; negative margin allowed). Live via `product_preview`. Android: not exercised (input focus lost). |
| 1d. Display dates | `3eb971a`; 635/0/3 | Rust; Android (24-Nov-2026 shown) | Core `BillDate::display` (dDisp), `month_label` (monthName "Apr 25"), `first_of_month`; every screen's dates use them; inputs stay ISO. |
| 2. Generic glue | `2d95cf3`, `2f03c7d`; 640/0/3 | Rust, JS 9/9, Android | Modal template (customer ledger, Cash Book entry, stock lots, Edit Bill), `data-live` (Cash Book difference, New Bill quantities, Products margin), `data-search` (New Bill customer/item, Receive Payment with balances, GST item lines, Godown item), `data-copy` (Reorder, Rate List, Godown). Core `search_customers/search_products`, `product_suggestions`, `manual_godown_copy_text`. Android found and fixed: clipped item list, click-race on pick (now `pointerdown`), list not closing on blur. Android limit: New Bill item suggestions open under the on-screen keyboard until it is dismissed. |
| 3. Bill image (R-3) | `73f692d`; 644/0/3 | Rust; Android (share sheet "Sharing image" via FileProvider) | Core `bill_image` / `bill_image_layout` display list (billImageCanvas L4490–4760); golden list; PNG 1520×854, deterministic bytes, Manager denied. Desktop saves to app-data/outputs and reveals; Android `AlmAndroid.shareFile` ACTION_SEND. Rate List Print/Save PDF (core `rate_list_html`) and WhatsApp (`open_url` / `AlmAndroid.openUrl`, wa.me only). **Departures:** invoiceImagePNG (HTML→PNG) not ported, canvas layout used; pack, MRP-savings box, customer mobile/address blank; bold/italic synthesized from one font. |
| 4. Settings | `e044804`; 647/0/3 | Rust; Android (Dark/Light applied) | Device-local: theme, balance mode, privacy (alm-store Preferences, `SettingScope::DeviceLocal`). Shared: GSTIN, FSSAI, show flags, default reorder in app-data/shop.json archive. Core `archive_records`, `parse_reorder_default`, `BusinessProfile::shows`. Unported: name/tagline/owner/mobile/address/loading fields, Match-device theme, privacy blurring, cloud sync. |
| 4. Backup & Restore | `1ac7dd8`; 649/0/3 | Rust; Android (backup created, restored to `restored/…-restored-1.json`, invalid archive rejected, shop.json SHA-256 unchanged) | Core `ReadModel::backup_with`. Backups in app-data/backups (create-new, listed); restore validates and opens at a fresh path. **Departure:** restore does not replace the live in-memory billing state (no archive→fixture path). Unported: external file import, folder auto-save, mirror, Google Sheet. |
| 4. Exports | `f84505d`; 650/0/3 | Rust | Sales CSV and Excel workbook from the read model over live records; Owner-only; output folder + reveal/share. |
| 5. Batch 3 check | `4a4772e`; 651/0/3 | Rust (`alm-app::batch3`) | Bill image, estimate, GST invoice ALM/25-26/1, PO-001 print and rate list produced with fixed expectations; shared setting + device-local preferences changed; backed up; trial dropped; restored on a fresh device at a fresh path; all five outputs regenerated from the reopened archive alone (core `latest_estimate`, `rate_list_html_from_records`, archived PO capture): **PNG bytes and every HTML equal**; device B keeps its own preferences; Manager denied archive, backup, regeneration and exports; truncated archive rejected without data loss. |
| 6. Android | Debug APK `-- --locked --offline`, AVD `alm-trial`, windowed, `-gpu host -no-snapshot` | Android 15 tablet | Search pick, live totals, draft save, Owner confirm, Sales, Edit modal, bill-image share sheet, customer ledger modal, Cash Book live difference (+₹5) and save, Reorder "Copied ✓", Settings theme, backup, restore, invalid restore. Captures in `phase2-local/target/android-batch3/`. Not exercised on Android: Products live margin, Edit Bill save, GST item search, estimate/PO print views, Rate List print/WhatsApp. `adb emu kill`. |
| NOT RUN | — | — | Windows; Supabase CLI stack; hosted REST; live PostgreSQL replays; real WhatsApp handoff (only the share/intent sheet); frozen HTML runtime (OPEN-008); **macOS smokes for this round** (login session locked throughout). |

**Closed from OPEN-013:** inline modals; ISO dates; Cash Book difference on Check; Reorder Copy box; bill edit adds a line; Products typed sell wins over buy-keeps-margin; Reports split Misc/Salary; native pickers / "Fill from items" (New Bill, Receive Payment, GST items, Godown); Rate List print/PDF/WhatsApp.

**Still open:** Customers "Only Due" debtor rule; negative opening balances; Dashboard deltas/charts/top lists; Purchase Orders "Close remaining" not pressed in any smoke; GST customer autocomplete; bill image uses the canvas layout (no HTML-to-PNG) with blank pack/MRP/contact; restore doesn't replace live billing state; Settings profile fields beyond GSTIN/FSSAI; New Bill item suggestions under the Android keyboard.

**New OPEN-004 questions (core denies the Manager on each):** Settings (including device-local appearance); Backup & Restore; bill image share; estimate and PO print views.

## Module split, OPEN-013 closures, device data layer and batch 4 check

Development evidence only, synthetic fixture; `LocalAuthority` is a journaled development simulation of the sales authority, not a server. Manager stays provisional-only. Every commit passed `cargo fmt`, clippy `-D warnings` (`--all-targets`) and `cargo test --locked --offline --workspace` (PostgreSQL tests returned early without `ALM_PG_URL`). Baseline **651/0/3**; final **677 passed, 0 failed, 3 ignored**. JS glue **10/10**. No new external dependencies; `Cargo.lock` changed only by the internal edges alm-sync→alm-core and alm-app→alm-sync. Bundled `NotoSans-Bold.ttf` (same 2.015 release as Regular, OFL, `assets/fonts/OFL.txt`).

| Step | Commits / tests | Platforms | Result |
|---|---|---|---|
| 1. Split alm-app | `ca3e303`; 651/0/3 | Rust | lib.rs → trial/render/commands; outputs → export/platform/purchase/regen; gst_print → format/parts; bill_entry/commands; views → nav/widgets. Registration stays in `run()`. Only test files remain over 250 lines. |
| 2a. Only Due | `8a32b12`; 652 | Rust | `CustomerFilter::Due` = raw balance > 0; debtor count/top debtors keep the Counter rule. |
| 2b. Advances | `15b1e55`; 654 | Rust | Core `Money::parse_signed_rupees`, `OpeningBalance::parse_amount`; "-5" now saves a ₹5 advance (the old test asserted rejection; replaced, malformed signs still rejected). |
| 2c. Settings | `6fca920`; 656 | Rust | Name, tagline, owner, mobile, address, GSTIN, FSSAI, show flags, loading ₹/pkt, reorder (shared archive, core `BusinessProfile::with_input`); Match device theme (device-local); Supabase, Sheets, folder, activity log, Clear All shown inert. Bill image header uses the profile. |
| 2d. Bill image | `0d5a500`; 657 | Rust | PACK text, "You saved ₹N on MRP!" box (`mrpSavings`), customer mobile/address; Bold/SemiBold drawn with the bundled bold face. Expected PNG height 854→974 and CSV Pack "600 mlCrate" (HTML concatenation) follow from the seeded contact/MRP. |
| 2e. Dashboard | `2870530`, `a52b1c5`; 659 | Rust; Android | Core `dashboard_insights`: delta chips, avg, collection rate, margin (Owner), Packets Sold, top products/customers, 14-day line and payment donut geometry; inline SVG only. |
| 2f. GST customer | `a239b04`; 660 | Rust | "Pick customer" via `data-search` → core `fill_gst_customer`. |
| 2g. Keyboard | `4147961`, `a52b1c5`; 660 | JS 10/10; Android | Glue scrolls the box into view and opens the list upward when needed; the fix that worked on the tablet is the platform adapter: `MainActivity.kt` pads the content view by the IME inset so the WebView ends at the keyboard. |
| 3a. SalesAuthority | `d20122f`; 669 | Rust | Trait: submit/status/GST number/pull + post_sale, cancel/restore, edit, receipt, cash day. `LocalAuthority` (journal replay, atomic rollback, lost-response hook). Core role-filtered `projection()` and screen models. Opt-in `postgres_authority` test (post_sale replay/conflict/stale/already-posted, GST idempotency) — NOT RUN. |
| 3b/3c. Outbox, projection | `ace87b6`; 672 | Rust | `Outbox` (pending→unknown on reopen, conflicting reuse refused); `ProjectionStore` per account+role, checkpoint in the same transaction, older batches rejected, rebuild via clear; Manager file refuses owner values; file bytes contain no cost/margin/fifo/profit/buy. |
| 3d/3e. App wiring | `cbd6a55`; 676 | Rust | Sales, Customers, Stock render from the projection with "Posted data as of … UTC · checkpoint"; post/cancel/restore/edit/receipt/Cash Book go through outbox + authority; unknown intents shown with Check status / Retry; reconnect only looks up status; restore validates, replaces shop settings (staged rename) and rebuilds the projection; account switch closes drafts, GST, outbox and projection and clears screen state. |
| 4. Batch 4 check | `c68e652`; 677 | Rust (`alm-app::batch4`) | Manager draft → Owner post with lost response → retry (same request) → restart → sale from device file → receipt → Cash Book close → restart (journal replay) → backup → restore at fresh path → bill PNG byte-identical; Manager projection file has no cost fields. Regeneration is for the Latest balance mode (as batch 3). |
| 5. Smokes | — | Android 15 tablet AVD `alm-trial`, `-gpu host -no-snapshot` | Debug APK `-- --locked --offline`. Item suggestions above the keyboard (after the inset fix), draft, review, post, force-stop + relaunch: sale still shown (checkpoint 1), Dashboard SVG line + donut, Purchase Orders **Close remaining pressed** (PO-001 Completed). Captures `phase2-local/target/android-batch4/`. `adb emu kill`. |
| NOT RUN | — | — | **macOS smoke** (session locked: window capture refused, AX exposed only the menu bar); Windows; Supabase CLI stack; hosted REST; live PostgreSQL (`ALM_PG_URL` unset); real WhatsApp handoff; frozen HTML runtime (OPEN-008). |

**Closed from OPEN-013:** Only Due rule; negative opening balances; Settings fields; bill image pack/MRP/contact and imitated bold; Dashboard deltas/charts/top lists; GST customer autocomplete; Android suggestions under the keyboard; "Close remaining" never pressed (pressed on Android, not macOS); restore no longer read-only.

**Still open:** bill image uses the canvas layout (no HTML-to-PNG), italic still slanted from Regular; archive bill-image regeneration only for the Latest balance; Cash Book screen, ledger and stock-lots modals still read the simulation, not the projection; products, POs, crates, openings, godown and GST saves are not routed through `SalesAuthority` (unjournaled, lost on restart) and GST saves do not use `allocate_gst_number`; loading charge stored but not applied on New Bill; hosted sync, activity log and Clear All inert; Customers list shows no mobile/address.

**New OPEN-004 questions (core denies the Manager on each):** what a Manager projection may hold (now catalog only: no sales, balances or stock); who may check status or retry an unknown intent; whether an unknown intent of one account may be resolved after switching accounts; Manager visibility of freshness/outbox.
