use alm_phase2_local::synthetic_read_model::{
    AccountId, AccountScope, BusinessProfile, Error, ReceiptId, SaleId, Store,
};
use alm_phase2_local::{BillDate, Role};

pub fn scope() -> AccountScope {
    let account = AccountId::new(super::fixture::BUYER_ID.into());
    AccountScope {
        sales: vec![
            (SaleId::new("sale-cancelled-id".into()), account.clone()),
            (
                SaleId::new("sale-active-id".into()),
                AccountId::new("control-account".into()),
            ),
        ],
        receipts: vec![(
            ReceiptId::new("receipt-standalone-id".into()),
            account.clone(),
        )],
        zero_openings: vec![account, AccountId::new("control-account".into())],
    }
}

pub fn save_profile(store: &mut Store) {
    let profile = BusinessProfile {
        fssai: "A<&>".into(),
        gstin: "Hidden & GST".into(),
        show_fssai: Some(true),
        show_gstin: Some(false),
        ..BusinessProfile::default()
    };
    store
        .save_business_profile(Role::Owner, &profile, 4, "local")
        .unwrap();
}

pub fn document(store: &Store, path: &std::path::Path) -> String {
    let disk_before = std::fs::read(path).unwrap();
    let before = store.backup(Role::Owner).unwrap();
    let date = BillDate::new(2025, 3, 1).unwrap();
    assert!(matches!(
        store.estimate_html(
            Role::Manager,
            &SaleId::new("sale-cancelled-id".into()),
            date
        ),
        Err(Error::AccessDenied)
    ));
    assert!(matches!(
        store.backup(Role::Manager),
        Err(Error::AccessDenied)
    ));
    let html = store
        .estimate_html(Role::Owner, &SaleId::new("sale-cancelled-id".into()), date)
        .unwrap();
    for anchor in [
        "<h1>Estimate #1</h1><p>Date: 21-Feb-2025</p>",
        "<p>CANCELLED</p>",
        "<tr><td>Synthetic A</td><td>2</td><td>₹177.00</td><td>₹354.00</td></tr>",
        "<tr><td>Synthetic B</td><td>1</td><td>₹56.00</td><td>₹56.00</td></tr>",
        "<tr><th>Bill Amount</th><td>₹410.00</td></tr>",
        "<tr><th>Paid on this bill</th><td>₹100.00</td></tr>",
        "<tr><th>Total Due (as on 01-Mar-2025)</th><td>₹-50.00</td></tr>",
        "FSSAI: A&lt;&amp;&gt;",
    ] {
        assert!(html.contains(anchor), "missing {anchor}: {html}");
    }
    assert!(!html.contains("Hidden"));
    assert_eq!(store.backup(Role::Owner).unwrap(), before);
    assert_eq!(std::fs::read(path).unwrap(), disk_before);
    html
}

