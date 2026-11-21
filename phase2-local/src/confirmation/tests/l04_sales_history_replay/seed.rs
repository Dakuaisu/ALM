use crate::confirmation::{ConfirmationFixture, CustomerReceiptInput, ReceiptMode, RequestId};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};
use std::num::NonZeroU32;

pub(super) const OWNER: Role = Role::Owner;
pub(super) const GROVE_ID: &str = "grove";
const LIME: ProductId = ProductId::new(1);
const SODA: ProductId = ProductId::new(2);
const GROVE: Party = Party {
    id: GROVE_ID,
    name: "Synthetic Grove",
};
const ORCHARD: Party = Party {
    id: "orchard",
    name: "Synthetic Orchard",
};

#[derive(Clone, Copy)]
struct Party {
    id: &'static str,
    name: &'static str,
}

pub(super) fn date(iso: &str) -> BillDate {
    iso.parse().unwrap()
}

pub(super) fn history_fixture() -> ConfirmationFixture {
    let money = Money::from_minor_units;
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(LIME, &[(10, money(950))], money(1499)).unwrap();
    fixture.add_product(SODA, 0, money(1049)).unwrap();
    fixture
        .catalog
        .seed_lots(SODA, &[(10, money(600))])
        .unwrap();
    for (product, name) in [(LIME, "Synthetic Lime"), (SODA, "Synthetic Soda")] {
        let mut form = fixture.open_product_entry(OWNER, product).unwrap();
        form.item = name.into();
        fixture.save_product_entry(OWNER, &form).unwrap();
    }

    // Posting takes place in this sequence, independently of document date or draft ID.
    for (id, document, party, product, quantity, paid) in [
        (41, "2025-03-25", Some(GROVE), SODA, 2, 149),
        (12, "2025-02-28", Some(ORCHARD), SODA, 1, 75),
        (73, "2025-03-01", Some(GROVE), LIME, 1, 149),
        (36, "2025-03-15", Some(GROVE), SODA, 1, 149),
        (24, "2025-03-15", Some(ORCHARD), LIME, 1, 251),
        (58, "2025-03-15", None, LIME, 1, 0),
    ] {
        let draft_id = DraftId::new(id);
        let customer = match party {
            Some(party) => DraftCustomer::Selected {
                id: DraftCustomerId::new(party.id.into()),
                name: party.name.into(),
            },
            None => DraftCustomer::Typed("Synthetic Guest".into()),
        };
        fixture
            .register_draft(
                ProvisionalDraft::new(
                    draft_id,
                    vec![DraftLine::new(product, NonZeroU32::new(quantity).unwrap())],
                )
                .with_customer(customer)
                .with_bill_date(Some(date(document))),
            )
            .unwrap();
        let review = match party {
            Some(_) => fixture.review_named(OWNER, draft_id, money(paid)),
            // Legacy unlinked bills have no recorded payment; no partial-guest API is assumed.
            None => fixture.review(OWNER, draft_id),
        }
        .unwrap();
        fixture.confirm(OWNER, RequestId::new(id), &review).unwrap();
        if id == 24 {
            fixture.cancel_sale(OWNER, draft_id).unwrap();
        }
    }
    use ReceiptMode::{Cash, Online};
    for (party, document, amount, mode) in [
        (GROVE, "2025-03-10", 149, Online),
        (ORCHARD, "2025-03-10", 49, Cash),
        (GROVE, "2025-02-28", 51, Cash),
        (ORCHARD, "2025-03-15", 151, Online),
        (GROVE, "2025-03-25", 49, Cash),
        (GROVE, "2025-03-10", 51, Online),
        (GROVE, "2025-03-01", 50, Cash),
    ] {
        fixture
            .record_customer_receipt(
                OWNER,
                CustomerReceiptInput {
                    customer_id: DraftCustomerId::new(party.id.into()),
                    date: date(document),
                    amount: money(amount),
                    mode,
                },
            )
            .unwrap();
    }
    fixture
}
