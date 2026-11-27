//! Randomized consistency check: seeded bills mixing item lines, crates (Case and Piece
//! components; several crate rows with no item line among them), loading charges and payments,
//! with edits, cancels, restores and receipts, run through the core's `LocalAuthority` always and through the dev Postgres authority when
//! `ALM_PG_URL` is set. Every step's outcome must match, the effects are compared every
//! `COMPARE_EVERY` steps and at the end, and every posted or edited bill's total must be its
//! rows plus its loading charge (New Bill pricing, the receipt, the projection and the bill
//! preview alike).
//!
//! Replay one case: `ALM_RANDOM_SEED=<seed> ALM_RANDOM_BILLS=<n> cargo test -p alm-sync --test
//! randomized`; a failure prints its seed and step.
#![forbid(unsafe_code)]

mod conformance;
mod differential;
mod pg_authority;

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::{fixture, seed_purchasing};
use alm_core::confirmation::{BalanceMode, ProductTerms};
use alm_core::{DraftId, Money, ProductId, Role};
use alm_sync::{AuthorityError, Intent, LocalAuthority, Outcome, RequestId, SalesAuthority};
use differential::script::{
    Bill, CUSTOMER, For, LIME, MARKET, SODA, crate_terms, mixed_sale, purchase, receipt, receive,
};
use differential::{compare, kind, local_effects, pg_effects};
use pg_authority::PgAuthority;

const SEEDS: &[u64] = &[0x5EED_0001, 0x00C0_FFEE, 0x0BAD_CAFE];
const BILLS_PER_SEED: usize = 1_000;
const COMPARE_EVERY: usize = 40;
/// Soda's pieces come 2 to a packet (checkout terms); Lime has none, so a piece is a packet.
const SODA_PACK: u32 = 2;
const DATES: &[&str] = &["2025-03-01", "2025-03-02", "2025-03-03"];

/// xorshift64*: small, seeded and stable across platforms.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[usize::try_from(self.below(items.len() as u64)).unwrap()]
    }
}

type Components = Vec<(u64, u32, bool)>;

/// A saved crate template: its ID, name, price and components.
struct Template {
    id: u64,
    name: &'static str,
    price: i64,
    components: Components,
}

/// A posted bill as the generator knows it: its lines, crate rows and loading rate.
#[derive(Clone)]
struct Posted {
    lines: Vec<(u64, u32, i64)>,
    crates: Vec<(String, i64, u32, Components)>,
    loading: i64,
}

fn pack(product: u64, piece: bool) -> u32 {
    if piece && product == SODA {
        SODA_PACK
    } else {
        1
    }
}

impl Posted {
    /// Packets for loading, or `None` when a crate's pieces don't make whole packets.
    fn packets(&self) -> Option<u64> {
        let mut packets: u64 = self.lines.iter().map(|l| u64::from(l.1)).sum();
        for (_, _, crates, components) in &self.crates {
            for (product, qty, piece) in components {
                let demand = u64::from(*qty) * u64::from(*crates);
                let pack = u64::from(pack(*product, *piece));
                if demand % pack != 0 {
                    return None;
                }
                packets += demand / pack;
            }
        }
        Some(packets)
    }

    /// Rows plus loading: `Σ qty × rate + Σ crates × rate + packets × loading`.
    fn total(&self) -> Option<i64> {
        let rows: i64 = self.lines.iter().map(|l| i64::from(l.1) * l.2).sum::<i64>()
            + self
                .crates
                .iter()
                .map(|c| i64::from(c.2) * c.1)
                .sum::<i64>();
        Some(rows + i64::try_from(self.packets()?).ok()? * self.loading)
    }
}

struct Run {
    seed: u64,
    step: usize,
    local: LocalAuthority,
    pg: Option<PgAuthority>,
}

impl Run {
    fn fail(&self, what: &str) -> ! {
        panic!(
            "randomized check failed at seed {:#x} step {}: {what}\nreplay: ALM_RANDOM_SEED={} ALM_RANDOM_BILLS={}",
            self.seed,
            self.step,
            self.seed,
            bills()
        )
    }

    /// Submits `intent` to both; the outcomes must match. Returns the core's.
    fn submit(&mut self, intent: &Intent) -> Result<Outcome, AuthorityError> {
        self.step += 1;
        let request =
            RequestId::new(&format!("rnd-{:x}-{}", self.seed, self.step)).expect("request id");
        let core = self.local.submit(Role::Owner, &request, intent);
        if let Some(pg) = &mut self.pg {
            let sql = pg.submit(Role::Owner, &request, intent);
            if kind(&core) != kind(&sql) {
                self.fail(&format!(
                    "outcome differs for {intent:?}\n  core {}\n  sql  {}",
                    kind(&core),
                    kind(&sql)
                ));
            }
        }
        if self.step.is_multiple_of(COMPARE_EVERY) {
            self.compare_effects();
        }
        core
    }

