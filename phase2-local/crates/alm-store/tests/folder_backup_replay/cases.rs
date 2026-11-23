use super::model::{Database, Permission, Picker, Replay, Sandbox, TokenStore};

const CURRENT: &str = "ALM_data.json";
const DAILY: &str = "ALM_backup_2024-05-01.json";

fn expected(record: &str, folder: &str, last: u64) -> Database {
    let mut db = Database::new(record);
    db.meta.backup_folder = Some(folder.into());
    db.meta.last_file_backup = last;
    db
}

#[test]
fn automatic_burst_uses_execution_time_data_and_replaces_timer() {
    let sandbox = Sandbox::new();
    let mut r = Replay::new(&sandbox, "before");
    r.schedule();
    assert_eq!(r.deadline, None);
    r.advance(1600);
    assert!(r.trace.is_empty());
    assert!(!sandbox.file("old", CURRENT).exists());
    assert!(!sandbox.file("old", DAILY).exists());
    assert!(!sandbox.file("new", CURRENT).exists());
    assert!(!sandbox.file("new", DAILY).exists());
    assert!(r.feedback.is_empty());
    r.now = 100;
    r.selected = Some("old");
    r.db.meta.backup_folder = Some("old".into());
    r.schedule();
    r.advance(200);
    r.schedule();
    r.db.records = vec!["at callback".into()];
    r.advance(1600);
    assert!(!sandbox.file("old", CURRENT).exists());
    r.advance(1700);
    assert_eq!(
        sandbox.read("old", CURRENT),
        expected("at callback", "old", 7)
    );
    assert_eq!(
        sandbox.read("old", DAILY),
        expected("at callback", "old", 7)
    );
    assert_eq!(r.db.meta.last_file_backup, 1700);
    assert_eq!(r.requested, 0);
    assert!(r.feedback.is_empty());
    assert_eq!(
        r.trace,
        [
            "callback",
            "query",
            "current-closed",
            "dated-staged",
            "dated-closed"
        ]
    );
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&TokenStore::default());
    assert_eq!(fresh.selected, None);
    println!(
        "L19-05 burst: replaced deadline; callback -> query -> current close -> dated close; no request"
    );
}

#[test]
fn automatic_checks_permission_at_callback_without_prompting() {
    let sandbox = Sandbox::new();
    let mut r = Replay::new(&sandbox, "denied callback");
    r.selected = Some("old");
    r.schedule();
    r.permission = Permission::Denied;
    r.advance(1600);
    assert!(r.needs_grant);
    assert_eq!(r.requested, 0);
    assert!(r.feedback.is_empty());
    assert_eq!(r.db.meta.last_file_backup, 7);
    assert!(!sandbox.file("old", CURRENT).exists());
    assert!(!sandbox.file("old", DAILY).exists());
    assert_eq!(r.trace, ["callback", "query"]);
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&TokenStore::default());
    assert_eq!(fresh.selected, None);
    println!("L19-05 denied callback: query -> reconnect needed; no prompt or file");
}

#[test]
fn manual_save_and_reconnect_report_their_own_results() {
    let sandbox = Sandbox::new();
    let mut r = Replay::new(&sandbox, "manual");
    r.selected = Some("new");
    r.db.meta.backup_folder = Some("new".into());
    r.permission = Permission::Prompt;
    r.request_result = Permission::Granted;
    r.manual(false);
    assert_eq!(r.feedback, ["Saved to folder"]);
    assert_eq!(r.requested, 1);
    assert_eq!(sandbox.read("new", CURRENT), expected("manual", "new", 7));
    assert_eq!(sandbox.read("new", DAILY), expected("manual", "new", 7));
    assert_eq!(r.db.meta.last_file_backup, 100);
    r.permission = Permission::Denied;
    r.request_result = Permission::Denied;
    r.manual(true);
    assert_eq!(r.feedback[1], "Permission denied");
    assert_eq!(r.requested, 2);
    assert!(r.needs_grant);
    assert_eq!(r.db.meta.last_file_backup, 100);
    r.request_result = Permission::Granted;
    r.db.records = vec!["reconnected".into()];
    r.now = 200;
    r.manual(true);
    assert_eq!(r.feedback[2], "Reconnected & backed up");
    assert_eq!(
        sandbox.read("new", CURRENT),
        expected("reconnected", "new", 100)
    );
    assert_eq!(sandbox.read("new", DAILY), expected("manual", "new", 7));
    assert_eq!(r.db.meta.last_file_backup, 200);
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&TokenStore::default());
    assert_eq!(fresh.selected, None);
    println!(
        "L19-02 manual save/reconnect: prompt granted; denied; retry granted; same-day copy unchanged"
    );
}

