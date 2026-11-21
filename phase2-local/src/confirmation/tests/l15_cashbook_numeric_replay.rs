use super::*;
use crate::DraftCustomerId;
use crate::confirmation::PreviewPayment;
use crate::confirmation::manual_cashbook::{
    CashBookAmounts, ManualCashBook, ManualCashBookDraft, ManualCashBookRow,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const CUSTOMER: &str = "l15-signed-history";

fn date() -> BillDate {
    BillDate::new(2024, 2, 29).unwrap()
}

const fn money(minor_units: i64) -> Money {
    Money::from_minor_units(minor_units)
}

fn amounts(cash: i64, paytm: i64, phonepe: i64) -> CashBookAmounts {
    CashBookAmounts {
        cash: money(cash),
        paytm: money(paytm),
        phonepe: money(phonepe),
        goods: money(0),
        misc: money(0),
        salary: money(0),
        debt: money(0),
    }
}

pub(super) fn saved_l15_row() -> (ManualCashBook, ManualCashBookRow) {
    let mut book = ManualCashBook::default();
    let original = ManualCashBookDraft::new(Some(date()), amounts(111, 222, 333));
    let saved = book.save(OWNER, &original).unwrap();
    (book, saved)
}

fn bill(id: u64) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(id),
        vec![DraftLine::new(PRODUCT, NonZeroU32::new(1).unwrap())],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new(CUSTOMER.into()),
        name: "Synthetic L15 buyer".into(),
    })
    .with_bill_date(Some(date()))
}

