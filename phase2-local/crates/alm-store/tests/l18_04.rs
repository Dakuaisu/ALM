#[path = "l18_04/support.rs"]
mod l18_04_support;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use alm_core::Role;
use alm_core::synthetic_read_model::{BusinessProfile, Records};
use alm_store::synthetic_read_model::Store;
use l18_04_support::{Expected, capture, fixture};

#[test]
fn identity_outputs_survive_each_disk_reopen_and_fresh_restore() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("alm-l18-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let source = dir.join("source.json");
    let records = Records {
        reorder_default: 5,
        products: vec![],
        customers: vec![],
        sales: vec![],
        receipts: vec![],
    };
    let mut store = Store::create(&source, Role::Owner, records).unwrap();
    let (fixture, invoice) = fixture();
    let cases = [
        ("unset", "F<&", "G<&", None, None, true, false),
        (
            "fssai-off-gst-unset",
            "F<&",
            "G<&",
            Some(false),
            None,
            false,
            false,
        ),
        (
            "fssai-unset-gst-on",
            "F<&",
            "G<&",
            None,
            Some(true),
            true,
            true,
        ),
        (
            "both-off",
            "F<&",
            "G<&",
            Some(false),
            Some(false),
            false,
            false,
        ),
        (
            "fssai-on-gst-off",
            "F<&",
            "G<&",
            Some(true),
            Some(false),
            true,
            false,
        ),
        ("both-on", "F<&", "G<&", Some(true), Some(true), true, true),
        (
            "empty-fssai",
            "",
            "G<&",
            Some(true),
            Some(true),
            false,
            true,
        ),
        (
            "empty-gstin",
            "F<&",
            "",
            Some(true),
            Some(true),
            true,
            false,
        ),
        ("both-empty", "", "", None, Some(true), false, false),
    ];
    let denied = BusinessProfile::default();
    assert!(
        store
            .save_business_profile(Role::Manager, &denied, 123, "local-device")
            .is_err()
    );
    assert!(store.business_profile(Role::Manager).is_err());
    store.set_reorder_default(Role::Owner, 13).unwrap();
    for (index, (name, fssai, gstin, show_fssai, show_gstin, visible_f, visible_g)) in
        cases.into_iter().enumerate()
    {
        let profile = BusinessProfile {
            fssai: fssai.into(),
            gstin: gstin.into(),
            show_fssai,
            show_gstin,
            ..BusinessProfile::default()
        };
        store
            .save_business_profile(Role::Owner, &profile, 123, "local-device")
            .unwrap();
        drop(store);
        store = Store::open(&source, Role::Owner).unwrap();
        let saved = store.business_profile(Role::Owner).unwrap();
        assert_eq!(
            (
                saved.fssai.as_str(),
                saved.gstin.as_str(),
                saved.show_fssai,
                saved.show_gstin
            ),
            (fssai, gstin, show_fssai, show_gstin)
        );
        assert_eq!(
            (saved.version, saved.modified_at, saved.device.as_str()),
            (u64::try_from(index + 1).unwrap(), 123, "local-device")
        );
        let expected = Expected {
            fssai,
            gstin,
            visible_f,
            visible_g,
        };
        let before = capture(&fixture, &invoice, saved, &expected);
        let backup = store.backup(Role::Owner).unwrap();
        let destination = dir.join(format!("restored-{index}.json"));
        drop(Store::restore(&destination, Role::Owner, &backup).unwrap());
        let restored = Store::open(&destination, Role::Owner).unwrap();
        assert_eq!(restored.business_profile(Role::Owner).unwrap(), saved);
        assert_eq!(restored.records(Role::Owner).unwrap().reorder_default, 13);
        let after = capture(
            &fixture,
            &invoice,
            restored.business_profile(Role::Owner).unwrap(),
            &expected,
        );
        assert_eq!(before, after);
        println!(
            "{name}: expected FSSAI={visible_f} GSTIN={visible_g}; reopened/restored draw calls {:?}; GST header {}",
            after.draws, after.gst_header
        );
    }
    let backup = store.backup(Role::Owner).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&backup).unwrap();
    old.as_object_mut().unwrap().remove("profile");
    let old = Store::restore(
        &dir.join("old.json"),
        Role::Owner,
        &serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert_eq!(
        old.business_profile(Role::Owner).unwrap(),
        &BusinessProfile::default()
    );
    assert_eq!(old.records(Role::Owner).unwrap().reorder_default, 13);
    std::fs::remove_dir_all(dir).unwrap();
}
