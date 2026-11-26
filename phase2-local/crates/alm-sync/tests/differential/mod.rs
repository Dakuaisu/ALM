//! The differential runner: drives the same scripted commands through `LocalAuthority` (the
//! core, the reference) and the dev Postgres authority, and after every step compares the
//! outcome and every effect: the Owner and Manager projection rows (invoice and GST numbers,
//! sales, receipts, Cash Book rows, balances, stock, PO outstanding, catalog), each product's
//! active FIFO lots with their costs, and its fridge count. Differences are collected and
//! reported together at the end of a scenario.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::Debug;

use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::projection::ProjectionSnapshot;
use alm_core::projection::book::OwnerBook;
use alm_core::{ProductId, Role};
use alm_sync::{AuthorityError, Intent, LocalAuthority, Outcome, RequestId, SalesAuthority};

use crate::pg_authority::{Lot, PgAuthority};

pub mod script;

/// What one step leaves behind, read the same way from both authorities.
#[derive(Debug, PartialEq)]
pub struct Effects {
    owner: ProjectionSnapshot,
    manager: ProjectionSnapshot,
    lots: BTreeMap<u64, Vec<Lot>>,
    fridge: BTreeMap<u64, Option<u32>>,
}

/// An outcome with rejections compared by kind (the core and SQL word reasons differently).
pub fn kind(result: &Result<Outcome, AuthorityError>) -> String {
    match result {
        Ok(outcome) => format!("{outcome:?}"),
        Err(AuthorityError::Rejected(_)) => "Rejected".into(),
        Err(error) => format!("{error:?}"),
    }
}

fn snapshot<A: SalesAuthority>(a: &A, role: Role) -> ProjectionSnapshot {
    a.pull(role, None)
        .expect("pull")
        .snapshot
        .expect("a full snapshot")
}

fn products(s: &ProjectionSnapshot) -> Vec<u64> {
    s.products.iter().map(|p| p.id).collect()
}

pub fn local_effects(a: &LocalAuthority) -> Effects {
    let owner = snapshot(a, Role::Owner);
    let ids = products(&owner);
    Effects {
        manager: snapshot(a, Role::Manager),
        lots: ids
            .iter()
            .map(|id| {
                let rows = a
                    .open_stock_lots(Role::Owner, ProductId::new(*id))
                    .map(|l| l.rows)
                    .unwrap_or_default();
                let lots = rows
                    .iter()
                    .map(|r| {
                        (
                            r.date.map(|d| d.to_string()),
                            r.remaining.get(),
                            r.unit_cost.map(alm_core::Money::minor_units),
                        )
                    })
                    .collect();
                (*id, lots)
            })
            .collect(),
        fridge: ids
            .iter()
            .map(|id| {
                let count = a
                    .open_physical_count(Role::Owner, ProductId::new(*id))
                    .ok()
                    .map(|c| c.fridge);
                (*id, count)
            })
            .collect(),
        owner,
    }
}

pub fn pg_effects(a: &PgAuthority) -> Effects {
    let owner = snapshot(a, Role::Owner);
    let ids = products(&owner);
    Effects {
        manager: snapshot(a, Role::Manager),
        lots: ids.iter().map(|id| (*id, a.lots(*id))).collect(),
        fridge: ids.iter().map(|id| (*id, a.fridge(*id))).collect(),
        owner,
    }
}

fn differ<T: PartialEq + Debug>(out: &mut Vec<String>, what: &str, core: &T, sql: &T) {
    if core != sql {
        out.push(format!("  {what}:\n    core {core:?}\n    sql  {sql:?}"));
    }
}

pub fn compare(core: &Effects, sql: &Effects) -> Vec<String> {
    let mut out = Vec::new();
    let (c, s) = (&core.owner, &sql.owner);
    differ(
        &mut out,
        "sales_unsupported",
        &c.sales_unsupported,
        &s.sales_unsupported,
    );
    differ(&mut out, "sales", &c.sales, &s.sales);
    differ(&mut out, "receipts", &c.receipts, &s.receipts);
    differ(&mut out, "customers", &c.customers, &s.customers);
    differ(&mut out, "stock", &c.stock, &s.stock);
    differ(&mut out, "catalog", &c.catalog, &s.catalog);
    differ(&mut out, "cash_days", &c.cash_days, &s.cash_days);
    differ(&mut out, "products", &c.products, &s.products);
    differ(&mut out, "suppliers", &c.suppliers, &s.suppliers);
    differ(&mut out, "crates", &c.crates, &s.crates);
    differ(&mut out, "godown", &c.godown, &s.godown);
    differ(&mut out, "gst_bills", &c.gst_bills, &s.gst_bills);
    let (co, so) = (
        c.owner.clone().unwrap_or_default(),
        s.owner.clone().unwrap_or_default(),
    );
    differ(&mut out, "owner.sale_costs", &co.sale_costs, &so.sale_costs);
    differ(
        &mut out,
        "owner.stock_value",
        &co.stock_value,
        &so.stock_value,
    );
    differ(&mut out, "owner.buy_rates", &co.buy_rates, &so.buy_rates);
    differ(
        &mut out,
        "owner.supplier_prices",
        &co.supplier_prices,
        &so.supplier_prices,
    );
    differ(
        &mut out,
        "owner.purchase_orders",
        &co.purchase_orders,
        &so.purchase_orders,
    );
    compare_book(&mut out, &co.book, &so.book);
    differ(&mut out, "manager projection", &core.manager, &sql.manager);
    differ(&mut out, "FIFO lots", &core.lots, &sql.lots);
    differ(&mut out, "fridge", &core.fridge, &sql.fridge);
    out
}