    fn compare_effects(&mut self) {
        if let Some(pg) = &self.pg {
            let differences = compare(&local_effects(&self.local), &pg_effects(pg));
            if !differences.is_empty() {
                self.fail(&format!("effects differ:\n{}", differences.join("\n")));
            }
        }
    }

    fn owner(&self) -> alm_core::projection::ProjectionSnapshot {
        self.local
            .pull(Role::Owner, None)
            .expect("pull")
            .snapshot
            .expect("snapshot")
    }

    /// The step 1 invariant for one bill: the receipt, the projection row and the bill
    /// preview all show its rows plus its loading charge.
    fn check_total(&self, draft: DraftId, bill: &Posted, receipt: Option<Money>) {
        let Some(expected) = bill.total() else {
            self.fail(&format!(
                "bill {draft} posted with pieces that don't make packets"
            ))
        };
        if let Some(total) = receipt
            && total.minor_units() != expected
        {
            self.fail(&format!(
                "receipt total {total:?} for {draft}, rows plus loading {expected}"
            ));
        }
        let preview = self
            .local
            .fixture()
            .bill_preview(Role::Owner, draft, BalanceMode::AsOnBillDate)
            .unwrap_or_else(|e| self.fail(&format!("bill preview {draft}: {e:?}")));
        let rows: i64 = preview.lines.iter().map(|l| l.amount.minor_units()).sum();
        if rows != preview.items_total.minor_units()
            || rows + preview.loading.minor_units() != preview.total.minor_units()
            || preview.total.minor_units() != expected
        {
            self.fail(&format!(
                "bill preview {draft}: rows {rows} + loading {:?} vs items {:?} and total {:?}, expected {expected}",
                preview.loading, preview.items_total, preview.total
            ));
        }
    }
}

