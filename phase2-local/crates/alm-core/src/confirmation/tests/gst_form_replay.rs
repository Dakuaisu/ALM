use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Form {
    buyer_state: String,
    buyer_code: String,
    pos_state: String,
    pos_code: String,
    gstin: String,
    buyer_editable: bool,
    pos_editable: bool,
    gstin_editable: bool,
    length_warning: Option<usize>,
}

impl Form {
    fn reopen(stored_gstin: &str) -> Self {
        Self {
            buyer_state: "Kerala".into(),
            buyer_code: "32".into(),
            pos_state: "Goa".into(),
            pos_code: "30".into(),
            gstin: stored_gstin.into(),
            buyer_editable: recognized(stored_gstin.trim()).is_none(),
            pos_editable: true,
            gstin_editable: true,
            length_warning: length_warning(stored_gstin),
        }
    }

    fn input(&mut self, value: &str) {
        self.gstin = value.to_uppercase();
        let trimmed = self.gstin.trim();
        self.length_warning = length_warning(trimmed);
        if let Some((state, code)) = recognized(trimmed) {
            self.buyer_state = state.into();
            self.buyer_code = code.into();
            self.pos_state = self.buyer_state.clone();
            self.pos_code = self.buyer_code.clone();
            self.buyer_editable = false;
        } else {
            self.buyer_editable = true;
        }
    }
}

fn length_warning(value: &str) -> Option<usize> {
    let length = value.encode_utf16().count();
    (length != 0 && length != 15).then_some(length)
}

// This replay models only the recognized prefix used by its synthetic cases.
fn recognized(value: &str) -> Option<(&'static str, &'static str)> {
    value.starts_with("27").then_some(("Maharashtra", "27"))
}

fn invoice_draft<'a>(gstin: Option<&'a str>) -> GstBillDraft<'a> {
    const LINES: &[GstBillLineInput<'static>] = &[GstBillLineInput {
        item: "Replay item",
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_000),
        gst_basis_points: 0,
        ..GstBillLineInput::BLANK
    }];
    GstBillDraft {
        id: GstBillId(61),
        number: GstBillNumber(761),
        customer: "Replay buyer",
        date: test_bill_date(),
        gstin,
        eway_number: None,
        pos_state: "Maharashtra",
        pos_code: "27",
        lines: LINES,
        ..GstBillDraft::default()
    }
}

#[test]
fn l06_06_populated_reopen_keeps_saved_buyer_distinct_from_pos() {
    let reopened = Form::reopen("27ABCDE1234F1Z5");
    assert_eq!(
        (reopened.buyer_state.as_str(), reopened.buyer_code.as_str()),
        ("Kerala", "32")
    );
    assert_eq!(
        (reopened.pos_state.as_str(), reopened.pos_code.as_str()),
        ("Goa", "30")
    );
    assert_eq!(reopened.gstin, "27ABCDE1234F1Z5");
    assert!(!reopened.buyer_editable && reopened.pos_editable && reopened.gstin_editable);
    assert_eq!(reopened.length_warning, None);
    println!("L06-06 populated reopen without input: {reopened:?}");
    let padded = Form::reopen(" 27abcde1234f1z5 ");
    assert_eq!(
        padded,
        Form {
            gstin: " 27abcde1234f1z5 ".into(),
            length_warning: Some(17),
            ..reopened
        }
    );
    println!("L06-06 padded reopen without input: {padded:?}");
}

#[test]
fn l06_06_lowercase_input_infers_recognized_prefix_and_tracks_length() {
    for (input, expected_gstin, warning) in [
        (" 27abcde1234f1z5 ", " 27ABCDE1234F1Z5 ", None),
        (" 27abcde1234f1z ", " 27ABCDE1234F1Z ", Some(14)),
    ] {
        let mut form = Form::reopen("");
        form.input(input);
        assert_eq!(form.gstin, expected_gstin);
        assert_eq!(
            (form.buyer_state.as_str(), form.buyer_code.as_str()),
            ("Maharashtra", "27")
        );
        assert_eq!(
            (form.pos_state.as_str(), form.pos_code.as_str()),
            ("Maharashtra", "27")
        );
        assert!(!form.buyer_editable && form.pos_editable && form.gstin_editable);
        assert_eq!(form.length_warning, warning);
        println!("L06-06 normalized recognized input {input:?}: {form:?}");
    }
}

#[test]
fn l06_06_unknown_prefix_stays_editable_and_correct_length_round_trips() {
    let mut form = Form::reopen("27ABCDE1234F1Z5");
    form.input(" 88abcde1234f1z5 ");
    assert_eq!(form.gstin, " 88ABCDE1234F1Z5 ");
    assert_eq!(
        (form.buyer_state.as_str(), form.buyer_code.as_str()),
        ("Kerala", "32")
    );
    assert_eq!(
        (form.pos_state.as_str(), form.pos_code.as_str()),
        ("Goa", "30")
    );
    assert!(form.buyer_editable && form.pos_editable && form.gstin_editable);
    assert_eq!(form.length_warning, None);

    let mut fixture =
        ConfirmationFixture::new(ProductId::new(61), 0, Money::from_minor_units(0)).unwrap();
    let draft = invoice_draft(Some(&form.gstin));
    let mut saved = Vec::new();
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |invoice| {
            saved.push(invoice.clone())
        })
        .unwrap();
    let mut readback = Vec::new();
    fixture
        .emit_gst_bill(Role::Owner, GstBillId(61), &mut |invoice| {
            readback.push(invoice.clone())
        })
        .unwrap();
    assert_eq!(readback, saved);
    assert_eq!(
        readback[0].record.gstin.as_deref(),
        Some(" 88ABCDE1234F1Z5 ")
    );
    assert_eq!(readback[0].record.jurisdiction(), GstJurisdiction::Inter);
    println!(
        "L06-06 unknown-prefix save/readback: {form:?}; {:?}",
        readback[0].record
    );
}

#[test]
fn l06_06_clear_after_inference_unlocks_buyer_without_clearing_state() {
    let mut form = Form::reopen("");
    form.input("27ABCDE1234F1Z5");
    form.input("");
    assert!(form.gstin.is_empty());
    assert_eq!(
        (form.buyer_state.as_str(), form.buyer_code.as_str()),
        ("Maharashtra", "27")
    );
    assert_eq!(
        (form.pos_state.as_str(), form.pos_code.as_str()),
        ("Maharashtra", "27")
    );
    assert!(form.buyer_editable && form.pos_editable && form.gstin_editable);
    assert_eq!(form.length_warning, None);
    println!("L06-06 clear after inference: {form:?}");
}

#[test]
fn l06_06_wrong_length_inferred_gstin_is_rejected_on_save() {
    let mut form = Form::reopen("");
    form.input("27abcde1234f1z");
    assert_eq!(form.length_warning, Some(14));
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(61), 0, Money::from_minor_units(0)).unwrap();
    let result = fixture.save_gst_bill(Role::Owner, &invoice_draft(Some(&form.gstin)), &mut |_| {
        panic!("invalid GSTIN emitted")
    });
    assert_eq!(result, Err(GstBillError::InvalidGstin));
    println!("L06-06 wrong-length input/save: {form:?}; {result:?}");
}
