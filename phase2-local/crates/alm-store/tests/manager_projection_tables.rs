//! Every table of a Manager projection file, read back through SQLite, holds no cost, buy-rate,
//! FIFO, margin or profit data, while the Owner file of the same authority state does.
use std::collections::BTreeSet;
use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::owner_billing::{OwnerPosting, SaleTerms};
use alm_core::confirmation::{
    CrateTemplate, ManualGodownInput, PurchaseDraft, PurchaseLine, PurchaseSupplier, ReceiveLine,
    ReceiveRequest,
};
use alm_core::projection::ProjectionBatch;
use alm_core::{
    BillDate, CrateComponent, DraftCustomer, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role, SaleUnit,
};
use alm_store::ProjectionStore;
use rusqlite::Connection;

struct Root(std::path::PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

/// An authority state with a posted sale, a received PO, a crate, a godown row and quotes.
fn busy() -> alm_core::confirmation::ConfirmationFixture {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    let owner = Role::Owner;
    let draft = ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(2).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: alm_core::confirmation::demo_fixture::customer_id(),
        name: "Synthetic Customer".into(),
    })
    .with_bill_date(Some(day()));
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            &mut shop,
            &directory(),
            owner,
            Some(&draft),
            SaleTerms {
                date: "2025-04-30",
                paid: "20",
            },
        )
        .unwrap();
    let serial = posting.pending(owner).unwrap().serial();
    posting.confirm(&mut shop, owner, serial).unwrap();
    let id = shop
        .save_purchase_order(
            owner,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Depot".into(),
                }),
                date: Some(day()),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic Soda".into(),
                    product_id: Some(ProductId::new(2)),
                    ordered: NonZeroU32::new(10).unwrap(),
                    rate: Money::from_minor_units(3_610),
                }],
            },
        )
        .unwrap();
    shop.receive_purchase_order(ReceiveRequest {
        role: owner,
        id,
        date: Some(day()),
        today: day(),
        lines: &[ReceiveLine {
            quantity: 4,
            rate: Money::from_minor_units(3_620),
        }],
    })
    .unwrap();
    shop.save_crate_template(
        owner,
        CrateTemplate {
            name: "Party".into(),
            components: vec![CrateComponent::new(
                ProductId::new(1),
                NonZeroU32::new(2).unwrap(),
                SaleUnit::Case,
            )],
            price: Money::from_minor_units(40_000),
        },
    )
    .unwrap();
    shop.save_manual_godown(
        owner,
        ManualGodownInput {
            item: "Synthetic Soda",
            cases: "9",
            note: "",
        },
    )
    .unwrap();
    shop
}

/// Every cell of every table, as text.
fn cells(path: &std::path::Path) -> Vec<(String, String, String)> {
    let conn = Connection::open(path).unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = Vec::new();
    for table in tables {
        let mut query = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns: Vec<String> = query.column_names().map(str::to_owned).collect();
        let mut rows = query.query([]).unwrap();
        out.push((table.clone(), String::new(), columns.join(",")));
        while let Some(row) = rows.next().unwrap() {
            for (i, column) in columns.iter().enumerate() {
                let value: rusqlite::types::Value = row.get(i).unwrap();
                let text = match value {
                    rusqlite::types::Value::Null => String::new(),
                    rusqlite::types::Value::Integer(n) => n.to_string(),
                    rusqlite::types::Value::Real(f) => f.to_string(),
                    rusqlite::types::Value::Text(t) => t,
                    rusqlite::types::Value::Blob(b) => String::from_utf8_lossy(&b).into_owned(),
                };
                out.push((table.clone(), column.clone(), text));
            }
        }
    }
    out
}

fn file(root: &std::path::Path, role: &str) -> std::path::PathBuf {
    std::fs::read_dir(root.join("projections"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.to_string_lossy()
                .ends_with(&format!("-{role}.sqlite3"))
        })
        .unwrap()
}

#[test]
fn each_manager_projection_table_holds_no_cost_buy_rate_fifo_margin_or_profit_data() {
    let root =
        Root(std::env::temp_dir().join(format!("alm-manager-tables-{}", std::process::id())));
    std::fs::create_dir_all(&root.0).unwrap();
    let shop = busy();
    let book = ManualCashBook::default();
    for role in [Role::Owner, Role::Manager] {
        let mut store = ProjectionStore::open_scoped(&root.0, "trial", role).unwrap();
        let snapshot = shop.projection(role, &directory(), &book).unwrap();
        store
            .apply(&ProjectionBatch {
                role,
                checkpoint: 1,
                as_of_ms: 1,
                snapshot: Some(snapshot),
            })
            .unwrap();
    }
    let owner = shop
        .projection(Role::Owner, &directory(), &book)
        .unwrap()
        .owner
        .unwrap();
    // The cost values this state holds: buy rates, quotes, PO and receipt rates, lot costs,
    // the frozen sale cost and the FIFO stock value.
    let mut costs: BTreeSet<i64> = owner
        .buy_rates
        .iter()
        .map(|(_, m)| m.minor_units())
        .chain(
            owner
                .supplier_prices
                .iter()
                .map(|(_, _, m)| m.minor_units()),
        )
        .chain(
            owner
                .purchase_orders
                .iter()
                .flat_map(|o| o.lines.iter().map(|l| l.rate.minor_units())),
        )
        .chain(
            owner
                .sale_costs
                .iter()
                .filter_map(|(_, _, c)| c.map(Money::minor_units)),
        )
        .chain(owner.stock_value.map(Money::minor_units))
        .chain([7_000, 3_800, 3_620])
        .collect();
    costs.remove(&0);
    assert!(costs.len() >= 8, "{costs:?}");

    let owner_cells = cells(&file(&root.0, "owner"));
    assert!(
        owner_cells
            .iter()
            .any(|(t, _, v)| t == "owner_buy_rates" || v.contains("unit_cost")),
        "the Owner file holds costs, so the Manager check is meaningful"
    );

    let manager = cells(&file(&root.0, "manager"));
    let tables: BTreeSet<&str> = manager.iter().map(|(t, _, _)| t.as_str()).collect();
    assert!(tables.contains("projected_catalog") && tables.contains("projected_products"));
    for (table, column, value) in &manager {
        assert!(!table.starts_with("owner_"), "{table} in a Manager file");
        let lower = format!("{column} {value}").to_lowercase();
        for word in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
            assert!(
                ![
                    "cost", "costs", "buy", "fifo", "margin", "profit", "lot", "lots", "value"
                ]
                .contains(&word),
                "{word} in Manager table {table}.{column} = {value}"
            );
        }
        if let Ok(number) = value.parse::<i64>() {
            assert!(
                !costs.contains(&number),
                "cost value {number} in Manager table {table}.{column}"
            );
        }
    }
    // The Manager rows are the catalog it drafts with, and nothing else.
    let filled: BTreeSet<&str> = manager
        .iter()
        .filter(|(_, column, _)| !column.is_empty())
        .map(|(t, _, _)| t.as_str())
        .collect();
    assert_eq!(
        filled,
        [
            "projected_catalog",
            "projection_checkpoint",
            "projection_scope",
            "projected_notes"
        ]
        .into_iter()
        .filter(|t| tables.contains(t))
        .collect(),
        "{filled:?}"
    );
}
