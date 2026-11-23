use std::{fs, path::PathBuf};

use super::model::{Database, Permission, Picker, Replay, Sandbox, TokenStore};

const CURRENT: &str = "ALM_data.json";
const DAILY: &str = "ALM_backup_2024-05-01.json";

const OLD_CURRENT: &[u8] =
    br#"{"records":["old current"],"meta":{"backup_folder":"new","last_file_backup":41}}"#;
const OLD_DATED: &[u8] =
    br#"{"records":["old dated"],"meta":{"backup_folder":"new","last_file_backup":37}}"#;
const NEW_PAYLOAD: &[u8] =
    br#"{"records":["initial connect"],"meta":{"backup_folder":"new","last_file_backup":7}}"#;

fn seed_occupied_backups(sandbox: &Sandbox) -> [PathBuf; 2] {
    let destinations = [sandbox.file("new", CURRENT), sandbox.file("new", DAILY)];
    for (path, bytes) in destinations.iter().zip([OLD_CURRENT, OLD_DATED]) {
        fs::write(path, bytes).unwrap();
    }
    destinations
}

#[test]
fn picker_rejection_preserves_each_independent_selection_and_files() {
    for (picker, expected_feedback) in [
        (Picker::Cancel, vec![]),
        (Picker::Fail, vec!["Could not connect folder"]),
    ] {
        let sandbox = Sandbox::new();
        let mut store = TokenStore {
            token: Some("old"),
            accepts: true,
        };
        let mut r = Replay::new(&sandbox, "picker fixture");
        r.restore(&store);
        r.trace.clear();
        r.connect(picker, &mut store);
        assert_eq!(r.feedback, expected_feedback);
        assert_eq!(r.selected, Some("old"));
        assert_eq!(r.db.meta.last_file_backup, 7);
        assert!(r.trace.is_empty());
        for folder in ["old", "new"] {
            assert!(!sandbox.file(folder, CURRENT).exists());
            assert!(!sandbox.file(folder, DAILY).exists());
        }
        let mut fresh = Replay::new(&sandbox, "fresh");
        fresh.restore(&store);
        assert_eq!(fresh.selected, Some("old"));
        assert_eq!(fresh.trace, ["restore-query"]);
        assert_eq!(fresh.requested, 0);
    }
    println!("picker cancel/failure: independent sandboxes, no writes, prior token reopened");
}

#[test]
fn denied_connect_reopens_token_without_files() {
    let sandbox = Sandbox::new();
    let mut store = TokenStore {
        token: None,
        accepts: true,
    };
    let mut r = Replay::new(&sandbox, "denied connect");
    r.permission = Permission::Prompt;
    r.request_result = Permission::Denied;
    r.connect(Picker::Select("new"), &mut store);
    assert_eq!(r.feedback, ["Backup folder set"]);
    assert_eq!(r.trace, ["query", "request"]);
    assert!(r.needs_grant);
    assert_eq!(r.db.meta.last_file_backup, 7);
    assert!(!sandbox.file("new", CURRENT).exists());
    assert!(!sandbox.file("new", DAILY).exists());
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.permission = Permission::Denied;
    fresh.restore(&store);
    assert_eq!(fresh.selected, Some("new"));
    assert_eq!(fresh.trace, ["restore-query"]);
    assert_eq!(fresh.requested, 0);
    assert!(fresh.needs_grant);
    println!("connect denied: normalized success feedback, no files, denied token reopened");
}

#[test]
fn initial_current_write_rejection_leaves_both_existing_backups_unchanged() {
    let sandbox = Sandbox::new();
    let destinations = seed_occupied_backups(&sandbox);
    let mut store = TokenStore {
        token: Some("old"),
        accepts: true,
    };
    let mut r = Replay::new(&sandbox, "initial connect");
    r.current_write.reject = true;

    r.connect(Picker::Select("new"), &mut store);

    assert_eq!(r.current_write.attempts, [NEW_PAYLOAD]);
    assert_eq!(r.trace, ["query", "current-write-failed"]);
    assert_eq!(
        destinations.map(|path| fs::read(path).unwrap()),
        [OLD_CURRENT, OLD_DATED]
    );
    assert_eq!((r.db.meta.last_file_backup, r.daily), (7, None));
    assert_eq!(r.feedback, ["Backup folder set"]);
    assert_eq!(r.requested, 0);
    assert!(!r.needs_grant);
    let mut reopened = Replay::new(&sandbox, "fresh session");
    reopened.restore(&store);
    assert_eq!(reopened.selected, Some("new"));
    println!(
        "L19-08 current write rejected with new payload: both existing files retain their distinct old bytes; no current close or dated attempt"
    );
}

#[test]
fn initial_current_write_success_publishes_new_bytes_to_both_existing_backups() {
    let sandbox = Sandbox::new();
    let destinations = seed_occupied_backups(&sandbox);
    let mut store = TokenStore {
        token: Some("old"),
        accepts: true,
    };
    let mut r = Replay::new(&sandbox, "initial connect");

    r.connect(Picker::Select("new"), &mut store);

    assert_eq!(r.current_write.attempts, [NEW_PAYLOAD]);
    assert_eq!(
        r.trace,
        ["query", "current-closed", "dated-staged", "dated-closed"]
    );
    assert_eq!(
        destinations.map(|path| fs::read(path).unwrap()),
        [NEW_PAYLOAD, NEW_PAYLOAD]
    );
    assert_eq!(
        (r.db.meta.last_file_backup, r.daily),
        (100, Some("2024-05-01"))
    );
    assert_eq!(r.feedback, ["Backup folder set"]);
    assert_eq!(store.token, Some("new"));
    println!(
        "L19-08 success control: current closes before dated publication; both existing files contain the new payload"
    );
}