pub fn controls(store: &Store, dir: &std::path::Path) {
    let mut missing = scope();
    missing.receipts.clear();
    let missing = Store::create_scoped(
        &dir.join("missing.json"),
        Role::Owner,
        store.records(Role::Owner).unwrap().clone(),
        missing,
    );
    assert!(matches!(missing, Err(Error::InvalidArchive)));
    let mut records = store.records(Role::Owner).unwrap().clone();
    records.receipts[0].amount_minor = 7_000;
    let mut control =
        Store::create_scoped(&dir.join("control.json"), Role::Owner, records, scope()).unwrap();
    let mut profile = store.business_profile(Role::Owner).unwrap().clone();
    profile.show_fssai = Some(false);
    profile.show_gstin = Some(true);
    control
        .save_business_profile(Role::Owner, &profile, 5, "control")
        .unwrap();
    let html = control
        .estimate_html(
            Role::Owner,
            &SaleId::new("sale-cancelled-id".into()),
            BillDate::new(2025, 3, 1).unwrap(),
        )
        .unwrap();
    assert!(html.contains("<tr><th>Total Due (as on 01-Mar-2025)</th><td>₹-70.00</td></tr>"));
    assert!(!html.contains("FSSAI: A&lt;&amp;&gt;"));
    assert!(html.contains("GSTIN: Hidden &amp; GST"));

    let records = store.records(Role::Owner).unwrap().clone();
    let mut linked = scope();
    linked.sales[1].1 = AccountId::new(super::fixture::BUYER_ID.into());
    linked.zero_openings.pop();
    let linked = Store::create_scoped(
        &dir.join("linked.json"),
        Role::Owner,
        records.clone(),
        linked,
    )
    .unwrap();
    let date = BillDate::new(2025, 3, 1).unwrap();
    let id = SaleId::new("sale-cancelled-id".into());
    assert!(
        linked
            .estimate_html(Role::Owner, &id, date)
            .unwrap()
            .contains("<tr><th>Total Due (as on 01-Mar-2025)</th><td>₹-42.00</td></tr>")
    );

    let unscoped = Store::create(&dir.join("v1.json"), Role::Owner, records.clone()).unwrap();
    let v1 = unscoped.backup(Role::Owner).unwrap();
    drop(unscoped);
    let reopened_v1 = Store::open(&dir.join("v1.json"), Role::Owner).unwrap();
    assert_eq!(reopened_v1.backup(Role::Owner).unwrap(), v1);
    assert!(reopened_v1.estimate_html(Role::Owner, &id, date).is_err());

    let mut overflow = records.clone();
    overflow.receipts[0].amount_minor = i64::MIN;
    let overflow =
        Store::create_scoped(&dir.join("overflow.json"), Role::Owner, overflow, scope()).unwrap();
    assert!(overflow.estimate_html(Role::Owner, &id, date).is_err());

    let mut fractional = records.clone();
    fractional.receipts[0].amount_minor += 1;
    let fractional = Store::create_scoped(
        &dir.join("fractional.json"),
        Role::Owner,
        fractional,
        scope(),
    )
    .unwrap();
    assert!(fractional.estimate_html(Role::Owner, &id, date).is_err());

    let mut no_opening = scope();
    no_opening.zero_openings.clear();
    let no_opening = Store::create_scoped(
        &dir.join("no-opening.json"),
        Role::Owner,
        records.clone(),
        no_opening,
    );
    assert!(matches!(no_opening, Err(Error::InvalidArchive)));

    let mut unsupported: serde_json::Value =
        serde_json::from_slice(&store.backup(Role::Owner).unwrap()).unwrap();
    unsupported["scope"]["zero_openings"][0] =
        serde_json::json!({"account": super::fixture::BUYER_ID, "opening_minor": 100});
    assert!(
        Store::restore(
            &dir.join("nonzero-opening.json"),
            Role::Owner,
            &serde_json::to_vec(&unsupported).unwrap()
        )
        .is_err()
    );

    let saved: serde_json::Value =
        serde_json::from_slice(&store.backup(Role::Owner).unwrap()).unwrap();
    let mut missing_receipt = saved.clone();
    missing_receipt["scope"]["receipts"] = serde_json::json!([]);
    let mut malformed = saved;
    malformed["scope"]["sales"][0][1] = serde_json::json!(" ");
    for (name, archive) in [
        ("missing-receipt", missing_receipt),
        ("malformed", malformed),
    ] {
        let path = dir.join(format!("{name}-restore.json"));
        assert!(
            Store::restore(&path, Role::Owner, &serde_json::to_vec(&archive).unwrap()).is_err()
        );
        assert!(!path.exists());
    }

    let mut duplicate = scope();
    duplicate.receipts.push(duplicate.receipts[0].clone());
    let duplicate = Store::create_scoped(
        &dir.join("duplicate.json"),
        Role::Owner,
        records.clone(),
        duplicate,
    );
    assert!(matches!(duplicate, Err(Error::InvalidArchive)));

    let mut dangling = scope();
    dangling.sales[1].0 = SaleId::new("missing-sale".into());
    let dangling = Store::create_scoped(
        &dir.join("dangling.json"),
        Role::Owner,
        records.clone(),
        dangling,
    );
    assert!(matches!(dangling, Err(Error::InvalidArchive)));

    let mut missing_sale = scope();
    missing_sale.sales.pop();
    let mut dangling_receipt = scope();
    dangling_receipt.receipts[0].0 = ReceiptId::new("missing-receipt".into());
    let mut repeated_opening = scope();
    repeated_opening
        .zero_openings
        .push(repeated_opening.zero_openings[0].clone());
    let mut repeated_sale = scope();
    repeated_sale.sales.push(repeated_sale.sales[0].clone());
    for (name, invalid_scope) in [
        ("missing-sale", missing_sale),
        ("dangling-receipt", dangling_receipt),
        ("repeated-opening", repeated_opening),
        ("repeated-sale", repeated_sale),
    ] {
        assert!(matches!(
            Store::create_scoped(
                &dir.join(format!("{name}.json")),
                Role::Owner,
                records.clone(),
                invalid_scope
            ),
            Err(Error::InvalidArchive)
        ));
    }

    let mut invalid = records;
    invalid.sales[0].total_minor += 1;
    let invalid =
        Store::create_scoped(&dir.join("invalid.json"), Role::Owner, invalid, scope()).unwrap();
    assert!(invalid.estimate_html(Role::Owner, &id, date).is_err());
}
