use super::model::{Database, Permission, Picker, Replay, Sandbox, TokenStore};

const CURRENT: &str = "ALM_data.json";
const DAILY: &str = "ALM_backup_2024-05-01.json";

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
