//! Source-derived simulation using the typed archive subset, not the legacy JSON codec.
//! Browser APIs, transports, watermarks and folder handles are not modeled.
//! The primary winner's follow-up writes are assumed successful and synchronous.
use alm_core::Role;
use alm_core::synthetic_read_model::{BusinessProfile, Records};
use alm_store::synthetic_read_model::Store;

#[path = "clear_restore_replay/fixture.rs"]
mod fixture;
#[path = "clear_restore_replay/slots.rs"]
mod slots;
use fixture::{check_preferences, empty_records, imported_records, marked_records, preferences};
use slots::{Outcome, Sandbox, Snapshot, attempt, follow_up, read_slot, select, write_slot};

const OWNER: Role = Role::Owner;

fn archive(
    dir: &Sandbox,
    name: &str,
    records: Records,
    profile: Option<&BusinessProfile>,
) -> Vec<u8> {
    let mut store = Store::create(&dir.path(name), OWNER, records).unwrap();
    if let Some(value) = profile {
        store
            .save_business_profile(OWNER, value, 23, "fixture-device")
            .unwrap();
    }
    store.backup(OWNER).unwrap()
}

fn inspect(dir: &Sandbox, name: &str, snapshot: &Snapshot, expected: &(Records, BusinessProfile)) {
    let store = Store::restore(&dir.path(name), OWNER, &snapshot.archive).unwrap();
    assert_eq!(store.records(OWNER).unwrap(), &expected.0);
    assert_eq!(store.business_profile(OWNER).unwrap(), &expected.1);
}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Decline,
    Both,
    MirrorOnly,
    PrimaryOnly,
    Restore,
}

