use std::num::NonZeroU32;

use alm_core::confirmation::{
    ConfirmationFixture, CustomerReceiptInput, OpeningBalance, ReceiptMode, RequestId,
};
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft,
};

use super::OWNER;

pub(super) const TODAY: BillDate = match BillDate::new(2025, 4, 15) {
    Ok(today) => today,
    Err(_) => panic!("invalid frozen UTC day"),
};
pub(super) const NOON_MS: u64 = 1_744_718_400_000;
const STOCKED: ProductId = ProductId::new(9);
const EMPTY: ProductId = ProductId::new(2);
pub(super) const CATALOG: [(ProductId, &str); 2] = [(STOCKED, "Zinc Soda"), (EMPTY, "Apple Soda")];

pub(super) struct Contact {
    pub(super) id: DraftCustomerId,
    pub(super) name: &'static str,
    pub(super) mobile: &'static str,
}

pub(super) struct Fixture {
    pub(super) app: ConfirmationFixture,
    pub(super) contacts: [Contact; 5],
}

pub(super) fn date(iso: &str) -> BillDate {
    iso.parse().expect("canonical replay date")
}

pub(super) const fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

pub(super) fn seed() -> Fixture {
    use ReceiptMode::{Cash, Online};
    let mut app = ConfirmationFixture::with_ordered_lots(
        STOCKED,
        &[(2, money(349)), (6, money(463))],
        money(12_549),
    )
    .unwrap();
    app.add_product(EMPTY, 0, money(117)).unwrap();
    let contacts = [
        ("second", "Opening debtor", "0000000002", 12_150),
        ("zero", "Zero", "", 0),
        ("buyer", "Buyer", "0000000001", 0),
        ("rounds-away", "Rounded to zero", "", 49),
        ("advance", "Advance", "", 0),
    ]
    .map(|(key, name, mobile, opening)| {
        let id = DraftCustomerId::new(key.into());
        app.register_customer(OWNER, &id).unwrap();
        if opening != 0 {
            app.save_customer_opening(
                OWNER,
                &id,
                OpeningBalance {
                    amount: Some(money(opening)),
                    date: None,
                },
            )
            .unwrap();
        }
        Contact { id, name, mobile }
    });
    let [_, zero, buyer, rounds_away, advance] = &contacts;
    for (id, iso, contact, paid, mode, cancelled) in [
        (1, "2025-02-28", buyer, 4_949, Cash, false),
        (2, "2025-03-01", buyer, 2_449, Cash, false),
        (3, "2025-03-01", buyer, 2_449, Online, false),
        (4, "2025-03-02", buyer, 12_000, Online, false),
        (5, "2025-02-28", zero, 250, Cash, true),
        (6, "2025-03-01", zero, 250, Cash, true),
        (7, "2025-04-16", zero, 250, Cash, true),
    ] {
        let draft_id = DraftId::new(id);
        let draft = ProvisionalDraft::new(draft_id, vec![DraftLine::new(STOCKED, NonZeroU32::MIN)])
            .with_bill_date(Some(date(iso)))
            .with_customer(DraftCustomer::Selected {
                id: contact.id.clone(),
                name: contact.name.into(),
            });
        app.register_draft(draft).unwrap();
        let review = app
            .review_named_with_mode(OWNER, draft_id, money(paid), mode)
            .unwrap();
        app.confirm(OWNER, RequestId::new(id), &review).unwrap();
        if cancelled {
            app.cancel_sale(OWNER, draft_id).unwrap();
        }
    }
    for (iso, contact, amount, mode) in [
        ("2025-03-03", advance, 150, Online),
        ("2025-02-28", buyer, 3_051, Online),
        ("2025-03-01", buyer, 49, Cash),
        ("2025-04-17", rounds_away, 1, Cash),
    ] {
        app.record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: contact.id.clone(),
                date: date(iso),
                amount: money(amount),
                mode,
            },
        )
        .unwrap();
    }
    Fixture { app, contacts }
}