/// The authority's own clock stamps (receipt and bill creation keys) differ between the two;
/// only their presence is compared.
fn unstamped(book: &OwnerBook) -> OwnerBook {
    let mut b = book.clone();
    for s in &mut b.sales {
        s.creation_key = s.creation_key.map(|_| 0);
    }
    for r in &mut b.receipts {
        if let Some((_, key)) = r.selected.as_mut() {
            *key = 0;
        }
    }
    for a in &mut b.activity {
        a.at_ms = 0;
    }
    b
}

fn compare_book(out: &mut Vec<String>, core: &OwnerBook, sql: &OwnerBook) {
    let (c, s) = (unstamped(core), unstamped(sql));
    differ(out, "book.settings", &c.settings, &s.settings);
    differ(out, "book.products", &c.products, &s.products);
    differ(
        out,
        "book.checkout_terms",
        &c.checkout_terms,
        &s.checkout_terms,
    );
    differ(out, "book.sales", &c.sales, &s.sales);
    differ(out, "book.receipts", &c.receipts, &s.receipts);
    differ(out, "book.customers", &c.customers, &s.customers);
    differ(out, "book.purchasing", &c.purchasing, &s.purchasing);
    differ(out, "book.crates", &c.crates, &s.crates);
    differ(out, "book.godown", &c.godown, &s.godown);
    differ(out, "book.gst_bills", &c.gst_bills, &s.gst_bills);
    differ(out, "book.gst_deleted", &c.gst_deleted, &s.gst_deleted);
    differ(out, "book.cash_days", &c.cash_days, &s.cash_days);
    differ(out, "book.activity", &c.activity, &s.activity);
    differ(out, "book.legacy", &c.legacy, &s.legacy);
}

/// A device following one authority as one role by incremental pulls.
#[derive(Default)]
struct Device {
    checkpoint: Option<u64>,
    rows: Option<alm_core::projection::StoredRows>,
    snapshot: Option<ProjectionSnapshot>,
    changes: usize,
}

/// Pulls from `device`'s checkpoint, rebuilds what it holds, and compares it with a full pull.
fn follow<A: SalesAuthority>(a: &A, role: Role, device: &mut Device) -> Result<(), String> {
    use alm_core::projection::PulledRows;
    let batch = a
        .pull(role, device.checkpoint)
        .map_err(|e| format!("{e:?}"))?;
    if let (Role::Manager, Some(PulledRows::Changes(delta))) = (role, &batch.rows) {
        let text = delta.to_string();
        for word in ["\"owner\"", "cost", "buy", "margin", "fifo", "lots"] {
            if text.contains(word) {
                return Err(format!("Manager changes carry {word}"));
            }
        }
    }
    let view = alm_sync::delta::device_view(device.rows.as_ref(), &batch)
        .map_err(|_| "changes do not fit what the device holds".to_owned())?;
    if let Some((snapshot, rows, full)) = view {
        if full {
            device.rows = Some(rows);
        } else {
            device.changes += 1;
            let held = device.rows.as_mut().ok_or("no rows held")?;
            held.skeleton = rows.skeleton;
            held.lists.extend(rows.lists);
        }
        device.snapshot = Some(snapshot);
    }
    device.checkpoint = Some(batch.checkpoint);
    let full = a
        .pull(role, None)
        .map_err(|e| format!("{e:?}"))?
        .snapshot
        .ok_or("a full pull")?;
    if device.snapshot.as_ref() != Some(&full) {
        return Err("the incremental view differs from a full pull".into());
    }
    Ok(())
}

/// Both authorities, seeded with the same shop, and the differences seen so far.
pub struct Pair {
    pub local: LocalAuthority,
    pg: PgAuthority,
    scenario: &'static str,
    steps: usize,
    report: Vec<String>,
    /// Owner and Manager devices of the core, then of SQL.
    devices: [Device; 4],
}

