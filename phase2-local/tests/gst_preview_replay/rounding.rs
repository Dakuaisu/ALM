use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, GstBillDraft, GstBillError, GstBillId,
    GstBillLineInput, GstBillNumber, GstBillTotals, GstJurisdiction,
};
use alm_phase2_local::{BillDate, DraftId, Money, ProductId, Role};

use super::artifacts::Sandbox;
use super::sink::{Action, Event, Sink};

const PRODUCT: ProductId = ProductId::new(1);
const BILL: GstBillId = GstBillId(91);
const OWNER: Role = Role::Owner;
const ITEM: GstBillLineInput<'static> = GstBillLineInput {
    item: "Synthetic Standalone 18%",
    quantity: 1,
    inclusive_rate: Money::from_minor_units(10_500),
    gst_basis_points: 1_800,
};

fn amounts(totals: GstBillTotals) -> [i64; 6] {
    [
        totals.taxable,
        totals.tax,
        totals.cgst,
        totals.sgst,
        totals.igst,
        totals.total,
    ]
    .map(Money::minor_units)
}

#[test]
fn l06_02_105_inclusive_standalone_save_readback_and_reprint() {
    let sandbox = Sandbox::new();
    for (count, expected, taxable, tax, split, grand) in [
        (
            1,
            [8898, 1602, 801, 801, 0, 10500],
            "88.98",
            "16.02",
            "8.01",
            "105.00",
        ),
        (
            2,
            [17797, 3203, 1602, 1602, 0, 21000],
            "177.97",
            "32.03",
            "16.02",
            "210.00",
        ),
    ] {
        let mut fixture = ConfirmationFixture::new(PRODUCT, 7, ITEM.inclusive_rate).unwrap();
        let lines = vec![ITEM; count];
        let date = BillDate::new(2025, 1, 2).unwrap();
        let draft = GstBillDraft {
            id: BILL,
            number: GstBillNumber(47),
            customer: "Synthetic Standalone Buyer",
            date: Some(date),
            gstin: None,
            eway_number: None,
            jurisdiction: GstJurisdiction::Intra,
            lines: &lines,
        };
        let before = fixture.open_stock_lots(OWNER, PRODUCT).unwrap();
        let movements_before = fixture
            .open_daily_movement_register(OWNER, date)
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row.product,
                    row.opening,
                    row.received,
                    row.dispatched,
                    row.adjustment,
                    row.closing,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fixture.save_gst_bill(Role::Manager, &draft, &mut |_| panic!("Manager save")),
            Err(GstBillError::AccessDenied)
        );
        assert_eq!(
            fixture.emit_gst_bill(OWNER, BILL, &mut |_| panic!("already saved")),
            Err(GstBillError::NotFound)
        );
        let form = fixture.gst_bill_form(OWNER, &draft).unwrap();
        assert_eq!(amounts(form), expected);
        let mut saved = Vec::new();
        fixture
            .save_gst_bill(OWNER, &draft, &mut |invoice| saved.push(invoice.clone()))
            .unwrap();
        let [saved] = saved.as_slice() else {
            panic!("one Save & View invoice expected")
        };
        assert_eq!(amounts(saved.totals), expected);
        assert_eq!(
            (
                saved.record.id,
                saved.record.number,
                saved.record.date,
                saved.record.cancelled
            ),
            (BILL, GstBillNumber(47), date, false)
        );
        assert_eq!(saved.record.lines.len(), count);
        for line in &saved.record.lines {
            assert_eq!(
                (
                    line.item.as_str(),
                    line.quantity.get(),
                    line.inclusive_rate,
                    line.gst_basis_points,
                    line.amount
                ),
                (
                    ITEM.item,
                    1,
                    ITEM.inclusive_rate,
                    1_800,
                    ITEM.inclusive_rate
                )
            );
        }

        let mut sink = Sink::default();
        assert_eq!(
            fixture.emit_gst_bill(Role::Manager, BILL, &mut |_| panic!("Manager reprint")),
            Err(GstBillError::AccessDenied)
        );
        for action in [Action::Print, Action::SavePdf] {
            fixture
                .emit_gst_bill(OWNER, BILL, &mut |stored| {
                    assert_eq!(stored, saved);
                    sink.invoke(action, stored, true);
                })
                .unwrap();
        }
        assert_eq!(
            sink.events,
            [
                Event::Render(Action::Print),
                Event::Open(Action::Print, "", "_p", "width=840,height=920"),
                Event::Write(Action::Print),
                Event::Close(Action::Print),
                Event::Render(Action::SavePdf),
                Event::Open(Action::SavePdf, "", "_p", "width=840,height=920"),
                Event::Write(Action::SavePdf),
                Event::Close(Action::SavePdf),
            ]
        );
        assert_eq!(sink.documents[0].html, sink.documents[1].html);
        let doc = sandbox.readback(count, Action::Print, &sink.documents[0].html);
        assert_eq!(
            doc.matches("text-align:right\">88.98</td>").count(),
            count * 2
        );
        assert!(doc.contains(&format!("<tr><td>{taxable}</td><td>{tax}</td></tr>")));
        for kind in ["CGST", "SGST"] {
            assert!(doc.contains(&format!(">{kind}</td><td style=\"padding:3px 7px;border-bottom:1px solid #eaedf1;text-align:right\">{split}</td>")));
        }
        assert!(doc.contains(&format!(">&#8377; {grand}</td>")));
        assert!(!doc.contains("Round Off"));
        assert_eq!(fixture.open_stock_lots(OWNER, PRODUCT).unwrap(), before);
        assert!(
            fixture
                .open_daily_movement_register(OWNER, date)
                .unwrap()
                .iter()
                .map(|row| (
                    row.product,
                    row.opening,
                    row.received,
                    row.dispatched,
                    row.adjustment,
                    row.closing
                ))
                .eq(movements_before)
        );
        assert_eq!(fixture.posted_count(), 0);
        assert_eq!(fixture.ledger().minor_units(), 0);
        assert_eq!(fixture.received_ledger().minor_units(), 0);
        assert_eq!(fixture.customer_due(draft.customer).minor_units(), 0);
        assert_eq!(
            fixture.current_sale(OWNER, DraftId::new(BILL.0)),
            Err(ConfirmationError::DraftNotFound)
        );
        println!(
            "L06-02 standalone 105 at 18%, rows={count}: per-line raw taxable ₹5250/59, raw tax ₹945/59, raw half ₹945/118; [taxable, tax, CGST, SGST, IGST, total] source={expected:?}, saved/reopened={:?}; print/PDF equal, no ordinary posting",
            amounts(saved.totals)
        );
    }
}
