use alm_store::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, EstimateLineDisplay,
};
use alm_store::synthetic_read_model::{Error, SaleId, Store};
use alm_store::{BillDate, Role};

use super::fixture::{A, B, ITEM_A, ITEM_B, SALE, feb, money};

#[path = "../milestone_d_replay/outputs/layout.rs"]
mod layout;

pub(super) fn source(fixture: &ConfirmationFixture) -> String {
    assert_eq!(
        fixture.bill_preview(Role::Manager, SALE, BalanceMode::Latest),
        Err(ConfirmationError::AccessDenied)
    );
    let preview = fixture
        .bill_preview(Role::Owner, SALE, BalanceMode::Latest)
        .unwrap();
    assert_eq!(
        (preview.invoice, preview.date, preview.cancelled),
        (1, feb(21), true)
    );
    assert_eq!((preview.items_quantity, preview.loading), (3, money(0)));
    assert_eq!(
        (preview.total, preview.paid, preview.balance),
        (money(41_000), money(10_000), money(-5_000))
    );
    let labels = [
        EstimateLineDisplay {
            product: A,
            label: ITEM_A,
        },
        EstimateLineDisplay {
            product: B,
            label: ITEM_B,
        },
    ];
    layout::render(&preview, &labels, today())
}

pub(super) fn document(store: &Store) -> String {
    let id = SaleId::new("sale-cancelled-id".into());
    for denied in [&id, &SaleId::new("unknown-sale".into())] {
        assert!(matches!(
            store.estimate_document(Role::Manager, denied, today()),
            Err(Error::AccessDenied)
        ));
    }
    let bill = store.estimate_document(Role::Owner, &id, today()).unwrap();
    assert_eq!(bill.customer, "Synthetic Buyer");
    assert_eq!(
        (
            bill.items_total,
            bill.loading,
            bill.total,
            bill.paid,
            bill.balance
        ),
        (
            money(41_000),
            money(0),
            money(41_000),
            money(10_000),
            money(-5_000)
        )
    );
    assert!(
        bill.lines
            .iter()
            .map(|line| (line.label, line.quantity, line.price, line.amount))
            .eq([
                (ITEM_A, 2, money(17_700), money(35_400)),
                (ITEM_B, 1, money(5_600), money(5_600)),
            ])
    );
    layout::render_document(&bill)
}

fn today() -> BillDate {
    BillDate::new(2025, 3, 1).unwrap()
}