#[test]
fn l15_01_signed_numeric_replay_rounds_each_bucket_and_saves_exact_manual_inputs() {
    // Given: an independent signed legacy history plus positive/zero bills through real APIs.
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, money(10_000)).unwrap();
    for (id, paid, invoice) in [(1, 175, 1), (2, 0, 2), (90, 0, 3)] {
        let draft = bill(id);
        fixture.register_draft(draft).unwrap();
        let review =
            fixture.review_named_with_mode(OWNER, DraftId::new(id), money(paid), ReceiptMode::Cash);
        let review = review.unwrap();
        let receipt = fixture.confirm(OWNER, RequestId::new(id), &review).unwrap();
        assert_eq!(receipt.invoice, invoice);
    }
    let mut fractional = ConfirmationFixture::new(PRODUCT, 10, money(0)).unwrap();
    for (index, mode) in [ReceiptMode::Cash, ReceiptMode::Online]
        .into_iter()
        .enumerate()
    {
        let customer_id = DraftCustomerId::new(format!("l15-fractional-{index}"));
        let bill_id = DraftId::new(10 + u64::try_from(index).unwrap());
        fractional
            .register_draft(
                bill(bill_id.value()).with_customer(DraftCustomer::Selected {
                    id: customer_id.clone(),
                    name: format!("Synthetic fractional {index}"),
                }),
            )
            .unwrap();
        let review = fractional.review_named(OWNER, bill_id, money(0)).unwrap();
        fractional
            .confirm(
                OWNER,
                RequestId::new(10 + u64::try_from(index).unwrap()),
                &review,
            )
            .unwrap();
        for _ in 0..2 {
            fractional
                .record_customer_receipt(
                    OWNER,
                    CustomerReceiptInput {
                        customer_id: customer_id.clone(),
                        date: date(),
                        amount: money(25),
                        mode,
                    },
                )
                .unwrap();
        }
    }
    let fractional_day = fractional.cashbook_day_close(OWNER, date()).unwrap();
    assert_eq!(
        (
            fractional_day.count,
            fractional_day.cash.minor_units(),
            fractional_day.online.minor_units(),
            fractional_day.cash_rupees,
            fractional_day.online_rupees,
        ),
        (4, 50, 50, 1, 1)
    );
    assert!(
        fractional_day
            .entries
            .iter()
            .all(|entry| { entry.kind == CashBookKind::Receipt && entry.amount == money(25) })
    );
    println!(
        "L15-01 quarter buckets expected count=4, paise=(50, 50), rupees=(1, 1); observed {fractional_day:?}"
    );

    // Seed only the historical rows that current APIs correctly reject.
    let mut signed = fixture;
    let (_, historical_bill, _) = signed.current_sales.get_mut(&DraftId::new(90)).unwrap();
    historical_bill.received = Some(money(-125));
    for (mode, paise) in [(ReceiptMode::Cash, 75), (ReceiptMode::Online, -150)] {
        signed
            .record_customer_receipt(
                OWNER,
                CustomerReceiptInput {
                    customer_id: DraftCustomerId::new(CUSTOMER.into()),
                    date: date(),
                    amount: money(paise),
                    mode,
                },
            )
            .unwrap();
    }
    signed.preview_payments.push(PreviewPayment {
        customer_id: CUSTOMER.into(),
        amount: money(0),
        date: date(),
        after_invoice: 3,
        mode: Some(ReceiptMode::Online),
    });
    let zero_submission = signed.record_customer_receipt(
        OWNER,
        CustomerReceiptInput {
            customer_id: DraftCustomerId::new(CUSTOMER.into()),
            date: date(),
            amount: money(0),
            mode: ReceiptMode::Online,
        },
    );
    assert_eq!(zero_submission, Err(ConfirmationError::InvalidAmount));
    let negative_submission =
        signed.review_named_with_mode(OWNER, DraftId::new(1), money(-125), ReceiptMode::Cash);
    assert_eq!(negative_submission, Err(ConfirmationError::InvalidAmount));
    let day = signed.cashbook_day_close(OWNER, date()).unwrap();
    assert_eq!(
        day.entries
            .iter()
            .map(|entry| (&entry.kind, entry.mode, entry.amount.minor_units()))
            .collect::<Vec<_>>(),
        [
            (
                &CashBookKind::Bill {
                    draft_id: DraftId::new(1),
                    invoice: 1
                },
                ReceiptMode::Cash,
                175
            ),
            (&CashBookKind::Receipt, ReceiptMode::Cash, 75),
            (&CashBookKind::Receipt, ReceiptMode::Online, -150),
            (&CashBookKind::Receipt, ReceiptMode::Online, 0),
        ]
    );
    assert_eq!(
        (
            day.count,
            day.cash.minor_units(),
            day.online.minor_units(),
            day.cash_rupees,
            day.online_rupees,
        ),
        (4, 250, -150, 3, -1)
    );
    println!(
        "L15-01 signed history expected count=4, paise=(250, -150), rupees=(3, -1); observed {day:?}"
    );

    let base = ManualCashBookDraft::new(Some(date()), amounts(0, 0, 0));
    let comparison = base.compare(&fractional, OWNER).unwrap();
    assert_eq!(
        (
            comparison.system.cash_rupees,
            comparison.system.online_rupees
        ),
        (1, 1)
    );
    let cases = [
        ([150, 25, 24], (1, -1)),
        ([49, 75, 75], (-1, 1)),
        ([50, 25, 25], (0, 0)),
        ([100, 60, 40], (0, 0)),
    ];
    for (entered, expected) in cases {
        let draft =
            ManualCashBookDraft::new(Some(date()), amounts(entered[0], entered[1], entered[2]));
        let actual = draft.compare(&fractional, OWNER).unwrap();
        let observed = (
            actual.cash_difference_rupees,
            actual.online_difference_rupees,
        );
        assert_eq!(observed, expected);
        println!(
            "L15-02 manual paise={entered:?}; differences in rupees expected {expected:?}, observed {observed:?}"
        );
    }

    let (mut book, saved_original) = saved_l15_row();
    {
        let mut edited = saved_original.draft();
        edited.amounts = amounts(150, 25, 24);
        assert_eq!(book.row(OWNER, saved_original.id), Ok(Some(saved_original)));
        assert_eq!(
            edited
                .compare(&fractional, OWNER)
                .unwrap()
                .cash_difference_rupees,
            1
        );
    }
    let reopened = book.row(OWNER, saved_original.id).unwrap().unwrap().draft();
    assert_eq!(reopened.amounts, amounts(111, 222, 333));

    let mut exact = reopened;
    exact.amounts = amounts(149, 76, 73);
    let saved = book.save(OWNER, &exact).unwrap();
    assert_eq!(saved.id, saved_original.id);
    assert_eq!(saved.amounts, amounts(149, 76, 73));
    assert_eq!(book.row(OWNER, saved.id), Ok(Some(saved)));
    assert_eq!(
        (
            saved.amounts.cash.minor_units(),
            saved.amounts.paytm.minor_units(),
            saved.amounts.phonepe.minor_units()
        ),
        (149, 76, 73)
    );
    println!(
        "L15-02 discarded edit readback=(111p,222p,333p); saved exact=(149p,76p,73p); differences are not stored"
    );
}