#[test]
fn connect_success_persists_token_and_fresh_session_restores() {
    let sandbox = Sandbox::new();
    let mut store = TokenStore {
        token: None,
        accepts: true,
    };
    let mut r = Replay::new(&sandbox, "connect success");
    r.connect(Picker::Select("new"), &mut store);
    assert_eq!(r.feedback, ["Backup folder set"]);
    assert_eq!(store.token, Some("new"));
    assert_eq!(
        sandbox.read("new", CURRENT),
        expected("connect success", "new", 7)
    );
    assert_eq!(
        sandbox.read("new", DAILY),
        expected("connect success", "new", 7)
    );
    assert_eq!(r.db.meta.last_file_backup, 100);
    let mut fresh = Replay::new(&sandbox, "fresh session");
    fresh.restore(&store);
    assert_eq!(fresh.selected, Some("new"));
    assert_eq!(fresh.db.meta.backup_folder.as_deref(), Some("new"));
    assert!(!fresh.needs_grant);
    assert_eq!(fresh.requested, 0);
    println!(
        "L19-06/08 connect: both files closed; fresh session restored persisted token without prompting"
    );
}

#[test]
fn false_handle_store_keeps_prior_token_or_none_after_successful_write() {
    for prior in [None, Some("old")] {
        let sandbox = Sandbox::new();
        let mut store = TokenStore {
            token: prior,
            accepts: false,
        };
        let mut r = Replay::new(&sandbox, "volatile selection");
        r.connect(Picker::Select("new"), &mut store);
        assert_eq!(r.selected, Some("new"));
        assert_eq!(store.token, prior);
        assert_eq!(r.feedback, ["Backup folder set"]);
        assert_eq!(
            sandbox.read("new", CURRENT),
            expected("volatile selection", "new", 7)
        );
        assert_eq!(
            sandbox.read("new", DAILY),
            expected("volatile selection", "new", 7)
        );
        assert_eq!(r.db.meta.last_file_backup, 100);
        let mut fresh = Replay::new(&sandbox, "fresh");
        fresh.restore(&store);
        assert_eq!(fresh.selected, prior);
    }
    println!(
        "L19-08 false token store: success feedback and files; fresh session restores prior token or none"
    );
}

#[test]
fn restore_permission_variants_never_prompt_or_write() {
    let sandbox = Sandbox::new();
    let store = TokenStore {
        token: Some("old"),
        accepts: true,
    };
    for permission in [Permission::Granted, Permission::Prompt, Permission::Denied] {
        let mut r = Replay::new(&sandbox, "restore");
        r.permission = permission;
        r.restore(&store);
        assert_eq!(r.selected, Some("old"));
        assert_eq!(r.db.meta.backup_folder.as_deref(), Some("old"));
        assert_eq!(r.needs_grant, permission != Permission::Granted);
        assert_eq!(r.requested, 0);
        assert_eq!(r.trace, ["restore-query"]);
        assert!(r.feedback.is_empty());
        assert!(!sandbox.file("old", CURRENT).exists());
        assert!(!sandbox.file("old", DAILY).exists());
    }
    println!("L19-06 restore: granted, prompt and denied queries; no request or write");
}

#[test]
fn manual_save_denial_reports_write_failure() {
    let sandbox = Sandbox::new();
    let mut r = Replay::new(&sandbox, "save denied");
    r.selected = Some("old");
    r.permission = Permission::Denied;
    r.request_result = Permission::Denied;
    r.manual(false);
    assert_eq!(r.feedback, ["Could not write"]);
    assert_eq!(r.requested, 1);
    assert!(r.needs_grant);
    assert_eq!(r.db.meta.last_file_backup, 7);
    assert!(!sandbox.file("old", CURRENT).exists());
    assert!(!sandbox.file("old", DAILY).exists());
    let mut fresh = Replay::new(&sandbox, "fresh");
    fresh.restore(&TokenStore::default());
    assert_eq!(fresh.selected, None);
    println!("L19-02 manual save denied: requested permission; could not write");
}
