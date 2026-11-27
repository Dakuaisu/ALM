use std::fs;
use std::path::Path;

use alm_core::confirmation::BalanceMode;
use alm_core::preferences::{Preference, Theme};
use alm_core::synthetic_read_model::{Error, SaleId};
use alm_core::{BillDate, Money, Role};
use alm_store::preferences::Preferences;
use alm_store::synthetic_read_model::Store;

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const SALE: &str = "sale-cancelled-id";
const DEVICE: &str = "synthetic-device-recovery-source";
const AMOUNT: Money = Money::from_minor_units(5_000);

pub(super) fn run(source: &Store, reopened: &Store, dir: &Path) {
    let baseline = source.backup(OWNER).unwrap();
    assert_eq!(reopened.backup(OWNER).unwrap(), baseline);
    let id = SaleId::new(SALE.into());
    let today = BillDate::new(2025, 3, 1).unwrap();
    let baseline_html = source.estimate_html(OWNER, &id, today).unwrap();
    let reopened_html = reopened.estimate_html(OWNER, &id, today).unwrap();
    assert_eq!(reopened_html, baseline_html);
    assert!(baseline_html.contains("FSSAI: A&lt;&amp;&gt;"));
    assert!(!baseline_html.contains("GSTIN:"));

    // Given two devices with opposite preferences and a copy of the historical archive.
    let scenario = dir.join("device-recovery");
    let origin = scenario.join("origin");
    let destination = scenario.join("destination");
    fs::create_dir_all(&origin).unwrap();
    fs::create_dir_all(&destination).unwrap();
    let mut origin_prefs = Preferences::load(&origin);
    let mut destination_prefs = Preferences::load(&destination);
    for preference in [
        Preference::Theme(Theme::Dark),
        Preference::Privacy(true),
        Preference::Balance(BalanceMode::Latest),
    ] {
        origin_prefs.set(preference);
    }
    for preference in [
        Preference::Theme(Theme::Light),
        Preference::Privacy(false),
        Preference::Balance(BalanceMode::AsOnBillDate),
    ] {
        destination_prefs.set(preference);
    }
    let origin_keys = preference_files(&origin);
    let destination_keys = preference_files(&destination);
    assert_ne!(origin_keys, destination_keys);
    let origin_view = (Theme::Dark, None, BalanceMode::Latest);
    let destination_view = (Theme::Light, Some(AMOUNT), BalanceMode::AsOnBillDate);
    assert_eq!(projection(&origin), origin_view);
    assert_eq!(projection(&destination), destination_view);

    let origin_path = origin.join("business.json");
    let mut changed = Store::restore(&origin_path, OWNER, &baseline).unwrap();
    let mut profile = changed.business_profile(OWNER).unwrap().clone();
    let visibility_before = (profile.show_fssai, profile.show_gstin);
    assert_eq!(visibility_before, (Some(true), Some(false)));
    profile.show_fssai = Some(false);
    profile.show_gstin = Some(true);
    let next_version = profile.version.checked_add(1).unwrap();
    let denied = changed.save_business_profile(MANAGER, &profile, 5, DEVICE);
    assert!(matches!(denied, Err(Error::AccessDenied)));
    assert_eq!(changed.backup(OWNER).unwrap(), baseline);

    // When the Owner saves the visibility flags and restores a new backup on the other device.
    changed
        .save_business_profile(OWNER, &profile, 5, DEVICE)
        .unwrap();
    let changed = Store::open(&origin_path, OWNER).unwrap();
    let archive = changed.backup(OWNER).unwrap();
    let saved = changed.business_profile(OWNER).unwrap();
    let metadata = (saved.version, saved.modified_at, saved.device.as_str());
    assert_eq!(metadata, (next_version, 5, DEVICE));
    let visibility_after = (saved.show_fssai, saved.show_gstin);
    assert_eq!(visibility_after, (Some(false), Some(true)));
    let identities = (&saved.fssai, &saved.gstin);
    assert_eq!(identities, (&profile.fssai, &profile.gstin));
    let archive_text = std::str::from_utf8(&archive).unwrap();
    for key in [r#""theme":"#, r#""privacy":"#, r#""balance":"#] {
        assert!(
            !archive_text.contains(key),
            "device preference serialized: {key}"
        );
    }
    let backup_path = scenario.join("backup.json");
    fs::write(&backup_path, &archive).unwrap();
    let destination_path = destination.join("business.json");
    Store::restore(&destination_path, OWNER, &fs::read(&backup_path).unwrap()).unwrap();
    let restored = Store::open(&destination_path, OWNER).unwrap();
    assert_eq!(restored.backup(OWNER).unwrap(), archive);

    // Then both Stores render the same new header without changing the historical bill body.
    let changed_html = changed.estimate_html(OWNER, &id, today).unwrap();
    assert_ne!(changed_html, baseline_html);
    let restored_html = restored.estimate_html(OWNER, &id, today).unwrap();
    assert_eq!(restored_html, changed_html);
    assert!(changed_html.contains("GSTIN: Hidden &amp; GST"));
    assert!(!changed_html.contains("FSSAI:"));
    assert!(!changed_html.contains("Hidden & GST"));
    let body_before = baseline_html.split_once("<h1>").unwrap().1;
    let body_after = changed_html.split_once("<h1>").unwrap().1;
    assert_eq!(body_after, body_before);
    let csv = source.sales_csv(OWNER).unwrap();
    let workbook = source.workbook(OWNER).unwrap();
    let records = source.records(OWNER).unwrap();
    for store in [&changed, &restored] {
        assert_eq!(store.records(OWNER).unwrap(), records);
        assert_eq!(store.sales_csv(OWNER).unwrap(), csv);
        assert_eq!(store.workbook(OWNER).unwrap(), workbook);
        let manager_profile = store.business_profile(MANAGER);
        let manager_html = store.estimate_html(MANAGER, &id, today);
        // DEC-034: the shop's header prints on a Manager's bills; the archive's documents stay
        // the Owner's.
        assert!(manager_profile.is_ok());
        assert!(matches!(manager_html, Err(Error::AccessDenied)));
    }
    for document in [&changed_html, &csv, &workbook] {
        for private in [
            SALE,
            "sale-active-id",
            "receipt-standalone-id",
            "hidden-component-id",
            DEVICE,
            "A<&>",
            "A&lt;&amp;&gt;",
        ] {
            assert!(!document.contains(private), "projection leaked {private}");
        }
    }
    fs::write(origin.join("estimate.html"), &changed_html).unwrap();
    fs::write(destination.join("estimate.html"), &restored_html).unwrap();

    let invalid = archive_text.replacen(r#""version":1"#, r#""version":2"#, 1);
    assert_ne!(invalid, archive_text);
    let empty_path = destination.join("invalid.json");
    let invalid_fresh = Store::restore(&empty_path, OWNER, invalid.as_bytes());
    assert!(matches!(invalid_fresh, Err(Error::InvalidArchive)));
    assert!(!empty_path.exists());
    let destination_before = fs::read(&destination_path).unwrap();
    let wrong_version = Store::restore(&destination_path, OWNER, invalid.as_bytes());
    assert!(matches!(wrong_version, Err(Error::InvalidArchive)));
    let malformed = Store::restore(&destination_path, OWNER, b"not JSON");
    assert!(matches!(malformed, Err(Error::Json(_))));
    assert_eq!(fs::read(&destination_path).unwrap(), destination_before);
    let destination_after = Store::open(&destination_path, OWNER).unwrap();
    assert_eq!(destination_after.backup(OWNER).unwrap(), archive);
    assert_eq!(fs::read(&origin_path).unwrap(), archive);
    assert_eq!(preference_files(&origin), origin_keys);
    assert_eq!(preference_files(&destination), destination_keys);
    assert_eq!(projection(&origin), origin_view);
    assert_eq!(projection(&destination), destination_view);
    assert_eq!(source.backup(OWNER).unwrap(), baseline);
    assert_eq!(reopened.backup(OWNER).unwrap(), baseline);
    println!(
        "Device recovery: explicit GSTIN on / FSSAI off on both archives; local preferences differ; invalid restore preserved state"
    );
}

fn preference_files(directory: &Path) -> [Vec<u8>; 3] {
    ["theme", "privacy", "balance"].map(|key| fs::read(directory.join(key)).unwrap())
}

fn projection(directory: &Path) -> (Theme, Option<Money>, BalanceMode) {
    let prefs = Preferences::load(directory);
    (
        prefs.theme(),
        prefs.present_money(AMOUNT),
        prefs.balance_mode(),
    )
}
