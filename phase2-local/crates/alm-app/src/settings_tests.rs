use alm_core::BillDate;
use alm_core::preferences::{Preference, Theme};

use crate::Trial;
use crate::screen_tests::Root;
use crate::settings::Profile;

#[test]
fn settings_port_every_html_card_and_profile_field() {
    let root = Root::new("settings-full");
    let mut trial = Trial::open(&root.0, Some(BillDate::new(2025, 4, 30).unwrap())).unwrap();
    let html = trial.render("settings").unwrap();
    for card in [
        "🎨 Appearance",
        "Match device",
        "🏢 Business Profile",
        r#"name="name""#,
        r#"name="tagline""#,
        r#"name="owner""#,
        r#"name="mobile""#,
        r#"<textarea class="inp" name="address""#,
        r#"name="loading""#,
        r#"name="reorder""#,
        "☁ Cloud Sync — Supabase (primary)",
        "Enable cloud sync on this device",
        "⬇ Rebuild from cloud",
        "☁️ Google Sheets Sync",
        "🛟 Data Safety",
        "💾 Backup &amp; Restore",
        "🕒 Activity Log",
        "⚠️ Danger Zone",
    ] {
        assert!(html.contains(card), "{card}");
    }
    let order: Vec<_> = ["🏢", "☁ Cloud", "☁️ Google", "🛟", "💾", "🕒", "⚠️ Danger"]
        .iter()
        .map(|mark| html.find(mark).unwrap())
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(html.contains(r#"disabled title="Needs the hosted service"#));

    let form = Profile {
        name: "ALM Settings Shop".into(),
        tagline: "Cold drinks & water".into(),
        owner: "Owner A".into(),
        mobile: "9000000001".into(),
        address: "Shop 2, Synthetic Road".into(),
        loading: "2.5".into(),
        reorder: "".into(),
        ..Default::default()
    };
    trial.settings_profile(&form, 9).unwrap();
    let profile = trial.shop.business_profile(alm_core::Role::Owner).unwrap();
    assert_eq!(profile.loading_charge().minor_units(), 250);
    assert_eq!(profile.owner, "Owner A");
    assert_eq!(
        trial
            .shop
            .records(alm_core::Role::Owner)
            .unwrap()
            .reorder_default,
        5
    );
    let party = crate::outputs::party(Some(profile));
    assert_eq!(
        (party.business, party.tagline, party.address, party.mobile),
        (
            "ALM Settings Shop",
            "Cold drinks & water",
            "Shop 2, Synthetic Road",
            "9000000001"
        )
    );
    let html = trial.render("settings").unwrap();
    assert!(html.contains(r#"value="2.50""#));
    assert!(html.contains("Cold drinks &amp; water"));
    let bad = Profile {
        loading: "-1".into(),
        ..form
    };
    assert!(trial.settings_profile(&bad, 10).is_err());

    trial
        .settings_preference(Preference::Theme(Theme::System))
        .unwrap();
    let html = trial.render("settings").unwrap();
    assert!(html.contains(r#"data-theme="system""#));
    assert!(html.contains("Following this device's light or dark setting."));
    drop(trial);
    let reopened = Trial::open(&root.0, None).unwrap();
    assert_eq!(reopened.prefs.theme(), Theme::System);
    assert_eq!(
        reopened
            .shop
            .business_profile(alm_core::Role::Owner)
            .unwrap()
            .name,
        "ALM Settings Shop"
    );
}
