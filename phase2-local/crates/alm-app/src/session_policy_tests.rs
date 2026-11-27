use crate::session_policy::{Access, authorize, classify};
use crate::{Trial, billing_tests::day, dev_authority::RestConfig, screen_tests::Root};

#[test]
fn direct_bill_commands_deny_signed_out_and_revoked_sessions() {
    use crate::{bill_entry::commands, ipc::State};
    use std::sync::Mutex;
    let root = Root::new("policy-bill");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.rest_config = Some(RestConfig {
        url: String::new(),
        auth: String::new(),
        apikey: String::new(),
    });
    let state = Mutex::new(trial);
    for command in ["bill_live", "bill_customer_search", "bill_item_search"] {
        let call = || match command {
            "bill_live" => commands::bill_live(
                State::for_test(&state, command).unwrap(),
                "1".into(),
                "1".into(),
            ),
            "bill_customer_search" => commands::bill_customer_search(
                State::for_test(&state, command).unwrap(),
                String::new(),
            ),
            "bill_item_search" => {
                commands::bill_item_search(State::for_test(&state, command).unwrap(), String::new())
            }
            _ => unreachable!(),
        };
        assert!(call().is_err());
        {
            let mut trial = state.lock().unwrap();
            trial.rest_config = None;
            trial.revoked = true;
        }
        assert!(call().is_err());
        {
            let mut trial = state.lock().unwrap();
            trial.rest_config = Some(RestConfig {
                url: String::new(),
                auth: String::new(),
                apikey: String::new(),
            });
            trial.revoked = false;
        }
    }
}

#[test]
fn every_registered_command_has_an_explicit_policy() {
    let source = include_str!("lib.rs");
    let registered = source
        .split("= tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let commands: Vec<_> = registered
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.rsplit("::").next().unwrap())
        .collect();
    assert!(commands.len() > 100);
    for command in commands {
        assert!(classify(command).is_some(), "unclassified: {command}");
    }
    assert_eq!(classify("not_registered"), None);
}

#[test]
fn public_recovery_commands_are_explicitly_limited() {
    let source = include_str!("lib.rs");
    let registered = source
        .split("= tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let public: Vec<_> = registered
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.rsplit("::").next().unwrap())
        .filter(|name| classify(name) == Some(Access::Public))
        .collect();
    assert_eq!(
        public,
        [
            "start",
            "screen",
            "settings_rest_off_signed_out",
            "rest_login",
            "rest_logout"
        ]
    );
}

#[test]
fn every_registered_command_uses_the_guarded_state() {
    let sources = [
        include_str!("commands.rs"),
        include_str!("bill_entry/commands.rs"),
        include_str!("bill_entry/crate_row.rs"),
        include_str!("dev_authority.rs"),
        include_str!("device/account.rs"),
        include_str!("billing.rs"),
        include_str!("outputs/platform.rs"),
        include_str!("outputs/export.rs"),
        include_str!("settings.rs"),
        include_str!("backup.rs"),
        include_str!("receipts.rs"),
        include_str!("customers.rs"),
        include_str!("cashbook.rs"),
        include_str!("copy.rs"),
        include_str!("stock.rs"),
        include_str!("godown.rs"),
        include_str!("gst_sales.rs"),
        include_str!("products.rs"),
        include_str!("suppliers.rs"),
        include_str!("order.rs"),
        include_str!("purchases.rs"),
        include_str!("crates.rs"),
        include_str!("legacy_import.rs"),
        include_str!("file_pick.rs"),
    ];
    let registered = include_str!("lib.rs")
        .split("= tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    for command in registered
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let name = command.rsplit("::").next().unwrap();
        let marker = format!("pub fn {name}(");
        let source = sources
            .iter()
            .find(|source| source.contains(&marker))
            .unwrap();
        assert!(
            source.contains("use crate::ipc::State;"),
            "unguarded import: {name}"
        );
        let function = source
            .split(&marker)
            .nth(1)
            .unwrap()
            .split("#[tauri::command")
            .next()
            .unwrap();
        let signature = function.split('{').next().unwrap();
        assert!(
            signature.contains("state: State<'_, AppState>"),
            "unguarded command: {name}"
        );
        assert!(
            function.contains("state.lock()")
                || function.contains("with(&state")
                || function.contains("lock(&state"),
            "state not checked: {name}"
        );
    }
}

#[test]
fn signed_out_user_can_return_to_the_local_simulation() {
    use crate::{dev_authority, ipc::State};
    use std::sync::Mutex;
    let root = Root::new("policy-simulation-recovery");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.rest_config = Some(RestConfig {
        url: String::new(),
        auth: String::new(),
        apikey: String::new(),
    });
    let state = Mutex::new(trial);
    let command = State::for_test(&state, "settings_rest_off_signed_out").unwrap();
    assert!(dev_authority::settings_rest_off_signed_out(command).is_ok());
    assert!(state.lock().unwrap().rest_config.is_none());
}

#[test]
fn local_manager_can_search_and_switch_role_and_account() {
    use crate::{bill_entry::commands as bill, commands, device::account, ipc::State};
    use std::sync::Mutex;
    let root = Root::new("policy-local-recovery");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.owner = false;
    let state = Mutex::new(trial);
    assert!(
        bill::bill_item_search(
            State::for_test(&state, "bill_item_search").unwrap(),
            String::new()
        )
        .is_ok()
    );
    assert!(commands::review_role(State::for_test(&state, "review_role").unwrap(), true).is_ok());
    assert!(
        account::account_switch(
            State::for_test(&state, "account_switch").unwrap(),
            "trial-b".into()
        )
        .is_ok()
    );
}

