use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{DraftCustomer, DraftId, DraftStorageError, LocalSales, Money, ProvisionalDraft, Role};

struct DeviceRoot(PathBuf);

impl DeviceRoot {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&target).unwrap();
        loop {
            let next = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = target.join(format!("manager-account-{}-{next}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create device root: {error}"),
            }
        }
    }
}

impl Drop for DeviceRoot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("remove device root: {error}");
        }
    }
}

const DRAFT: DraftId = DraftId::new(1);

fn draft(name: &str) -> ProvisionalDraft {
    ProvisionalDraft::new(DRAFT, vec![]).with_customer(DraftCustomer::Typed(name.into()))
}

#[test]
fn case_distinct_accounts_with_same_draft_id_use_independent_databases() {
    // Given: one device root and two distinct, case-sensitive synthetic account IDs.
    let root = DeviceRoot::new();
    let a = draft("Synthetic A");
    let b = draft("Synthetic B");

    // When: the Manager saves draft 1 in each account and reopens each store.
    let upper = LocalSales::open_manager_account(&root.0, "Shop_A-1").unwrap();
    upper.save_draft(a.clone()).unwrap();
    let lower = LocalSales::open_manager_account(&root.0, "shop_a-1").unwrap();
    assert_eq!(lower.reopen_draft(DRAFT).unwrap(), None);
    lower.save_draft(b.clone()).unwrap();

    // Then: reopening each account retains only its own draft 1.
    for (account, expected) in [("Shop_A-1", a), ("shop_a-1", b)] {
        let reopened = LocalSales::open_manager_account(&root.0, account).unwrap();
        assert_eq!(reopened.reopen_draft(DRAFT).unwrap(), Some(expected));
    }
    assert_eq!(
        fs::read_dir(root.0.join("manager-accounts"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn invalid_accounts_are_rejected_before_creating_files() {
    // Given: a clean device root and path-like or invalid scope IDs.
    let root = DeviceRoot::new();
    let long = "a".repeat(65);

    // When: the caller supplies each invalid ID to the scoped entry point.
    for id in [
        "",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        "/escape",
        "a.b",
        " x",
        "a\0b",
        "é",
        &long,
    ] {
        assert!(
            matches!(
                LocalSales::open_manager_account(&root.0, id),
                Err(DraftStorageError::InvalidAccountScope)
            ),
            "accepted {id:?}"
        );
    }

    // Then: nothing has been allocated under the root.
    assert!(fs::read_dir(&root.0).unwrap().next().is_none());
}

#[test]
fn legacy_database_at_scoped_path_is_not_migrated_or_used() {
    // Given: an unbound legacy database at the exact derived account pathname.
    let root = DeviceRoot::new();
    let accounts = root.0.join("manager-accounts");
    fs::create_dir(&accounts).unwrap();
    let path = accounts.join("account-41.sqlite3");
    LocalSales::open_sqlite(&path, Money::from_minor_units(0))
        .unwrap()
        .session(Role::Manager)
        .save_draft(draft("Legacy only"))
        .unwrap();
    let legacy = fs::read(&path).unwrap();

    // When: account A is selected through the scoped API.
    let opened = LocalSales::open_manager_account(&root.0, "A");

    // Then: it fails closed and leaves the file untouched.
    assert!(matches!(
        opened,
        Err(DraftStorageError::AccountStoreMismatch)
    ));
    assert_eq!(fs::read(path).unwrap(), legacy);
}

#[test]
fn scoped_reopen_rejects_a_database_copied_from_another_account() {
    // Given: A has a draft and its database was copied to the derived slot for B.
    let root = DeviceRoot::new();
    let store = LocalSales::open_manager_account(&root.0, "A").unwrap();
    store.save_draft(draft("A only")).unwrap();
    let directory = root.0.join("manager-accounts");
    let a = directory.join("account-41.sqlite3");
    let b = directory.join("account-42.sqlite3");
    fs::copy(&a, &b).unwrap();

    // When: the trusted launcher selects B.
    let reopened = LocalSales::open_manager_account(&root.0, "B");

    // Then: it refuses A's file and does not claim it for B.
    assert!(matches!(
        reopened,
        Err(DraftStorageError::AccountStoreMismatch)
    ));
    assert_eq!(fs::read(&b).unwrap(), fs::read(&a).unwrap());
}

#[cfg(unix)]
#[test]
fn scoped_open_does_not_follow_links_for_namespace_database_or_sidecar() {
    use std::os::unix::fs::symlink;

    // Given: a disposable device root with a separate file and directory as link targets.
    let root = DeviceRoot::new();
    let target = root.0.join("other");
    fs::create_dir(&target).unwrap();
    let sentinel = target.join("sentinel");
    fs::write(&sentinel, b"untouched").unwrap();
    let directory = root.0.join("manager-accounts");

    // When: the namespace, database slot, or SQLite sidecar is a link.
    symlink(&target, &directory).unwrap();
    assert!(matches!(
        LocalSales::open_manager_account(&root.0, "A"),
        Err(DraftStorageError::InvalidPath)
    ));
    fs::remove_file(&directory).unwrap();
    fs::create_dir(&directory).unwrap();
    for slot in ["account-41.sqlite3", "account-41.sqlite3-wal"] {
        let path = directory.join(slot);
        symlink(&sentinel, &path).unwrap();
        assert!(matches!(
            LocalSales::open_manager_account(&root.0, "A"),
            Err(DraftStorageError::InvalidPath)
        ));
        fs::remove_file(path).unwrap();
    }
    fs::hard_link(&sentinel, directory.join("account-41.sqlite3")).unwrap();
    assert!(matches!(
        LocalSales::open_manager_account(&root.0, "A"),
        Err(DraftStorageError::InvalidPath)
    ));

    // Then: none of those attempts opens or initializes the linked target.
    assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
    assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
}
