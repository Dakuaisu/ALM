//! Source-derived replay only: this is not a browser or a production modal implementation.
use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    ConfirmationFixture, CustomerLedgerKind, CustomerReceiptInput, ReceiptMode, RequestId,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const SALE: DraftId = DraftId::new(1);
const PRODUCT: ProductId = ProductId::new(1);

#[path = "l18_03/model.rs"]
mod model;
use model::{BodyEvent, Dismiss, FormKind, Layer, Replay};
#[path = "l18_03/snapshot.rs"]
mod snapshot;
use snapshot::unchanged;
#[path = "l18_03/privacy.rs"]
mod privacy;

fn fixture() -> (ConfirmationFixture, DraftCustomerId, ProvisionalDraft) {
    let buyer = DraftCustomerId::new("l18-03-buyer".into());
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, Money::from_minor_units(500)).unwrap();
    let sale = ProvisionalDraft::new(
        SALE,
        vec![DraftLine::new(PRODUCT, NonZeroU32::new(2).unwrap())],
    )
    .with_customer(DraftCustomer::Selected {
        id: buyer.clone(),
        name: "Synthetic buyer".into(),
    })
    .with_bill_date(Some(BillDate::new(2025, 2, 21).unwrap()));
    fixture.register_draft(sale).unwrap();
    let review = fixture
        .review_named(Role::Owner, SALE, Money::from_minor_units(200))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: buyer.clone(),
                date: BillDate::new(2025, 2, 22).unwrap(),
                amount: Money::from_minor_units(125),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
    assert!(matches!(
        fixture
            .customer_ledger(Role::Owner, &buyer)
            .unwrap()
            .entries[1]
            .kind,
        CustomerLedgerKind::Receipt {
            mode: ReceiptMode::Online
        }
    ));
    let new_bill = ProvisionalDraft::new(
        DraftId::new(2),
        vec![DraftLine::new(PRODUCT, NonZeroU32::new(1).unwrap())],
    );
    fixture.register_draft(new_bill.clone()).unwrap();
    (fixture, buyer, new_bill)
}

#[test]
fn clean_and_dirty_dismissal_matrix_preserves_synthetic_accounts() {
    for kind in [FormKind::EditBill, FormKind::ReceivePayment] {
        for dirty in [false, true] {
            for how in [
                Dismiss::Header,
                Dismiss::Escape,
                Dismiss::Overlay,
                Dismiss::Inside,
            ] {
                let (mut fixture, buyer, new_bill) = fixture();
                let mut replay = Replay::new(kind);
                if dirty {
                    let Layer::Form(form) = &mut replay.layers[1] else {
                        panic!("form missing")
                    };
                    form.customer = "Unsaved buyer".into();
                    form.amount = "321".into();
                    replay.event(BodyEvent::Input);
                }
                let original = replay.form().clone();
                unchanged(&mut fixture, &buyer, |_| replay.dismiss(how));
                match how {
                    Dismiss::Header | Dismiss::KeepEditing | Dismiss::Cancel => {
                        assert_eq!(replay.layers, [Layer::Parent])
                    }
                    Dismiss::Inside => {
                        assert_eq!(replay.layers, [Layer::Parent, Layer::Form(original)])
                    }
                    Dismiss::Escape | Dismiss::Overlay if dirty => assert_eq!(
                        replay.layers,
                        [Layer::Parent, Layer::Form(original), Layer::Discard]
                    ),
                    Dismiss::Escape | Dismiss::Overlay => {
                        assert_eq!(replay.layers, [Layer::Parent])
                    }
                }
                assert_eq!(
                    fixture.review(Role::Owner, new_bill.id()).unwrap().draft(),
                    &new_bill
                );
                println!("L18-03 {kind:?} dirty={dirty} {how:?}: {:?}", replay.layers);
            }
        }
    }
}

