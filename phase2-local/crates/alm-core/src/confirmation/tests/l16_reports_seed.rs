use super::super::l04_sales_history_replay::seed::history_fixture;
use super::super::l15_cashbook_numeric_replay::saved_l15_row;
use super::date;
use crate::confirmation::manual_cashbook::{
    ManualCashBook, ManualCashBookDraft, ManualCashBookRow,
};
use crate::confirmation::{ConfirmationFixture, CustomerReceiptInput, ReceiptMode, RequestId};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};
use std::num::NonZeroU32;

pub(super) struct MonthlyReportSeed {
    pub(super) fixture: ConfirmationFixture,
    pub(super) book: ManualCashBook,
    pub(super) original: ManualCashBookRow,
    pub(super) saved: [ManualCashBookRow; 2],
}

pub(super) fn monthly_reports_fixture(
    ancestor_date: BillDate,
    april_received: Money,
) -> MonthlyReportSeed {
    let mut fixture = history_fixture();
    let product = ProductId::new(16);
    fixture
        .add_product(product, 0, Money::from_minor_units(15_025))
        .unwrap();
    fixture
        .catalog
        .seed_lots(product, &[(2, Money::from_minor_units(5_000))])
        .unwrap();
    // Post April last so the chart replay can cancel it through the Owner API.
    for (id, day, customer_id, name, received) in [
        (
            81,
            ancestor_date,
            "ancestor",
            "Synthetic Ancestor",
            Money::from_minor_units(0),
        ),
        (
            80,
            date("2025-04-25"),
            "same-name",
            "Synthetic Grove",
            april_received,
        ),
    ] {
        let draft_id = DraftId::new(id);
        fixture
            .register_draft(
                ProvisionalDraft::new(
                    draft_id,
                    vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
                )
                .with_customer(DraftCustomer::Selected {
                    id: DraftCustomerId::new(customer_id.into()),
                    name: name.into(),
                })
                .with_bill_date(Some(day)),
            )
            .unwrap();
        let review = fixture
            .review_named(Role::Owner, draft_id, received)
            .unwrap();
        fixture
            .confirm(Role::Owner, RequestId::new(id), &review)
            .unwrap();
    }
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: DraftCustomerId::new("same-name".into()),
                date: date("2025-01-10"),
                amount: Money::from_minor_units(100),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();

    let (mut book, original) = saved_l15_row();
    // Rebase the original L15 saved row; preserve its counted cash and online amounts.
    let mut feb = original.draft();
    feb.date = Some(date("2025-02-28"));
    feb.amounts.goods = Money::from_minor_units(1_201);
    feb.amounts.misc = Money::from_minor_units(2_302);
    feb.amounts.salary = Money::from_minor_units(3_403);
    feb.amounts.debt = Money::from_minor_units(4_504);
    let feb = book.save(Role::Owner, &feb).unwrap();
    let mut april = ManualCashBookDraft::new(Some(date("2025-04-10")), feb.amounts);
    april.amounts.goods = Money::from_minor_units(999);
    april.amounts.misc = Money::from_minor_units(25);
    april.amounts.salary = Money::from_minor_units(25);
    let april = book.save(Role::Owner, &april).unwrap();
    MonthlyReportSeed {
        fixture,
        book,
        original,
        saved: [feb, april],
    }
}