#[test]
fn dated_publication_failure_preserves_current_before_retry() {
    let sandbox = Sandbox::new();
    let mut store = TokenStore {
        token: None,
        accepts: true,
    };
    let mut r = Replay::new(&sandbox, "partial connect");
    r.fail_dated = true;
    r.connect(Picker::Select("new"), &mut store);
    assert_eq!(r.feedback, ["Backup folder set"]);
    assert_eq!(
        r.trace,
        ["query", "current-closed", "dated-staged", "dated-failed"]
    );
    let mut expected = Database::new("partial connect");
    expected.meta.backup_folder = Some("new".into());
    assert_eq!(sandbox.read("new", CURRENT), expected);
    assert!(!sandbox.file("new", DAILY).exists());
    assert_eq!(r.db.meta.last_file_backup, 7);
    assert_eq!(r.daily, None);
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&store);
    assert_eq!(fresh.selected, Some("new"));
    assert_eq!(fresh.trace, ["restore-query"]);
    assert_eq!(fresh.requested, 0);
    r.fail_dated = false;
    r.now = 200;
    r.manual(false);
    assert_eq!(r.feedback[1], "Saved to folder");
    assert_eq!(sandbox.read("new", CURRENT), expected);
    assert_eq!(sandbox.read("new", DAILY), expected);
    assert_eq!(r.db.meta.last_file_backup, 200);
    println!(
        "dated publication failure: current closed, staged daily not published; token reopened; retry succeeds"
    );
}

#[test]
fn manual_dated_failure_leaves_current_recoverable_without_overwriting() {
    use std::io::ErrorKind;

    use alm_store::{
        Role,
        synthetic_read_model::{Error, Records, Store},
    };

    let sandbox = Sandbox::new();
    let destinations = seed_occupied_backups(&sandbox);
    let records = Records {
        reorder_default: 23,
        products: vec![],
        customers: vec![],
        sales: vec![],
        receipts: vec![],
    };
    let source = sandbox.file("old", "fixture.json");
    let archive = Store::create(&source, Role::Owner, records.clone())
        .unwrap()
        .backup(Role::Owner)
        .unwrap();
    // The replay envelope carries a typed archive only for this recovery test.
    let mut r = Replay::new(&sandbox, std::str::from_utf8(&archive).unwrap());
    r.selected = Some("new");
    r.db.meta.backup_folder = Some("new".into());
    r.fail_dated = true;

    r.manual(false);

    assert_eq!(r.feedback, ["Could not write"]);
    assert_eq!(
        r.trace,
        ["query", "current-closed", "dated-staged", "dated-failed"]
    );
    assert_eq!((r.db.meta.last_file_backup, r.daily), (7, None));
    let current_bytes = fs::read(&destinations[0]).unwrap();
    let published = serde_json::from_slice::<Database>(&current_bytes).unwrap();
    let [payload] = published.records.as_slice() else {
        panic!("expected one archive in the published replay envelope");
    };
    assert_eq!(payload.as_bytes(), archive);
    assert_eq!(fs::read(&destinations[1]).unwrap(), OLD_DATED);
    let staged_dated = destinations[1].with_extension("json.stage");
    assert!(!staged_dated.try_exists().unwrap());
    let restore_path = &destinations[1];
    assert!(matches!(
        Store::restore(restore_path, Role::Owner, payload.as_bytes()),
        Err(Error::Io(error))
            if error.kind() == ErrorKind::AlreadyExists
    ));

    let fresh_path = sandbox.file("old", "restored.json");
    Store::restore(&fresh_path, Role::Owner, payload.as_bytes()).unwrap();
    let reopened = Store::open(&fresh_path, Role::Owner).unwrap();
    assert_eq!(reopened.records(Role::Owner).unwrap(), &records);
    assert_eq!(fs::read(&destinations[0]).unwrap(), current_bytes);
    assert_eq!(fs::read(restore_path).unwrap(), OLD_DATED);
    println!(
        "L19-02/08: injected dated publication failure reported Could not write; current payload restored and reopened at a fresh path; occupied dated file rejected with AlreadyExists and unchanged"
    );
}

#[test]
fn day_rollover_creates_new_dated_copy() {
    let sandbox = Sandbox::new();
    let mut r = Replay::new(&sandbox, "first day");
    r.selected = Some("new");
    r.db.meta.backup_folder = Some("new".into());
    r.manual(false);
    let mut first = Database::new("first day");
    first.meta.backup_folder = Some("new".into());
    r.day = "2024-05-02";
    r.now = 200;
    r.db.records = vec!["second day".into()];
    r.manual(false);
    let mut second = Database::new("second day");
    second.meta.backup_folder = Some("new".into());
    second.meta.last_file_backup = 100;
    assert_eq!(sandbox.read("new", DAILY), first);
    assert_eq!(sandbox.read("new", "ALM_backup_2024-05-02.json"), second);
    assert_eq!(sandbox.read("new", CURRENT), second);
    assert_eq!(r.db.meta.last_file_backup, 200);
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&TokenStore::default());
    assert_eq!(fresh.selected, None);
    println!("day rollover: old dated copy retained, new daily and current contain updated record");
}