impl Pair {
    /// None without `ALM_PG_URL`.
    pub fn new(scenario: &'static str) -> Option<Self> {
        Self::new_with(scenario, |_| {})
    }

    /// As [`Pair::new`], with extra seed terms applied to the shop first.
    pub fn new_with(
        scenario: &'static str,
        terms: impl FnOnce(&mut alm_core::confirmation::ConfirmationFixture),
    ) -> Option<Self> {
        let mut shop = fixture().unwrap();
        seed_purchasing(&mut shop).unwrap();
        terms(&mut shop);
        let mut local = LocalAuthority::new(shop, directory()).with_clock(|| 1_700_000_000_000);
        local.set_caller("diff-device", "diff-account");
        let pg = PgAuthority::connect(&local)?;
        let mut pair = Self {
            local,
            pg,
            scenario,
            steps: 0,
            report: Vec::new(),
            devices: Default::default(),
        };
        pair.check("seed", "ok", "ok");
        Some(pair)
    }

    fn check(&mut self, step: &str, core: &str, sql: &str) {
        let mut out = Vec::new();
        if core != sql {
            out.push(format!("  outcome:\n    core {core}\n    sql  {sql}"));
        }
        out.extend(compare(&local_effects(&self.local), &pg_effects(&self.pg)));
        for (i, role) in [Role::Owner, Role::Manager, Role::Owner, Role::Manager]
            .into_iter()
            .enumerate()
        {
            let device = &mut self.devices[i];
            let result = if i < 2 {
                follow(&self.local, role, device)
            } else {
                follow(&self.pg, role, device)
            };
            if let Err(error) = result {
                let side = if i < 2 { "core" } else { "sql" };
                out.push(format!("  incremental pull ({side}, {role:?}): {error}"));
            }
        }
        if !out.is_empty() {
            self.report.push(format!(
                "[{}] step {} {step}\n{}",
                self.scenario,
                self.steps,
                out.join("\n")
            ));
        }
    }

    /// Submits `intent` as `role` under the same request to both and compares.
    pub fn run(&mut self, role: Role, step: &str, intent: &Intent) -> Option<Outcome> {
        self.steps += 1;
        let request = RequestId::new(&format!("diff-{}-{}", self.scenario, self.steps)).unwrap();
        let core = self.local.submit(role, &request, intent);
        let sql = self.pg.submit(role, &request, intent);
        let (ck, sk) = (kind(&core), kind(&sql));
        self.check(step, &ck, &sk);
        // An identical retry returns the original outcome on both.
        if core.is_ok() {
            let again = (
                kind(&self.local.submit(role, &request, intent)),
                kind(&self.pg.submit(role, &request, intent)),
            );
            if again != (ck.clone(), sk.clone()) {
                self.report.push(format!(
                    "[{}] step {} {step} retry: core {} sql {}",
                    self.scenario, self.steps, again.0, again.1
                ));
            }
            let status = (
                self.local.status(Role::Owner, &request),
                self.pg.status(Role::Owner, &request),
            );
            if status.0 != status.1 {
                self.report.push(format!(
                    "[{}] step {} {step} status: core {:?} sql {:?}",
                    self.scenario, self.steps, status.0, status.1
                ));
            }
        }
        core.ok()
    }

    pub fn owner(&mut self, step: &str, intent: &Intent) -> Option<Outcome> {
        self.run(Role::Owner, step, intent)
    }

    /// Allocates a GST number for `bill` on both.
    pub fn gst_number(&mut self, step: &str, bill: u64) -> Option<u64> {
        self.steps += 1;
        let key = alm_sync::gst_bill_key(bill);
        let core = self.local.allocate_gst_number(Role::Owner, &key);
        let sql = self.pg.allocate_gst_number(Role::Owner, &key);
        self.check(step, &format!("{core:?}"), &format!("{sql:?}"));
        core.ok()
    }

    /// The Owner projection as the core has it.
    pub fn core(&self) -> ProjectionSnapshot {
        snapshot(&self.local, Role::Owner)
    }

    /// The Owner book rows as SQL has them.
    pub fn sql_book(&self) -> OwnerBook {
        snapshot(&self.pg, Role::Owner).owner.unwrap().book
    }

    /// Fails with every difference seen.
    pub fn finish(self) {
        if self.steps > 1 {
            for (i, device) in self.devices.iter().enumerate().filter(|(i, _)| i % 2 == 0) {
                assert!(device.changes > 0, "device {i} never took changes only");
            }
        }
        assert!(
            self.report.is_empty(),
            "SQL differs from the core:\n{}",
            self.report.join("\n")
        );
    }
}
