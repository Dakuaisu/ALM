// allow: SIZE_OK - One shared fixture drives the explicit failure and recovery matrix.
use alm_phase2_local::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, CustomerReceiptInput, EstimateDisplay,
    EstimateLineDisplay, EstimateRequest, ReceiptMode, RequestId,
};
use alm_phase2_local::preferences::{Preference, Preferences, Theme};
use alm_phase2_local::synthetic_read_model::{BusinessProfile, Customer, Records, Store};
use alm_phase2_local::{
    AccessDenied, BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, LocalSales, Money,
    ProductId, ProvisionalDraft, Role,
};

fn projection(preferences: &Preferences) -> (&'static str, Option<Money>, BalanceMode) {
    (
        preferences.theme().data_theme(),
        preferences.present_money(Money::from_minor_units(12345)),
        preferences.balance_mode(),
    )
}
use std::{
    fs,
    num::NonZeroU32,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn preference_failures_replay_without_changing_business_or_permissions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = root.join(format!("preference-replay-{}-{nonce}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let keys = dir.join("preferences");
    fs::create_dir(&keys).unwrap();
    let records = Records {
        reorder_default: 9,
        products: vec![],
        customers: vec![Customer {
            name: "Replay Buyer".into(),
            mobile: String::new(),
            address: String::new(),
            balance_minor: 12345,
        }],
        sales: vec![],
        receipts: vec![],
    };
    let archive = dir.join("business.json");
    let mut store = Store::create(&archive, Role::Owner, records.clone()).unwrap();
    let profile = BusinessProfile {
        fssai: "fixture-license".into(),
        ..BusinessProfile::default()
    };
    store
        .save_business_profile(Role::Owner, &profile, 1, "fixture")
        .unwrap();
    let saved_profile = store.business_profile(Role::Owner).unwrap().clone();
    let archive_bytes = fs::read(&archive).unwrap();

    let mut preferences = Preferences::load(&keys);
    assert_eq!(preferences.theme(), Theme::Light);
    assert_eq!(preferences.theme().data_theme(), "light");
    assert_eq!(preferences.balance_mode(), BalanceMode::AsOnBillDate);
    assert_eq!(
        preferences.present_money(Money::from_minor_units(12345)),
        Some(Money::from_minor_units(12345))
    );
    for value in [
        Preference::Theme(Theme::Dark),
        Preference::Privacy(true),
        Preference::Balance(BalanceMode::Latest),
    ] {
        preferences.set(value);
    }
    let loaded = Preferences::load(&keys);
    assert_eq!(loaded.theme(), Theme::Dark);
    assert_eq!(loaded.theme().data_theme(), "dark");
    assert_eq!(loaded.present_money(Money::from_minor_units(12345)), None);
    assert_eq!(loaded.balance_mode(), BalanceMode::Latest);
    assert_eq!(fs::read(&archive).unwrap(), archive_bytes);

    let product = ProductId::new(17);
    let sale = DraftId::new(31);
    let buyer = DraftCustomerId::new("replay-buyer".into());
    let date = BillDate::new(2025, 2, 21).unwrap();
    let today = BillDate::new(2025, 3, 1).unwrap();
    let draft = ProvisionalDraft::new(
        sale,
        vec![DraftLine::new(product, NonZeroU32::new(2).unwrap())],
    )
    .with_bill_date(Some(date))
    .with_customer(DraftCustomer::Selected {
        id: buyer.clone(),
        name: "Replay Buyer".into(),
    });
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product,
        &[(3, Money::from_minor_units(1000))],
        Money::from_minor_units(12500),
    )
    .unwrap();
    fixture.register_draft(draft.clone()).unwrap();
    let review = fixture
        .review_named(Role::Owner, sale, Money::from_minor_units(10000))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: buyer,
                date: today,
                amount: Money::from_minor_units(6000),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    let lines = [EstimateLineDisplay {
        product,
        label: "Replay Cola",
    }];
    let render = |mode| {
        let mut docs = Vec::new();
        fixture
            .print_estimate(
                EstimateRequest {
                    role: Role::Owner,
                    draft_id: sale,
                    balance_mode: mode,
                    display: EstimateDisplay {
                        today,
                        lines: &lines,
                    },
                },
                &mut |doc| docs.push(doc),
            )
            .unwrap();
        docs.remove(0).html
    };
    let balance_row = |mode| match mode {
        BalanceMode::AsOnBillDate => "<th>Balance Due</th><td>₹150.00</td>",
        BalanceMode::Latest => "<th>Total Due (as on 01-Mar-2025)</th><td>₹90.00</td>",
    };
    let saved_html = render(loaded.balance_mode());
    assert!(saved_html.contains("<p>Bill To: Replay Buyer</p>"));
    assert!(saved_html.contains(balance_row(BalanceMode::Latest)));

    let visible = Some(Money::from_minor_units(12345));
    let defaults = ("light", visible, BalanceMode::AsOnBillDate);
    let saved = ("dark", None, BalanceMode::Latest);
    for (saved_baseline, cases) in [
        (
            true,
            [
                (
                    "theme",
                    Preference::Theme(Theme::Light),
                    ("light", None, BalanceMode::Latest),
                ),
                (
                    "privacy",
                    Preference::Privacy(false),
                    ("dark", visible, BalanceMode::Latest),
                ),
                (
                    "balance",
                    Preference::Balance(BalanceMode::AsOnBillDate),
                    ("dark", None, BalanceMode::AsOnBillDate),
                ),
            ],
        ),
        (
            false,
            [
                (
                    "theme",
                    Preference::Theme(Theme::Dark),
                    ("dark", visible, BalanceMode::AsOnBillDate),
                ),
                (
                    "privacy",
                    Preference::Privacy(true),
                    ("light", None, BalanceMode::AsOnBillDate),
                ),
                (
                    "balance",
                    Preference::Balance(BalanceMode::Latest),
                    ("light", visible, BalanceMode::Latest),
                ),
            ],
        ),
    ] {
        for (key, failed, expected_current) in cases {
            let case_dir = dir.join(format!("case-{saved_baseline}-{key}"));
            fs::create_dir(&case_dir).unwrap();
            if saved_baseline {
                for name in ["theme", "privacy", "balance"] {
                    fs::copy(keys.join(name), case_dir.join(name)).unwrap();
                }
            }
            let mut session = Preferences::load(&case_dir);
            let expected_reopened = if saved_baseline { saved } else { defaults };
            assert_eq!(projection(&session), expected_reopened);
            fs::create_dir(case_dir.join(format!("{key}.stage"))).unwrap();
            session.set(failed);
            let current = projection(&session);
            assert_eq!(current, expected_current);
            let current_row = balance_row(expected_current.2);
            assert!(render(current.2).contains(current_row));
            fs::remove_dir(case_dir.join(format!("{key}.stage"))).unwrap();
            let reopened = projection(&Preferences::load(&case_dir));
            assert_eq!(reopened, expected_reopened);
            let reopened_row = balance_row(expected_reopened.2);
            assert!(render(reopened.2).contains(reopened_row));
            println!(
                "write {key} saved={saved_baseline}: current expected={expected_current:?} observed={current:?} {current_row}; reopened expected={expected_reopened:?} observed={reopened:?} {reopened_row}"
            );
        }
    }
    assert!(
        render(BalanceMode::AsOnBillDate)
            .contains("<td>Replay Cola</td><td>2</td><td>₹125.00</td><td>₹250.00</td>")
    );
    let mut local = LocalSales::new(Money::from_minor_units(500));
    for privacy in [true, false] {
        preferences.set(Preference::Privacy(privacy));
        assert_eq!(
            preferences.present_money(Money::from_minor_units(12345)),
            if privacy { None } else { visible }
        );
        assert_eq!(
            fixture.review_named(Role::Manager, sale, Money::from_minor_units(0)),
            Err(ConfirmationError::AccessDenied)
        );
        assert!(matches!(
            store.backup(Role::Manager),
            Err(alm_phase2_local::synthetic_read_model::Error::AccessDenied)
        ));
        assert_eq!(
            local.session(Role::Manager).total_profit(),
            Err(AccessDenied)
        );
        local
            .session(Role::Manager)
            .save_draft(draft.clone())
            .unwrap();
        assert_eq!(
            local.session(Role::Manager).reopen_draft(sale).unwrap(),
            Some(draft.clone())
        );
        println!("privacy={privacy}: expected/observed manager denial and provisional save/reopen");
    }
    preferences.set(Preference::Privacy(true));

    for key in ["theme", "privacy", "balance"] {
        fs::rename(keys.join(key), keys.join(format!("{key}.saved"))).unwrap();
        fs::create_dir(keys.join(key)).unwrap();
        assert!(fs::read(keys.join(key)).is_err());
    }
    let failed_reads = Preferences::load(&keys);
    assert_eq!(failed_reads.theme(), Theme::Light);
    assert_eq!(
        failed_reads.present_money(Money::from_minor_units(12345)),
        Some(Money::from_minor_units(12345))
    );
    assert_eq!(failed_reads.balance_mode(), BalanceMode::AsOnBillDate);
    let failed_html = render(failed_reads.balance_mode());
    assert!(failed_html.contains("<th>Balance Due</th><td>₹150.00</td>"));
    assert!(failed_html.contains("<td>Replay Cola</td><td>2</td><td>₹125.00</td><td>₹250.00</td>"));
    assert_eq!(fs::read(&archive).unwrap(), archive_bytes);
    let reopened = Store::open(&archive, Role::Owner).unwrap();
    assert_eq!(reopened.records(Role::Owner).unwrap(), &records);
    assert_eq!(
        reopened.business_profile(Role::Owner).unwrap(),
        &saved_profile
    );
    for key in ["theme", "privacy", "balance"] {
        fs::remove_dir(keys.join(key)).unwrap();
        fs::rename(keys.join(format!("{key}.saved")), keys.join(key)).unwrap();
    }
    let restored = Preferences::load(&keys);
    assert_eq!(projection(&restored), saved);
    let restored_html = render(restored.balance_mode());
    assert!(restored_html.contains("<th>Total Due (as on 01-Mar-2025)</th><td>₹90.00</td>"));
    assert!(
        restored_html.contains("<td>Replay Cola</td><td>2</td><td>₹125.00</td><td>₹250.00</td>")
    );
    assert_eq!(fs::read(&archive).unwrap(), archive_bytes);
    let reopened = Store::open(&archive, Role::Owner).unwrap();
    assert_eq!(reopened.records(Role::Owner).unwrap(), &records);
    assert_eq!(
        reopened.business_profile(Role::Owner).unwrap(),
        &saved_profile
    );
    println!(
        "read failure expected={defaults:?} observed={:?}; restored expected={saved:?} observed={:?}; business bytes/profile/records unchanged",
        projection(&failed_reads),
        projection(&restored)
    );
    fs::remove_dir_all(dir).unwrap();
}
