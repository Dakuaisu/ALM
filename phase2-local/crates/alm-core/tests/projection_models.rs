//! Screen models built from a stored projection agree with the authority's own reads.
#![cfg(feature = "demo-fixture")]
use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::{customer_id, directory, fixture};
use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::owner_billing::{
    CustomerFilter, OwnerPosting, ReceiptTerms, SaleTerms, save_receipt,
};
use alm_core::confirmation::{ReceiptMode, SalesPeriod};
use alm_core::{BillDate, DraftCustomer, DraftId, DraftLine, ProductId, ProvisionalDraft, Role};

#[test]
fn projected_sales_customers_and_stock_match_the_authority_reads() {
    let mut f = fixture().unwrap();
    let dir = directory();
    let draft = ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(2).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: "Synthetic Customer".into(),
    });
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            &mut f,
            &dir,
            Role::Owner,
            Some(&draft),
            SaleTerms {
                date: "2025-04-30",
                paid: "20",
            },
        )
        .unwrap();
    posting.confirm(&mut f, Role::Owner, 1).unwrap();
    save_receipt(
        &mut f,
        &dir,
        Role::Owner,
        "batch1-customer",
        ReceiptTerms {
            date: "2025-05-01",
            amount: "10",
            mode: ReceiptMode::Online,
        },
    )
    .unwrap();
    let day = BillDate::new(2025, 5, 2).unwrap();
    let snapshot = f
        .projection(Role::Owner, &dir, &ManualCashBook::default())
        .unwrap();
    for period in [SalesPeriod::All, SalesPeriod::Today, SalesPeriod::ThisMonth] {
        let core = f.sales_history(Role::Owner, day, period, "").unwrap();
        let view = snapshot.sales_view(Role::Owner, day, period, "").unwrap();
        assert_eq!(view.rows.len(), core.rows.len());
        assert_eq!(view.sales, core.sales);
        assert_eq!(view.total_collected, core.total_collected);
        assert_eq!(view.packets, core.active_packets);
        assert_eq!(view.profit, core.profit.ok().map(|p| p.profit));
    }
    assert!(
        snapshot
            .sales_view(Role::Owner, day, SalesPeriod::All, "lime")
            .unwrap()
            .rows
            .len()
            == 1
    );
    assert!(
        snapshot
            .sales_view(Role::Owner, day, SalesPeriod::All, "nobody")
            .unwrap()
            .rows
            .is_empty()
    );
    for filter in [
        CustomerFilter::All,
        CustomerFilter::Due,
        CustomerFilter::Name("market".into()),
    ] {
        let core = dir.customer_book(&f, Role::Owner, &filter).unwrap();
        let view = snapshot.customer_view(Role::Owner, &filter).unwrap();
        assert_eq!(view.rows.len(), core.rows.len());
        assert_eq!(
            (view.receivable, view.advances, view.debtors),
            (core.receivable, core.advances, core.debtors)
        );
    }
    let core = f.stock_list(Role::Owner, "", true).unwrap();
    let view = snapshot.stock_view(Role::Owner, "", true).unwrap();
    assert_eq!(
        (view.skus, view.low, view.out, view.value),
        (core.skus, core.low, core.out, core.value)
    );
    assert_eq!(view.rows.len(), core.rows.len());
    let manager = f
        .projection(Role::Manager, &dir, &ManualCashBook::default())
        .unwrap();
    assert!(manager.owner.is_none() && manager.sales.is_empty() && manager.receipts.is_empty());
    // DEC-034: a Manager's stock view (from its own lists) has quantities and no value.
    let quantities = snapshot.stock_view(Role::Manager, "", false).unwrap();
    assert_eq!(quantities.value, None);
    assert_eq!(quantities.skus, view.skus);
    // DEC-034: a Manager's Sales view holds the same bills and totals and never the profit.
    let owner_sales = snapshot
        .sales_view(Role::Owner, day, SalesPeriod::All, "")
        .unwrap();
    let manager_sales = snapshot
        .sales_view(Role::Manager, day, SalesPeriod::All, "")
        .unwrap();
    assert!(owner_sales.profit.is_some());
    assert_eq!(manager_sales.profit, None);
    assert_eq!(
        (
            manager_sales.rows,
            manager_sales.sales,
            manager_sales.total_collected
        ),
        (
            owner_sales.rows,
            owner_sales.sales,
            owner_sales.total_collected
        )
    );
    // DEC-034: a Manager's Customers view holds the same balances.
    assert_eq!(
        snapshot.customer_view(Role::Manager, &CustomerFilter::All),
        snapshot.customer_view(Role::Owner, &CustomerFilter::All)
    );
}
