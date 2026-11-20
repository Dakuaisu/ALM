//! L09-03 source-derived coverage: ALM_Business_Manager.html 10466–10479 and 7941–7948.
//! Only the disposable directory and explicit column list model the legacy deletion.

use super::super::{
    BillDate, PurchaseDraft, PurchaseError, PurchaseLine, PurchaseOrderId, PurchaseOrderStatus,
    PurchaseProduct, PurchaseReceipt, PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult,
};
use super::*;
use std::num::NonZeroU32;

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const ITEM: &str = "Synthetic Orange";
const TARGET: &str = "Synthetic Quoted Supply";
const CONTROL: &str = "Synthetic Control Supply";
const OPENING_STOCK: u32 = 10;
const OPENING_BUY: Money = Money::from_minor_units(1_000);
const SELL: Money = Money::from_minor_units(1_500);
const RECEIPT_RATE: Money = Money::from_minor_units(1_300);
const QUOTE: Money = Money::from_minor_units(875);
const QUANTITY: u32 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisposableDirectory {
    fixture: ConfirmationFixture,
    contacts: Vec<SavedPurchaseSupplier>,
    explicit: Vec<PurchaseSupplierId>,
}

#[derive(Debug, Eq, PartialEq)]
struct DeleteDialog {
    id: PurchaseSupplierId,
    title: String,
    warning: &'static str,
}

enum DeleteResponse {
    Decline,
    Confirm(Role),
}

impl DisposableDirectory {
    fn open(&self, role: Role, id: PurchaseSupplierId) -> Result<DeleteDialog, PurchaseError> {
        owner(role)?;
        let contact = self
            .contacts
            .iter()
            .find(|contact| contact.id == id)
            .unwrap();
        Ok(DeleteDialog {
            id,
            title: format!("Delete {}?", contact.name),
            warning: "Purchase history stays, and any buy rates recorded under this name stay visible in the price comparison.",
        })
    }

    fn resolve(
        &mut self,
        dialog: DeleteDialog,
        response: DeleteResponse,
    ) -> Result<(), PurchaseError> {
        match response {
            DeleteResponse::Decline => Ok(()),
            DeleteResponse::Confirm(role) => {
                owner(role)?;
                let index = self
                    .contacts
                    .iter()
                    .position(|contact| contact.id == dialog.id)
                    .unwrap();
                let removed = self.contacts.remove(index);
                let quoted = self
                    .fixture
                    .purchases
                    .quotes
                    .values()
                    .any(|quotes| quotes.rate(&removed.name).is_some());
                if quoted && !self.explicit.contains(&removed.id) {
                    self.explicit.push(removed.id);
                }
                Ok(())
            }
        }
    }

    fn reopen(
        &self,
        role: Role,
        product: Option<ProductId>,
    ) -> Result<SupplierPriceComparison, SupplierPriceError> {
        self.fixture.compare_supplier_prices_with_columns(
            role,
            product,
            PriceSupplierColumns {
                explicit: &self.explicit,
                contacts: &self.contacts,
            },
        )
    }
}

fn jan(day: u8) -> BillDate {
    BillDate::new(2025, 1, day).unwrap()
}

fn seeded_directory() -> (DisposableDirectory, PurchaseOrderId, PurchaseSupplierId) {
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(OPENING_STOCK, OPENING_BUY)], SELL)
            .unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: OPENING_BUY,
        })
        .unwrap();
    let mut save = |name: &str, date| {
        fixture
            .save_purchase_order(
                OWNER,
                &PurchaseDraft {
                    supplier: Some(PurchaseSupplier {
                        id: None,
                        name: name.into(),
                    }),
                    date: Some(date),
                    note: String::new(),
                    items: vec![PurchaseLine {
                        item: ITEM.into(),
                        product_id: Some(PRODUCT),
                        ordered: NonZeroU32::new(QUANTITY).unwrap(),
                        rate: OPENING_BUY,
                    }],
                },
            )
            .unwrap()
    };
    save(CONTROL, jan(1));
    let po = save(TARGET, jan(2));
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id: po,
            date: Some(jan(3)),
            today: jan(31),
            lines: &[ReceiveLine {
                quantity: QUANTITY,
                rate: RECEIPT_RATE
            }],
        }),
        Ok(ReceiveResult::Received),
    );
    let contacts = fixture.purchase_suppliers(OWNER).unwrap().to_vec();
    let target = fixture.purchase_orders(OWNER).unwrap()[po.0].supplier_id;
    fixture
        .edit_comparison_price(
            OWNER,
            SupplierPriceEdit {
                product: PRODUCT,
                supplier: target,
                rate: Some(QUOTE),
            },
        )
        .unwrap();
    let explicit = fixture.purchases.price_suppliers.clone();
    let directory = DisposableDirectory {
        fixture,
        contacts,
        explicit,
    };
    (directory, po, target)
}