#[test]
fn clear_restore_replay() {
    // Given independent disposable scenarios with distinct outgoing and imported archives.
    for scenario in [
        Scenario::Decline,
        Scenario::Both,
        Scenario::MirrorOnly,
        Scenario::PrimaryOnly,
        Scenario::Restore,
    ] {
        let dir = Sandbox::new();
        preferences(&dir.0);
        let outgoing_profile = BusinessProfile {
            fssai: "outgoing license".into(),
            gstin: "outgoing gst".into(),
            show_gstin: Some(true),
            version: 1,
            modified_at: 23,
            device: "fixture-device".into(),
            ..BusinessProfile::default()
        };
        let imported_profile = BusinessProfile {
            fssai: "imported license".into(),
            gstin: "imported gst".into(),
            show_gstin: Some(true),
            version: 1,
            modified_at: 23,
            device: "fixture-device".into(),
            ..BusinessProfile::default()
        };
        let outgoing = archive(&dir, "outgoing", marked_records(), Some(&outgoing_profile));
        let imported = archive(
            &dir,
            "import-source",
            imported_records(),
            Some(&imported_profile),
        );
        let fresh = archive(&dir, "fresh", empty_records(), None);
        assert_ne!(outgoing, imported);
        let backup = dir.path("backup.json");
        write_slot(
            &backup,
            &Snapshot {
                revision: 3,
                archive: imported,
                activity: vec!["imported".into()],
            },
        );
        let prior = Snapshot {
            revision: 9,
            archive: outgoing.clone(),
            activity: vec!["prior".into()],
        };
        write_slot(&dir.path("mirror"), &prior);
        write_slot(&dir.path("primary"), &prior);

        // When the modeled action resolves, both writes are attempted except on decline.
        let model = match scenario {
            Scenario::Decline => prior.clone(),
            Scenario::Restore => {
                let mut replacement = read_slot(&backup);
                let restored =
                    Store::restore(&dir.path("imported"), OWNER, &replacement.archive).unwrap();
                replacement.archive = restored.backup(OWNER).unwrap();
                println!("Restore: imported revision={}", replacement.revision);
                replacement.revision += 1;
                replacement.activity.push("restore".into());
                replacement
            }
            Scenario::Both | Scenario::MirrorOnly | Scenario::PrimaryOnly => Snapshot {
                revision: 1,
                archive: fresh.clone(),
                activity: vec!["clear".into()],
            },
        };
        let outcomes = match scenario {
            Scenario::Decline => None,
            Scenario::Both | Scenario::Restore => Some((Outcome::Accepted, Outcome::Accepted)),
            Scenario::MirrorOnly => Some((Outcome::Accepted, Outcome::Rejected)),
            Scenario::PrimaryOnly => Some((Outcome::Rejected, Outcome::Accepted)),
        };
        let attempts = outcomes.map(|(m, p)| {
            (
                attempt(&dir.path("mirror"), &model, m),
                attempt(&dir.path("primary"), &model, p),
            )
        });
        assert_eq!(attempts, outcomes);

        // Then model, each slot, selected winner and follow-up are checked independently.
        let (
            revision,
            activity,
            model_data,
            mirror_revision,
            primary_revision,
            winner,
            selected_revision,
        ) = match scenario {
            Scenario::Decline => (
                9,
                vec!["prior"],
                (marked_records(), outgoing_profile.clone()),
                9,
                9,
                "mirror",
                9,
            ),
            Scenario::Both => (
                1,
                vec!["clear"],
                (empty_records(), BusinessProfile::default()),
                1,
                1,
                "mirror",
                1,
            ),
            Scenario::MirrorOnly => (
                1,
                vec!["clear"],
                (empty_records(), BusinessProfile::default()),
                1,
                9,
                "primary",
                9,
            ),
            Scenario::PrimaryOnly => (
                1,
                vec!["clear"],
                (empty_records(), BusinessProfile::default()),
                9,
                1,
                "mirror",
                9,
            ),
            Scenario::Restore => (
                4,
                vec!["imported", "restore"],
                (imported_records(), imported_profile.clone()),
                4,
                4,
                "mirror",
                4,
            ),
        };
        assert_eq!(model.revision, revision);
        assert_eq!(model.activity, activity);
        inspect(&dir, "immediate-check", &model, &model_data);
        if matches!(scenario, Scenario::Restore) {
            assert_ne!(model.archive, prior.archive);
        }
        let mirror = read_slot(&dir.path("mirror"));
        let primary = read_slot(&dir.path("primary"));
        assert_eq!(mirror.revision, mirror_revision);
        assert_eq!(primary.revision, primary_revision);
        assert_eq!(
            mirror,
            if mirror_revision == 9 {
                prior.clone()
            } else {
                model.clone()
            }
        );
        assert_eq!(
            primary,
            if primary_revision == 9 {
                prior.clone()
            } else {
                model.clone()
            }
        );
        let (selected_slot, selected) = select(&dir);
        assert_eq!(
            (selected_slot, selected.revision),
            (winner, selected_revision)
        );
        let expected_selected = if selected_revision == 9 {
            (marked_records(), outgoing_profile)
        } else {
            model_data
        };
        inspect(&dir, "selected-check", &selected, &expected_selected);
        let selected_activity = if selected_revision == 9 {
            vec!["prior"]
        } else {
            activity
        };
        assert_eq!(selected.activity, selected_activity);
        println!(
            "{scenario:?}: attempts={attempts:?} immediate={}: {:?}; slots mirror={} primary={}; selected={selected_slot}:{}",
            model.revision, model.activity, mirror.revision, primary.revision, selected.revision
        );
        if selected_slot == "primary" {
            let saved = follow_up(&dir, &selected);
            assert_eq!(saved.revision, 10);
            assert_eq!(saved.activity, vec!["prior"]);
            assert_eq!(read_slot(&dir.path("mirror")), saved);
            assert_eq!(read_slot(&dir.path("primary")), saved);
            println!("{scenario:?}: follow-up primary 9->10; mirror=10 primary=10 (both accepted)");
        }
        check_preferences(&dir.0);
    }
    println!(
        "scope: typed archive subset; simulated slots; browser APIs, transports, watermarks and folder handles absent; follow-up writes assumed successful"
    );
}

#[test]
fn equal_revisions_choose_mirror_even_when_payloads_differ() {
    // Given different snapshots with equal revisions in the slots.
    let dir = Sandbox::new();
    let mirror = Snapshot {
        revision: 7,
        archive: archive(&dir, "tie-fresh", empty_records(), None),
        activity: vec!["clear".into()],
    };
    let primary = Snapshot {
        revision: 7,
        archive: archive(&dir, "tie-marked", marked_records(), None),
        activity: vec!["prior".into()],
    };
    write_slot(&dir.path("mirror"), &mirror);
    write_slot(&dir.path("primary"), &primary);
    // When selecting, Then the mirror wins without a follow-up save.
    assert_eq!(select(&dir), ("mirror", mirror));
    assert_eq!(read_slot(&dir.path("primary")), primary);
    println!("Tie: mirror=7 primary=7 selected=mirror:7 (distinct payloads; no follow-up)");
}
