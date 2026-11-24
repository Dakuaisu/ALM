use std::num::NonZeroU32;

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft,
};

use super::{
    CrateTerms, Intent, Outcome, ProductTerms, PurchaseLineTerms, PurchaseOrderTerms, SaleIntent,
    SaleReceipt,
};

#[test]
fn canonical_text_round_trips_and_rejects_other_text() {
    let draft = ProvisionalDraft::new(
        DraftId::new(7),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(2).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("batch1-customer".into()),
        name: "Synthetic Customer: A&B".into(),
    })
    .with_bill_date(Some(BillDate::new(2025, 4, 30).unwrap()));
    let sale = Intent::PostSale(SaleIntent {
        draft,
        date: BillDate::new(2025, 4, 30).unwrap(),
        paid: Money::from_minor_units(2_000),
        prices: vec![Money::from_minor_units(12_050)],
    });
    let text = sale.canonical();
    assert_eq!(
        text,
        "sale draft=7 customer=s:batch1-customer:Synthetic%20Customer%3A%20A%26B billdate=2025-04-30 date=2025-04-30 paid=2000 lines=1x2@12050"
    );
    assert_eq!(Intent::parse(&text), Some(sale.clone()));
    assert_eq!(
        sale.supabase_terms().unwrap(),
        r#"{"draft_id": "7", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 12050}]}"#
    );
    for intent in [
        Intent::CancelSale { draft: 7 },
        Intent::EditSale {
            draft: 7,
            lines: vec![(3, 11_800)],
        },
        Intent::RecordReceipt {
            customer: "batch1-market".into(),
            date: BillDate::new(2025, 5, 1).unwrap(),
            amount: Money::from_minor_units(3_050),
            online: true,
        },
        Intent::SaveCashDay {
            id: None,
            date: BillDate::new(2025, 4, 30).unwrap(),
            amounts: CashBookAmounts::ZERO,
        },
    ] {
        assert_eq!(Intent::parse(&intent.canonical()), Some(intent));
    }
    let day = BillDate::new(2025, 4, 30).unwrap();
    for intent in [
        Intent::SaveProduct(ProductTerms {
            id: 3,
            new: false,
            item: "Orange: 1,5 L".into(),
            brand: String::new(),
            buy: Money::from_minor_units(3_000),
            sell: Money::from_minor_units(4_700),
            reorder: None,
            reviewed: Some((
                Money::from_minor_units(3_000),
                Money::from_minor_units(4_500),
            )),
        }),
        Intent::SetSupplierPrices {
            product: 3,
            rates: vec![(0, None), (2, Some(Money::from_minor_units(2_950)))],
        },
        Intent::SavePurchaseOrder(PurchaseOrderTerms {
            supplier: Some((None, "~".into())),
            date: None,
            note: "a=b c".into(),
            lines: vec![PurchaseLineTerms {
                item: "Soda, 1:2".into(),
                product: None,
                ordered: 10,
                rate: Money::from_minor_units(3_600),
            }],
        }),
        Intent::ReceivePurchaseOrder {
            po: 0,
            date: Some(day),
            today: day,
            lines: vec![(4, Money::from_minor_units(2_950))],
            remaining: vec![10],
        },
        Intent::ClosePurchaseOrder {
            po: 0,
            remaining: Vec::new(),
        },
        Intent::SaveCrate(CrateTerms {
            id: Some(1),
            name: "Party".into(),
            price: Money::from_minor_units(40_000),
            components: vec![(1, 2, false), (2, 1, true)],
        }),
        Intent::SaveOpening {
            customer: "batch1-market".into(),
            amount: Some(Money::from_minor_units(-500)),
            date: None,
        },
        Intent::SaveGodown {
            item: "Soda".into(),
            cases: " 9".into(),
            note: String::new(),
        },
        Intent::CountStock {
            product: 2,
            counted: 37,
            fridge: None,
            date: day,
            reason: Some("damage".into()),
            note: None,
            on_hand: 40,
        },
        Intent::SaveGstBill {
            bill: r#"{"id":2}"#.into(),
        },
        Intent::ImportGstBill {
            bill: None,
            deleted: Some(6),
        },
        Intent::DeleteProduct { id: 4 },
        Intent::AddSupplier { name: "X Y".into() },
        Intent::DeleteCrate { id: 1 },
        Intent::SetReorderDefault { level: 7 },
        Intent::RecordRestore,
        Intent::ImportLegacy(Box::new(alm_core::legacy::import::ImportPlan {
            digest: "ab".into(),
            version: "legacy-map-1".into(),
            records: vec![alm_core::legacy::import::PlannedRecord {
                kind: "customer".into(),
                legacy_id: "c 1,%".into(),
                body: serde_json::json!({"name": "X Y", "openingBalance": 150}),
                refs: vec![("product".into(), "p_1".into())],
                source: "/customers/0".into(),
            }],
            quarantine: vec![alm_core::legacy::import::PlannedQuarantine {
                kind: "payment".into(),
                legacy_id: "pay_9".into(),
                reason: "no legacy ID".into(),
                source: String::new(),
            }],
            source: "{\"products\": [1]}".into(),
        })),
        Intent::CancelGstBill { id: 2 },
        Intent::DeleteGstBill { id: 2 },
    ] {
        assert_eq!(Intent::parse(&intent.canonical()), Some(intent));
    }
    assert_eq!(Outcome::parse("created id=2"), Some(Outcome::Created(2)));
    assert_eq!(Intent::parse("cancel draft=7 extra=1"), None);
    assert_eq!(Intent::parse("cancel draft=07"), None);
    let outcome = Outcome::Sale(SaleReceipt {
        draft: 7,
        invoice: 3,
        quantity: 2,
        total: Money::from_minor_units(24_100),
    });
    assert_eq!(Outcome::parse(&outcome.canonical()), Some(outcome));
}
