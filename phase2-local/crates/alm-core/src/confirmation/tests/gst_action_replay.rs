use super::*;
use crate::DraftCustomerId;

const TARGET: GstBillId = GstBillId(31);
const CONTROL: GstBillId = GstBillId(32);
const LINE: [GstBillLineInput<'static>; 1] = [GstBillLineInput {
    item: "Synthetic Ordinary Goods",
    quantity: 2,
    inclusive_rate: Money::from_minor_units(11_800),
    gst_basis_points: 1_800,
    ..GstBillLineInput::BLANK
}];

fn read_invoice(
    fixture: &ConfirmationFixture,
    id: GstBillId,
) -> Result<GstBillInvoice, GstBillError> {
    let mut output = Vec::new();
    let result =
        fixture.emit_gst_bill(Role::Owner, id, &mut |invoice| output.push(invoice.clone()));
    match result {
        Ok(()) => {
            assert_eq!(output.len(), 1);
            Ok(output.pop().unwrap())
        }
        Err(error) => {
            assert!(output.is_empty());
            Err(error)
        }
    }
}

fn ordinary_readback(fixture: &ConfirmationFixture) -> ([Option<u32>; 3], CustomerLedger) {
    let stock = [1, 2, 3].map(|id| fixture.product_stock(ProductId::new(id)));
    let ledger = fixture
        .customer_ledger(Role::Owner, &DraftCustomerId::new("customer".into()))
        .unwrap();
    (stock, ledger)
}

#[derive(Debug)]
enum DeleteChoice {
    Decline,
    Confirm,
}

fn resolve_delete(fixture: &mut ConfirmationFixture, choice: DeleteChoice) {
    match choice {
        DeleteChoice::Decline => {}
        DeleteChoice::Confirm => {
            fixture.gst_bills.remove(&TARGET);
        }
    }
}

#[test]
fn l06_03_delete_decline_confirm_and_cancel_replay_independently() {
    let mut active = gst_invoice::fixture(3_000);
    for (id, number) in [(CONTROL, 699), (TARGET, 700)] {
        active
            .save_gst_bill(
                Role::Owner,
                &GstBillDraft {
                    id,
                    number: GstBillNumber(number),
                    customer: "Fixture buyer",
                    date: test_bill_date(),
                    gstin: None,
                    eway_number: None,
                    lines: &LINE,
                    ..GstBillDraft::default()
                },
                &mut |_| {},
            )
            .unwrap();
    }

    #[derive(Debug)]
    enum Action {
        Delete(DeleteChoice),
        CancelInvoice,
    }

    for action in [
        Action::Delete(DeleteChoice::Decline),
        Action::Delete(DeleteChoice::Confirm),
        Action::CancelInvoice,
    ] {
        let mut branch = active.clone();
        let target_before = read_invoice(&branch, TARGET).unwrap();
        assert_eq!(
            (target_before.record.number, target_before.record.cancelled),
            (GstBillNumber(700), false)
        );
        let control = read_invoice(&branch, CONTROL).unwrap();
        let before = ordinary_readback(&branch);
        assert_eq!(before.0, [Some(3), Some(4), Some(4)]);
        assert_eq!(before.1.closing.minor_units(), 34_200);

        let expected = match action {
            Action::Delete(DeleteChoice::Decline) => {
                resolve_delete(&mut branch, DeleteChoice::Decline);
                Ok(target_before.clone())
            }
            Action::Delete(DeleteChoice::Confirm) => {
                resolve_delete(&mut branch, DeleteChoice::Confirm);
                Err(GstBillError::NotFound)
            }
            Action::CancelInvoice => {
                branch.cancel_gst_bill(Role::Owner, TARGET).unwrap();
                let mut cancelled = target_before.clone();
                cancelled.record.cancelled = true;
                Ok(cancelled)
            }
        };
        let observed = read_invoice(&branch, TARGET);
        let after = ordinary_readback(&branch);
        assert_eq!(observed, expected);
        assert_eq!(read_invoice(&branch, CONTROL), Ok(control));
        assert_eq!(after, before);
        let next_suggestion = branch
            .gst_bills
            .values()
            .map(|bill| bill.number.0)
            .max()
            .unwrap()
            + 1;
        println!(
            "L06-03 {action:?}: target_before={target_before:?}; target_after={observed:?}; ordinary_before={before:?}; ordinary_after={after:?}; modeled next suggestion={next_suggestion}"
        );
    }
}