#[test]
fn protected_commands_are_denied_when_signed_out_or_revoked() {
    let root = Root::new("policy-denied");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.rest_config = Some(RestConfig {
        url: String::new(),
        auth: String::new(),
        apikey: String::new(),
    });
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::NewBill)
        )
        .is_err()
    );
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::Settings)
        )
        .is_err()
    );
    assert!(authorize(&trial, Access::Public).is_ok());
    trial.rest_config = None;
    trial.revoked = true;
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::NewBill)
        )
        .is_err()
    );
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::Settings)
        )
        .is_err()
    );
    assert!(authorize(&trial, Access::Switch).is_ok());
}

#[test]
fn simulation_roles_and_switch_are_separate_from_real_sessions() {
    let root = Root::new("policy-simulation");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert!(authorize(&trial, Access::Simulation).is_ok());
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::Settings)
        )
        .is_ok()
    );
    trial.owner = false;
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::NewBill)
        )
        .is_ok()
    );
    assert!(
        authorize(
            &trial,
            Access::Gate(alm_core::permissions::Capability::Settings)
        )
        .is_err()
    );
    trial.rest_config = Some(RestConfig {
        url: String::new(),
        auth: String::new(),
        apikey: String::new(),
    });
    assert!(authorize(&trial, Access::Simulation).is_err());
    assert!(authorize(&trial, Access::Switch).is_err());
}

#[test]
fn owner_operations_are_denied_when_rest_is_signed_out() {
    let root = Root::new("policy-signed-out");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.rest_config = Some(RestConfig {
        url: String::new(),
        auth: String::new(),
        apikey: String::new(),
    });
    assert!(trial.require_owner().is_err());
}

/// The macOS bundle is sealed with an ad-hoc signature over every resource it lists, so
/// `codesign --verify --deep --strict` passes; the app writes only under its data and cache
/// folders, never into the bundle.
#[test]
fn the_bundle_is_ad_hoc_signed_and_lists_existing_resources_only() {
    let conf = include_str!("../tauri.conf.json");
    assert!(conf.contains(r#""signingIdentity": "-""#));
    let resources = conf
        .split(r#""resources": {"#)
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    let mut listed = 0;
    for line in resources.lines().filter(|l| l.contains(':')) {
        let source = line.split('"').nth(1).unwrap();
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(source)
                .is_file(),
            "{source}"
        );
        listed += 1;
    }
    assert!(resources.contains("assets/fonts/OFL.txt"), "font licence");
    assert!(listed >= 1);
    let setup = include_str!("lib.rs");
    assert!(setup.contains("app.path().app_data_dir()?"));
    assert!(!setup.contains("resource_dir") && !setup.contains("current_exe"));
    let root = Root::new("policy-bundle");
    let trial = Trial::open(&root.0, Some(day())).unwrap();
    assert!(trial.outputs.starts_with(&root.0));
}

/// DEC-034: every registered command, for both roles, is allowed or denied exactly as the
/// matrix row of its capability says; every screen tab likewise.
#[test]
fn every_app_command_and_screen_follows_the_matrix_for_both_roles() {
    use alm_core::Role;
    use alm_core::permissions::{Capability, allows};
    let registered = include_str!("lib.rs")
        .split("= tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let root = Root::new("policy-matrix");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let mut walked = std::collections::BTreeSet::new();
    for role in [Role::Owner, Role::Manager] {
        trial.owner = role == Role::Owner;
        for name in registered
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| name.rsplit("::").next().unwrap())
        {
            let Some(Access::Gate(capability)) = classify(name) else {
                continue;
            };
            walked.insert(capability);
            assert_eq!(
                authorize(&trial, Access::Gate(capability)).is_ok(),
                allows(role, capability),
                "{role:?} {name} ({capability:?})"
            );
        }
        for tab in [
            "bill",
            "review",
            "sales",
            "counter",
            "customers",
            "receipt",
            "expenses",
            "dashboard",
            "reorder",
            "stock",
            "godown",
            "gstsales",
            "gst",
            "products",
            "suppliers",
            "order",
            "purchases",
            "crates",
            "ratelist",
            "rateprint",
            "reports",
            "settings",
            "estimate",
            "poprint",
        ] {
            let capability = crate::session_policy::screen(tab).unwrap();
            walked.insert(capability);
            assert_eq!(
                trial.require(capability).is_ok(),
                allows(role, capability),
                "{role:?} tab {tab}"
            );
        }
    }
    // Screens and device actions of the matrix all have a command or a tab, except the ones
    // the app does not offer yet and the Cash Book history, which the Cash Book screen gates.
    for row in alm_core::permissions::MATRIX {
        let device_only = row.sql.is_empty();
        let not_offered = matches!(
            row.capability,
            Capability::ClearAll
                | Capability::ActivityLog
                | Capability::SaveCustomer
                | Capability::CashBookHistory
        );
        assert!(
            !device_only || not_offered || walked.contains(&row.capability),
            "{:?} has no command or screen",
            row.capability
        );
    }
}