#[test]
fn child_dismissal_reveals_same_dirty_form_and_repeated_guard() {
    for kind in [FormKind::EditBill, FormKind::ReceivePayment] {
        for child_close in [
            Dismiss::KeepEditing,
            Dismiss::Header,
            Dismiss::Escape,
            Dismiss::Overlay,
        ] {
            for parent_close in [Dismiss::Escape, Dismiss::Overlay] {
                let (mut fixture, buyer, _) = fixture();
                let mut replay = Replay::new(kind);
                let Layer::Form(form) = &mut replay.layers[1] else {
                    panic!("form missing")
                };
                form.customer = "Unsaved buyer".into();
                form.amount = "321".into();
                replay.event(BodyEvent::Input);
                let original = replay.form().clone();
                unchanged(&mut fixture, &buyer, |_| {
                    replay.dismiss(parent_close);
                    assert_eq!(
                        replay.layers,
                        [Layer::Parent, Layer::Form(original.clone()), Layer::Discard]
                    );
                });
                unchanged(&mut fixture, &buyer, |_| {
                    replay.dismiss(child_close);
                    assert_eq!(
                        replay.layers,
                        [Layer::Parent, Layer::Form(original.clone())]
                    );
                });
                unchanged(&mut fixture, &buyer, |_| {
                    replay.dismiss(parent_close);
                    assert_eq!(
                        replay.layers,
                        [Layer::Parent, Layer::Form(original.clone()), Layer::Discard]
                    );
                });
                unchanged(&mut fixture, &buyer, |_| {
                    replay.discard();
                    assert_eq!(replay.layers, [Layer::Parent]);
                });
                println!(
                    "L18-03 {kind:?} {parent_close:?} child {child_close:?}: same dirty form; repeated discard"
                );
            }
        }
    }
}

#[test]
fn body_events_and_footer_paths_follow_guard_scope() {
    for kind in [FormKind::EditBill, FormKind::ReceivePayment] {
        for event in [
            BodyEvent::Input,
            BodyEvent::Change,
            BodyEvent::Button,
            BodyEvent::ButtonChild,
            BodyEvent::Plain,
            BodyEvent::Footer,
        ] {
            let mut replay = Replay::new(kind);
            replay.event(event);
            assert_eq!(
                replay.form().dirty,
                matches!(
                    event,
                    BodyEvent::Input
                        | BodyEvent::Change
                        | BodyEvent::Button
                        | BodyEvent::ButtonChild
                )
            );
        }
        let mut replay = Replay::new(kind);
        let original = replay.form().amount.clone();
        let Layer::Form(form) = &mut replay.layers[1] else {
            panic!("form missing")
        };
        form.amount = "changed".into();
        replay.event(BodyEvent::Input);
        let Layer::Form(form) = &mut replay.layers[1] else {
            panic!("form missing")
        };
        form.amount = original;
        replay.event(BodyEvent::Change);
        assert!(replay.form().dirty);
    }
    // Edit Bill footer Cancel is direct; a failed payment footer validation is not a body event.
    let mut edit = Replay::new(FormKind::EditBill);
    edit.event(BodyEvent::Input);
    edit.dismiss(Dismiss::Cancel);
    assert_eq!(edit.layers, [Layer::Parent]);
    let (mut fixture, buyer, _) = fixture();
    let mut payment = Replay::new(FormKind::ReceivePayment);
    unchanged(&mut fixture, &buyer, |_| {
        payment.event(BodyEvent::Footer);
        payment.dismiss(Dismiss::Escape);
    });
    assert_eq!(payment.layers, [Layer::Parent]);
}

#[test]
fn regular_confirmation_cancel_and_yes_are_single_pop_helpers() {
    for kind in [FormKind::EditBill, FormKind::ReceivePayment] {
        let (mut fixture, buyer, _) = fixture();
        let mut replay = Replay::new(kind);
        replay.event(BodyEvent::Input);
        let original = replay.form().clone();
        let mut called = false;
        unchanged(&mut fixture, &buyer, |_| {
            replay.layers.push(Layer::Confirm);
            replay.dismiss(Dismiss::Cancel);
            assert!(!called);
            assert_eq!(
                replay.layers,
                [Layer::Parent, Layer::Form(original.clone())]
            );
        });
        unchanged(&mut fixture, &buyer, |_| {
            replay.layers.push(Layer::Confirm);
            replay.confirm_yes(|state| {
                called = true;
                assert_eq!(state.layers, [Layer::Parent, Layer::Form(original)]);
            });
        });
        assert!(called);
    }
}

#[test]
fn top_level_forms_close_the_overlay_when_the_last_layer_is_popped() {
    for kind in [FormKind::EditBill, FormKind::ReceivePayment] {
        for how in [Dismiss::Header, Dismiss::Escape, Dismiss::Overlay] {
            let (mut fixture, buyer, _) = fixture();
            let mut replay = Replay::new(kind);
            replay.layers.remove(0);
            unchanged(&mut fixture, &buyer, |_| replay.dismiss(how));
            assert!(replay.layers.is_empty());
        }
    }
}