fn bills() -> usize {
    std::env::var("ALM_RANDOM_BILLS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(BILLS_PER_SEED)
}

fn setup(seed: u64) -> (Run, Vec<Template>) {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    shop.set_checkout_product(
        ProductId::new(SODA),
        ProductTerms {
            pack: NonZeroU32::new(SODA_PACK).unwrap(),
            piece_price: Money::from_minor_units(3_200),
            case_price: Money::from_minor_units(6_250),
        },
    )
    .unwrap();
    let mut local = LocalAuthority::new(shop, alm_core::confirmation::demo_fixture::directory())
        .with_clock(|| 1_700_000_000_000);
    local.set_caller("diff-device", "diff-owner");
    local.set_role_caller(Role::Manager, "diff-device", "diff-manager");
    let pg = PgAuthority::connect(&local);
    let mut run = Run {
        seed,
        step: 0,
        local,
        pg,
    };
    let po = match run.submit(&purchase(
        "Synthetic Depot",
        "2025-02-01",
        &[
            ("Synthetic Lime", Some(LIME), 1_000_000, 6_500),
            ("Synthetic Soda", Some(SODA), 1_000_000, 3_000),
        ],
    )) {
        Ok(Outcome::Created(id)) => id,
        other => run.fail(&format!("purchase order: {other:?}")),
    };
    let core = run.owner();
    run.submit(&receive(
        &core,
        po,
        Some("2025-02-02"),
        "2025-02-03",
        &[(1_000_000, 0), (1_000_000, 0)],
    ))
    .unwrap_or_else(|e| run.fail(&format!("receive: {e:?}")));
    let specs: [(&'static str, i64, Components); 4] = [
        (
            "Case Crate",
            40_000,
            vec![(LIME, 2, false), (SODA, 3, false)],
        ),
        ("Lime Pieces", 9_000, vec![(LIME, 3, true)]),
        (
            "Soda Pieces",
            15_000,
            vec![(SODA, 4, true), (LIME, 1, false)],
        ),
        ("Odd Soda", 5_000, vec![(SODA, 3, true)]),
    ];
    let mut templates = Vec::new();
    for (name, price, components) in specs {
        let id = match run.submit(&crate_terms(None, name, price, &components)) {
            Ok(Outcome::Created(id)) => id,
            other => run.fail(&format!("template {name}: {other:?}")),
        };
        templates.push(Template {
            id,
            name,
            price,
            components,
        });
    }
    (run, templates)
}

/// Accepted operations of one seed: bills posted, edits, cancels, restores, receipts.
#[derive(Debug, Default)]
struct Counts {
    bills: usize,
    posted: usize,
    /// Posted bills of several crate rows and no item line.
    crate_only: usize,
    edits: usize,
    cancels: usize,
    restores: usize,
    receipts: usize,
    steps: usize,
}

/// One seed: `bills` bills and the edits, cancels, restores and receipts between them.
fn run_seed(seed: u64) -> Counts {
    let (mut run, templates) = setup(seed);
    let mut rng = Rng::new(seed);
    let core = run.owner();
    let sell: BTreeMap<u64, i64> = core
        .products
        .iter()
        .map(|p| (p.id, p.sell.minor_units()))
        .collect();
    let mut posted: Vec<(DraftId, Posted)> = Vec::new();
    let mut next_draft = 10_000;
    let mut n = Counts::default();
    // Only the latest stock movement can be undone (the core's epoch rule), so cancels and
    // restores mostly target the bill moved last.
    let mut last_moved: Option<usize> = None;
    while n.bills < bills() {
        match rng.below(100) {
            0..=64 => {
                n.bills += 1;
                next_draft += 1;
                let draft = DraftId::new(next_draft);
                let customer = *rng.pick(&[CUSTOMER, MARKET]);
                let date = *rng.pick(DATES);
                let lines: Vec<(u64, u32, i64)> = (0..rng.below(4))
                    .map(|_| {
                        let p = *rng.pick(&[LIME, SODA]);
                        (p, 1 + u32::try_from(rng.below(6)).unwrap(), sell[&p])
                    })
                    .collect();
                let crates: Vec<&Template> =
                    (0..rng.below(3)).map(|_| rng.pick(&templates)).collect();
                let loading = if rng.chance(40) {
                    i64::try_from(rng.below(5)).unwrap() * 25
                } else {
                    0
                };
                let bill = Posted {
                    lines: lines.clone(),
                    crates: crates
                        .iter()
                        .map(|t| {
                            (
                                t.name.to_owned(),
                                t.price,
                                1 + u32::try_from(rng.below(3)).unwrap(),
                                t.components.clone(),
                            )
                        })
                        .collect(),
                    loading,
                };
                let total = bill.total().unwrap_or(0);
                let paid = if total > 0 && rng.chance(70) {
                    i64::try_from(rng.below(u64::try_from(total).unwrap() + 1)).unwrap()
                } else {
                    0
                };
                let single = lines.is_empty() && bill.crates.len() == 1 && rng.chance(60);
                let intent = if single {
                    let (_, rate, quantity, components) = &bill.crates[0];
                    let template = templates.iter().find(|t| t.price == *rate).unwrap();
                    Intent::SellCrate(alm_sync::CrateSale {
                        draft,
                        template: template.id,
                        quantity: *quantity,
                        customer: (customer.0.into(), customer.1.into()),
                        date: date.parse().unwrap(),
                        paid: Money::from_minor_units(paid),
                        rate: Money::from_minor_units(*rate),
                        components: components.clone(),
                    })
                } else {
                    let line_refs: Vec<(u64, u32)> = lines.iter().map(|l| (l.0, l.1)).collect();
                    let crate_rows: Vec<_> = bill
                        .crates
                        .iter()
                        .map(|(name, rate, qty, comps)| {
                            (Some(name.as_str()), *rate, *qty, comps.as_slice())
                        })
                        .collect();
                    mixed_sale(
                        &core,
                        &Bill {
                            draft: next_draft,
                            to: For::Customer(customer),
                            lines: &line_refs,
                            date,
                            paid,
                            loading,
                        },
                        &crate_rows,
                    )
                };
                if let Intent::PostSale(sale) = &intent {
                    new_bill_invariant(&run, &sale.draft, &bill);
                }
                if let Ok(Outcome::Sale(receipt)) = run.submit(&intent) {
                    n.posted += 1;
                    if lines.is_empty() && bill.crates.len() > 1 {
                        n.crate_only += 1;
                    }
                    let bill = if single {
                        // The crate post charges the authority's loading rate (0 in this shop).
                        Posted { loading: 0, ..bill }
                    } else {
                        bill
                    };
                    run.check_total(draft, &bill, Some(receipt.total));
                    posted.push((draft, bill));
                    last_moved = Some(posted.len() - 1);
                }
            }
            65..=79 if !posted.is_empty() => {
                let index = usize::try_from(rng.below(posted.len() as u64)).unwrap();
                let (draft, bill) = posted[index].clone();
                let mut edited = bill.clone();
                for line in &mut edited.lines {
                    line.1 = 1 + u32::try_from(rng.below(6)).unwrap();
                    line.2 += i64::try_from(rng.below(3)).unwrap() * 50;
                }
                for row in &mut edited.crates {
                    row.2 = 1 + u32::try_from(rng.below(3)).unwrap();
                    row.1 -= i64::try_from(rng.below(3)).unwrap() * 100;
                }
                let lines = edited
                    .lines
                    .iter()
                    .map(|l| (l.1, l.2))
                    .chain(edited.crates.iter().map(|c| (c.2, c.1)))
                    .collect();
                if run.submit(&Intent::EditSale { draft, lines }).is_ok() {
                    n.edits += 1;
                    run.check_total(draft, &edited, None);
                    posted[index].1 = edited;
                    last_moved = Some(index);
                }
            }
            80..=89 if !posted.is_empty() => {
                let index = match last_moved {
                    Some(index) if rng.chance(80) => index,
                    _ => usize::try_from(rng.below(posted.len() as u64)).unwrap(),
                };
                let (draft, bill) = posted[index].clone();
                let cancel = rng.chance(60);
                let intent = if cancel {
                    Intent::CancelSale { draft }
                } else {
                    Intent::RestoreSale { draft }
                };
                if run.submit(&intent).is_ok() {
                    last_moved = Some(index);
                    if cancel {
                        n.cancels += 1;
                    } else {
                        n.restores += 1;
                    }
                    run.check_total(draft, &bill, None);
                }
            }
            _ => {
                let customer = rng.pick(&[CUSTOMER, MARKET]).0;
                let amount = 100 + i64::try_from(rng.below(5_000)).unwrap();
                let date = rng.pick(DATES);
                let online = rng.chance(50);
                if run.submit(&receipt(customer, date, amount, online)).is_ok() {
                    n.receipts += 1;
                }
            }
        }
    }
    run.compare_effects();
    for (draft, bill) in &posted {
        run.check_total(*draft, bill, None);
    }
    let sales = run.owner().sales;
    for (draft, bill) in &posted {
        let row = sales.iter().find(|s| s.draft_id == *draft).unwrap();
        if Some(row.total.minor_units()) != bill.total() {
            run.fail(&format!("projection total of {draft}: {:?}", row.total));
        }
    }
    n.steps = run.step;
    n
}

/// New Bill's one pricing result for the draft: rows add up to the items total, the loading
/// is the draft's rate per packet and the bill amount is items plus loading.
fn new_bill_invariant(run: &Run, draft: &alm_core::ProvisionalDraft, bill: &Posted) {
    let Ok(pricing) = run
        .local
        .directory()
        .new_bill_pricing(run.local.fixture(), None, draft)
    else {
        if bill.packets().is_some() {
            run.fail(&format!("New Bill could not price {}", draft.id()));
        }
        return;
    };
    let rows: i64 = pricing
        .items
        .lines
        .iter()
        .map(|l| l.amounts.inclusive.minor_units())
        .chain(pricing.crates.iter().map(|c| c.amount.minor_units()))
        .sum();
    let packets = bill
        .packets()
        .unwrap_or_else(|| run.fail("priced odd pieces"));
    if rows != pricing.items_total.minor_units()
        || u64::from(pricing.packets) != packets
        || pricing.loading.minor_units() != i64::try_from(packets).unwrap() * bill.loading
        || pricing.total.minor_units() != rows + pricing.loading.minor_units()
    {
        run.fail(&format!(
            "New Bill pricing of {} does not add up: {pricing:?}",
            draft.id()
        ));
    }
}

#[test]
fn randomized_bills_agree_between_core_and_sql_and_add_up() {
    let seeds: Vec<u64> = match std::env::var("ALM_RANDOM_SEED") {
        Ok(seed) => vec![
            seed.trim_start_matches("0x")
                .parse::<u64>()
                .or_else(|_| u64::from_str_radix(seed.trim_start_matches("0x"), 16))
                .expect("ALM_RANDOM_SEED"),
        ],
        Err(_) => SEEDS.to_vec(),
    };
    for seed in seeds {
        let n = run_seed(seed);
        let sql = if std::env::var_os("ALM_PG_URL").is_some() {
            "core and SQL"
        } else {
            "core only (ALM_PG_URL unset)"
        };
        println!("seed {seed:#x}: {n:?}; {sql}");
    }
}