#[test]
fn l09_03_supplier_delete_open_shows_exact_title_and_warning_without_mutation() {
    let (directory, _, target) = seeded_directory();
    let before = directory.clone();

    let dialog = directory.open(OWNER, target).unwrap();

    assert_eq!(dialog.title, "Delete Synthetic Quoted Supply?");
    assert_eq!(
        dialog.warning,
        "Purchase history stays, and any buy rates recorded under this name stay visible in the price comparison."
    );
    assert_eq!(directory, before);
}

#[test]
fn l09_03_supplier_delete_decline_preserves_directory_and_comparison() {
    let (mut directory, _, target) = seeded_directory();
    let before = directory.clone();
    let comparison = before.reopen(OWNER, None).unwrap();
    let dialog = directory.open(OWNER, target).unwrap();

    assert_eq!(directory.resolve(dialog, DeleteResponse::Decline), Ok(()));

    assert_eq!(directory, before);
    assert_eq!(directory.reopen(OWNER, None), Ok(comparison));
}

#[test]
fn l09_03_supplier_delete_manager_sees_no_quote_and_cannot_mutate() {
    let (mut directory, _, target) = seeded_directory();
    let before = directory.clone();
    let dialog = directory.open(OWNER, target).unwrap();
    let manager = Role::Manager;
    let denied = PurchaseError::AccessDenied;

    assert_eq!(directory.open(manager, target).unwrap_err(), denied);
    assert_eq!(
        directory.fixture.purchase_suppliers(manager).unwrap_err(),
        denied
    );
    for product in [None, Some(PRODUCT)] {
        assert_eq!(
            directory.reopen(manager, product),
            Err(SupplierPriceError::AccessDenied)
        );
    }
    let result = directory.resolve(dialog, DeleteResponse::Confirm(manager));
    assert_eq!(result, Err(denied));
    assert_eq!(directory, before);
}

#[test]
fn l09_03_supplier_delete_owner_removes_only_contact_and_reopens_its_quote() {
    let (mut directory, po, target_id) = seeded_directory();
    let before = directory.clone();
    let [control, target] = before.contacts.as_slice() else {
        panic!("expected control and target contacts");
    };
    assert_eq!(control.id, PurchaseSupplierId(0));
    assert_eq!(control.name, CONTROL);
    assert_eq!(target_id, PurchaseSupplierId(1));
    assert_eq!(target.id, target_id);
    assert_eq!(target.name, TARGET);
    assert!(before.explicit.is_empty());
    assert_eq!(
        before
            .fixture
            .purchases
            .product_quotes(PRODUCT)
            .rate(TARGET),
        Some(QUOTE)
    );
    let comparison = before.fixture.compare_supplier_prices(OWNER, None).unwrap();
    assert_eq!(comparison.suppliers, before.contacts);
    assert_eq!(comparison.rows[0].prices, [None, Some(QUOTE)]);
    let dialog = directory.open(OWNER, target_id).unwrap();

    directory
        .resolve(dialog, DeleteResponse::Confirm(OWNER))
        .unwrap();

    assert_eq!(directory.contacts, std::slice::from_ref(control));
    assert_eq!(directory.explicit, [target_id]);
    assert_eq!(
        directory
            .fixture
            .purchases
            .product_quotes(PRODUCT)
            .rate(TARGET),
        Some(QUOTE)
    );
    let reopened = directory.reopen(OWNER, None).unwrap();
    assert_eq!(directory.reopen(OWNER, Some(PRODUCT)).unwrap(), reopened);
    assert_eq!(reopened.suppliers, [target.clone(), control.clone()]);
    let expected_row = SupplierPriceRow {
        product: PRODUCT,
        item: ITEM.into(),
        our_buy: RECEIPT_RATE,
        prices: vec![Some(QUOTE), None],
        best: Some((target_id, QUOTE)),
    };
    assert_eq!(reopened.rows, [expected_row]);
    assert!(reopened.rows[0].is_highlighted(0));
    assert!(!reopened.rows[0].is_highlighted(1));

    let orders = directory.fixture.purchase_orders(OWNER).unwrap();
    assert_eq!(po, PurchaseOrderId(1));
    let order = &orders[po.0];
    assert_eq!(order.id, po);
    assert_eq!(order.supplier_id, target_id);
    assert_eq!(order.supplier_name, TARGET);
    assert_eq!(order.date, jan(2));
    assert_eq!(order.status(), PurchaseOrderStatus::Completed);
    let receipt = PurchaseReceipt {
        date: jan(3),
        quantity: NonZeroU32::new(QUANTITY).unwrap(),
        rate: RECEIPT_RATE,
    };
    assert_eq!(order.items[0].receipts(), &[receipt]);
    assert_eq!(directory.fixture.product_stock(PRODUCT), Some(13));
    assert_eq!(directory.fixture, before.fixture);
}
