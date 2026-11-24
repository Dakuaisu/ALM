//! Batch 3 integrated check, following the milestone D structured / folder-backup / clear-restore
//! replays: produce every output on device A, change a shared setting and device-local
//! preferences, back up, drop the store, restore on a fresh device B at a fresh path, regenerate
//! every output from the reopened archive alone and compare with the originals and with fixed
//! expectations; then check device-local settings, Manager denial and invalid-restore rejection.
use std::collections::HashMap;

use alm_core::confirmation::BalanceMode;
use alm_core::confirmation::owner_billing::{ReceiptTerms, save_receipt};
use alm_core::preferences::{Preference, Theme};
use alm_core::{BillDate, DraftId, Role};
use alm_store::synthetic_read_model::Store;

use crate::Trial;
use crate::outputs::regenerate;
use crate::screen_tests::Root;

fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn outputs_survive_backup_drop_and_fresh_restore_byte_for_byte() {
    let device_a = Root::new("batch3-a");
    let mut trial = Trial::open(&device_a.0, Some(day())).unwrap();
    crate::billing_tests::posted(&mut trial.authority);
    let directory = trial.directory.clone();
    save_receipt(
        &mut trial.authority,
        &directory,
        Role::Owner,
        "batch1-customer",
        ReceiptTerms {
            date: "2025-05-01",
            amount: "30.50",
            mode: alm_core::confirmation::ReceiptMode::Cash,
        },
    )
    .unwrap();
    trial
        .order_add(&crate::order::Line {
            product: "2".into(),
            qty: "10".into(),
        })
        .unwrap();
    trial.order_checkout().unwrap();
    trial
        .order_po(&crate::order::Po {
            supplier: "Synthetic Depot".into(),
            date: "2025-04-30".into(),
            note: "back gate".into(),
            paste: String::new(),
            intent: "save".into(),
        })
        .unwrap();
    trial
        .po_receive_save(map(&[
            ("po", "0"),
            ("date", "2025-05-01"),
            ("qty_0", "4"),
            ("rate_0", "36"),
        ]))
        .unwrap();

    // A shared setting (in the archive) and device-local preferences (not in the archive).
    trial
        .settings_profile(
            &crate::settings::Profile {
                gstin: "09ABCDE1234F1Z5".into(),
                fssai: "12345678901234".into(),
                show_fssai: "on".into(),
                show_gstin: "on".into(),
                reorder: "7".into(),
                ..Default::default()
            },
            1_700_000_000_000,
        )
        .unwrap();
    trial
        .settings_preference(Preference::Balance(BalanceMode::Latest))
        .unwrap();
    trial
        .settings_preference(Preference::Theme(Theme::Dark))
        .unwrap();

    // Originals, with fixed expectations.
    let (png_name, png) = trial.bill_image_png(DraftId::new(1)).unwrap();
    let estimate = trial.estimate_html(DraftId::new(1)).unwrap();
    let gst = trial.gst_invoice_html(crate::TRIAL_BILL).unwrap();
    let po = trial.po_print_html(1).unwrap();
    let rate = trial
        .authority
        .rate_list_html(
            Role::Owner,
            &mut trial.rates,
            (crate::SELLER.name, crate::SELLER.address, ""),
            day(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(png_name, "Bill-1.png");
    let decoded = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (1520, 854));
    assert!(
        estimate.contains("FSSAI: 12345678901234") && estimate.contains("GSTIN: 09ABCDE1234F1Z5")
    );
    assert!(estimate.contains("Synthetic Lime") && estimate.contains("Synthetic Customer"));
    assert!(gst.contains("ALM/25-26/1") && gst.contains("Synthetic GST Buyer"));
    assert!(po.contains("Order-PO-001-Synthetic Depot") && po.contains("Synthetic Soda"));
    assert!(rate.contains("font-weight:700\">₹120.5</td>") && rate.contains("(30-Apr-2025)"));

    let backup = trial.backup_create().unwrap();
    let bytes = std::fs::read(device_a.0.join("backups").join(&backup)).unwrap();
    drop(trial);

    // Device B: a fresh path, restore, regenerate from the reopened archive only.
    let device_b = Root::new("batch3-b");
    std::fs::create_dir_all(device_b.0.join("backups")).unwrap();
    std::fs::write(device_b.0.join("backups").join(&backup), &bytes).unwrap();
    let mut other = Trial::open(&device_b.0, Some(day())).unwrap();
    other.backup_restore(&backup).unwrap();
    assert_eq!(
        other.prefs.balance_mode(),
        BalanceMode::AsOnBillDate,
        "balance preference stays on device A"
    );
    assert_eq!(other.prefs.theme(), Theme::Light, "theme stays on device A");
    let restored_path = device_b.0.join("restored").join(format!(
        "{}-restored-1.json",
        backup.trim_end_matches(".json")
    ));
    assert!(restored_path.exists());
    other.restored = None;
    let store = Store::open(&restored_path, Role::Owner).unwrap();
    assert_eq!(
        store.business_profile(Role::Owner).unwrap().gstin,
        "09ABCDE1234F1Z5"
    );
    assert_eq!(store.records(Role::Owner).unwrap().reorder_default, 7);
    let again = regenerate(&store, Role::Owner, "sale-1", crate::TRIAL_BILL, 1, day()).unwrap();
    assert_eq!(again.bill_png, png, "PNG bytes equal");
    assert_eq!(again.estimate, estimate);
    assert_eq!(again.gst, gst);
    assert_eq!(again.po, po);
    assert_eq!(again.rate_list, rate);
    assert!(store.backup(Role::Owner).unwrap() == bytes);

    // The Manager is denied exports and the reopened archive.
    assert!(matches!(
        Store::open(&restored_path, Role::Manager),
        Err(alm_core::synthetic_read_model::Error::AccessDenied)
    ));
    assert!(store.backup(Role::Manager).is_err() && store.records(Role::Manager).is_err());
    assert!(regenerate(&store, Role::Manager, "sale-1", crate::TRIAL_BILL, 1, day()).is_err());
    other.owner = false;
    assert!(other.export(crate::outputs::Export::SalesCsv).is_err());
    other.owner = true;

    // An invalid archive is rejected without touching the restored data or the shop archive.
    let shop_before = std::fs::read(device_b.0.join("shop.json")).unwrap();
    let restored_before = std::fs::read(&restored_path).unwrap();
    let mut broken = bytes.clone();
    broken.truncate(bytes.len() / 2);
    std::fs::write(device_b.0.join("backups/ALM-backup-broken.json"), &broken).unwrap();
    assert!(other.backup_restore("ALM-backup-broken.json").is_err());
    assert!(
        !device_b
            .0
            .join("restored/ALM-backup-broken-restored-1.json")
            .exists()
    );
    assert_eq!(
        std::fs::read(device_b.0.join("shop.json")).unwrap(),
        shop_before
    );
    assert_eq!(std::fs::read(&restored_path).unwrap(), restored_before);
}
