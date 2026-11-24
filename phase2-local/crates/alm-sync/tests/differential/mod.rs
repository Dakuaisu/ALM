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
struct Effects {
    owner: ProjectionSnapshot,
    manager: ProjectionSnapshot,
    lots: BTreeMap<u64, Vec<Lot>>,
    fridge: BTreeMap<u64, Option<u32>>,
}

/// An outcome with rejections compared by kind (the core and SQL word reasons differently).
fn kind(result: &Result<Outcome, AuthorityError>) -> String {
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

fn local_effects(a: &LocalAuthority) -> Effects {
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

fn pg_effects(a: &PgAuthority) -> Effects {
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

fn compare(core: &Effects, sql: &Effects) -> Vec<String> {
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
}

/// Both authorities, seeded with the same shop, and the differences seen so far.
pub struct Pair {
    pub local: LocalAuthority,
    pg: PgAuthority,
    scenario: &'static str,
    steps: usize,
    report: Vec<String>,
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
        let local = LocalAuthority::new(shop, directory()).with_clock(|| 1_700_000_000_000);
        let pg = PgAuthority::connect(&local)?;
        let mut pair = Self {
            local,
            pg,
            scenario,
            steps: 0,
            report: Vec::new(),
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

    /// Fails with every difference seen.
    pub fn finish(self) {
        assert!(
            self.report.is_empty(),
            "SQL differs from the core:\n{}",
            self.report.join("\n")
        );
    }
}
