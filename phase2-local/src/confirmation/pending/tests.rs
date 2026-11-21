use super::*;
use crate::BillDate;

#[test]
fn legacy_v1_snapshot_decodes_prices_for_each_line() {
    let review = decode("1|43|2|3|125|0,,|1,2;2,3;1,1").unwrap();
    assert_eq!(review.prices, vec![Money::from_minor_units(125); 3]);
    assert_eq!(review.draft.lines().len(), 3);
    assert_eq!(review.draft.bill_date(), None);
    assert_eq!(encode(&review), "2|43|2|3|125,125,125|0,,|1,2;2,3;1,1");
}

#[test]
fn versioned_snapshot_rejects_mismatched_line_prices() {
    assert!(decode("2|43|2|3|125,210|0,,|1,2;2,3;1,1").is_err());
}

#[test]
fn checkout_snapshot_rejects_invalid_unit_and_zero_pack() {
    let base = hex("3|43|2|3|125|2,61,62|1,12|100");
    assert!(decode(&format!("4|{base}|13|2,6,17,150")).is_err());
    assert!(decode(&format!("4|{base}|13|0,0,17,150")).is_err());
    assert!(decode(&format!("4|{base}|13|0,5,17,150")).is_err());
}

#[test]
fn snapshot_round_trips_customer_variants_and_ordered_lines() {
    for customer in [
        DraftCustomer::Unassigned,
        DraftCustomer::Typed("नाम | ' \0\n".into()),
        DraftCustomer::Selected {
            id: DraftCustomerId::new("id | \0".into()),
            name: "客户 ; ' \n".into(),
        },
    ] {
        let review = Review {
            draft: ProvisionalDraft::new(
                DraftId::new(u64::MAX),
                vec![
                    DraftLine::new(ProductId::new(987), NonZeroU32::new(13).unwrap()),
                    DraftLine::new(ProductId::new(12), NonZeroU32::new(7).unwrap()),
                ],
            )
            .with_customer(customer),
            draft_revision: u64::MAX,
            terms_revision: 39,
            prices: vec![Money::from_minor_units(223), Money::from_minor_units(456)],
            payment: ProposedPayment::Legacy,
            payment_mode: crate::confirmation::ReceiptMode::Cash,
            checkout: None,
            crate_packs: None,
        };
        assert_eq!(decode(&encode(&review)).unwrap(), review);
    }
}

#[test]
fn l03_03_v6_dates_round_trip_without_changing_the_undated_writer() {
    let named = "3|43|2|3|125|2,61,62|1,12|100";
    let checkout = format!("4|{}|13|0,6,17,150", hex(named));
    for payload in ["2|43|2|3|125|1,,62|1,12", named, &checkout] {
        let undated = decode(payload).unwrap();
        assert_eq!(undated.draft.bill_date(), None);
        assert_eq!(encode(&undated), payload);

        let envelope = format!("6|2024-02-29|{payload}");
        let dated = decode(&envelope).unwrap();
        let mut expected = undated;
        expected.draft = expected
            .draft
            .with_bill_date(Some(BillDate::new(2024, 2, 29).unwrap()));
        assert_eq!(dated, expected);
        assert_eq!(encode(&dated), envelope);
    }
}

#[test]
fn l03_03_v6_rejects_invalid_dates_and_nested_or_missing_payloads() {
    let payload = "2|43|2|3|125|1,,62|1,12";
    for snapshot in [
        format!("6||{payload}"),
        format!("6|2025-02-29|{payload}"),
        format!("6|2024-2-29|{payload}"),
        format!("6|2024-02-29|6|2024-02-29|{payload}"),
        "6|2024-02-29".into(),
    ] {
        assert!(matches!(
            decode(&snapshot),
            Err(DraftStorageError::InvalidOutput)
        ));
    }
}

#[test]
fn named_online_snapshot_round_trips_and_legacy_named_defaults_to_cash() {
    let payload = "6|2024-02-29|3|43|2|3|125|2,61,62|1,12|100";
    let cash = decode(payload).unwrap();
    assert_eq!(cash.payment_mode(), crate::confirmation::ReceiptMode::Cash);
    assert_eq!(encode(&cash), payload);
    let online = decode(&format!("8|{payload}")).unwrap();
    assert_eq!(
        online.payment_mode(),
        crate::confirmation::ReceiptMode::Online
    );
    assert_ne!(online, cash);
    assert_eq!(decode(&encode(&online)).unwrap(), online);
    for invalid in [
        "8|8|3|43|2|3|125|2,61,62|1,12|100",
        "8|2|43|2|3|125|1,,62|1,12",
        "8|3|43|2|3|125|1,,62|1,12|100",
        "8|9|anything",
    ] {
        assert!(decode(invalid).is_err());
    }
}

#[test]
fn online_snapshot_rejects_undated_and_invalid_named_payments() {
    for invalid in [
        "8|3|43|2|3|125|2,61,62|1,1|100",
        "8|6|2024-02-29|3|43|2|3|125|2,61,62|1,1|-1",
        "8|6|2024-02-29|3|43|2|3|125|2,61,62|1,1|126",
        "8|6|2024-02-29|3|43|2|3|125|2,,62|1,1|100",
        "8|6|2024-02-29|3|43|2|3|125|2,61,|1,1|100",
    ] {
        assert!(decode(invalid).is_err(), "accepted {invalid}");
    }
}
