use std::path::Path;

use alm_store::draft_entry::{
    ActiveBillFacts, ActiveBillForm, CustomerId, CustomerPrefill, DraftChoice, DraftEntry,
    DraftEntryAction, DraftEntryOutcome, FreshBillStart,
};
use alm_store::{AccessDenied, DraftCustomer, DraftCustomerId, DraftId, ProvisionalDraft, Role};

use super::{EDITED_LINES, ORIGINAL_LINES, ScratchDir, draft, open_sales, run_sql, run_worker};

const SELECTED: DraftId = DraftId::new(101);
const TYPED: DraftId = DraftId::new(102);
const UNASSIGNED: DraftId = DraftId::new(103);
const ID: &str = "id'|\nनमस्ते\0NULL";
const NAME: &str = "O'Brien|\n ग्राहक\0NULL";
const TEXT: &str = "NULL|\n' ग्राहक";
const EDIT: &str = "edited|\n'名";

fn from_entry(draft: &ProvisionalDraft, text: &str) -> ProvisionalDraft {
    let facts = ActiveBillFacts::new(&[], text, 0.0, "");
    let DraftEntry::AwaitingChoice(pending) = DraftEntryAction::NewBill.begin(Some((draft, facts)))
    else {
        panic!("customer is work");
    };
    let DraftEntryOutcome::OpenExisting(form) = pending.resolve(DraftChoice::Continue) else {
        panic!("continue opens the entry form");
    };
    form.snapshot()
}

fn expected(id: DraftId, edited: bool) -> ProvisionalDraft {
    match id {
        SELECTED => draft(id, &[]).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(ID.to_owned()),
            name: NAME.to_owned(),
        }),
        TYPED if edited => {
            draft(id, ORIGINAL_LINES).with_customer(DraftCustomer::Typed(EDIT.to_owned()))
        }
        TYPED => draft(id, ORIGINAL_LINES).with_customer(DraftCustomer::Typed(TEXT.to_owned())),
        UNASSIGNED => draft(id, &[]).with_customer(DraftCustomer::Unassigned),
        _ => panic!("unexpected draft"),
    }
}

#[test]
fn customer_entry_survives_process_restart_and_isolated_edit() {
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    run_worker("customer_write", &path);
    run_worker("customer_edit", &path);
    run_worker("customer_read", &path);
    let tables = run_sql(
        &path,
        true,
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;",
    );
    assert_eq!(
        tables.lines().collect::<Vec<_>>(),
        [
            "pending_owner_confirmations",
            "provisional_draft_crates",
            "provisional_draft_customers",
            "provisional_draft_dates",
            "provisional_draft_lines",
            "provisional_drafts"
        ]
    );
}

pub(super) fn worker(mode: &str, path: &Path) {
    let mut sales = open_sales(path);
    let mut manager = sales.session(Role::Manager);
    assert_eq!(manager.total_profit(), Err(AccessDenied));
    match mode {
        "customer_write" => {
            let selected = CustomerPrefill::new(CustomerId::new(ID), NAME);
            let DraftEntry::Resolved(DraftEntryOutcome::OpenFreshWith(form)) =
                DraftEntryAction::NewBill
                    .begin_with_customer(None, FreshBillStart::new(SELECTED, Some(selected)))
            else {
                panic!("new bill opens the selected prefill");
            };
            assert_eq!(form.customer(), Some(selected));
            assert_eq!(form.customer_name(), NAME);
            manager.save_draft(form.snapshot()).unwrap();
            manager
                .save_draft(from_entry(&draft(TYPED, ORIGINAL_LINES), TEXT))
                .unwrap();
            manager
                .save_draft(FreshBillStart::new(UNASSIGNED, None).into_form().snapshot())
                .unwrap();
            assert_eq!(manager.total_profit(), Err(AccessDenied));
        }
        "customer_edit" | "customer_read" => {
            let mut typed = None;
            for id in [SELECTED, TYPED, UNASSIGNED] {
                let reopened = manager.reopen_draft(id).unwrap().unwrap();
                assert_eq!(reopened, expected(id, mode == "customer_read"));
                let form = ActiveBillForm::reopen(&reopened);
                match id {
                    SELECTED => {
                        assert_eq!(
                            form.customer(),
                            Some(CustomerPrefill::new(CustomerId::new(ID), NAME))
                        );
                        assert_eq!(form.customer_name(), NAME);
                    }
                    TYPED => {
                        assert_eq!(form.customer(), None);
                        assert_eq!(
                            form.customer_name(),
                            if mode == "customer_edit" { TEXT } else { EDIT }
                        );
                    }
                    UNASSIGNED => {
                        assert_eq!(form.customer(), None);
                        assert_eq!(form.customer_name(), "");
                    }
                    _ => unreachable!(),
                }
                assert_eq!(form.snapshot(), reopened);
                if id == TYPED {
                    typed = Some(reopened);
                }
            }
            assert_eq!(manager.total_profit(), Err(AccessDenied));
            if mode == "customer_edit" {
                manager
                    .save_draft(from_entry(&typed.expect("typed draft reopened"), EDIT))
                    .unwrap();
                assert_eq!(manager.total_profit(), Err(AccessDenied));
            }
        }
        _ => unreachable!(),
    }
}

#[test]
fn old_database_reopens_unassigned_without_losing_lines() {
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    run_sql(
        &path,
        false,
        "CREATE TABLE provisional_drafts (id TEXT PRIMARY KEY NOT NULL); CREATE TABLE provisional_draft_lines (draft_id TEXT NOT NULL, position INTEGER NOT NULL, product_id TEXT NOT NULL, quantity INTEGER NOT NULL, PRIMARY KEY(draft_id, position)); INSERT INTO provisional_drafts VALUES ('101'); INSERT INTO provisional_draft_lines VALUES ('101',0,'4',1);",
    );
    let reopened = open_sales(&path)
        .session(Role::Manager)
        .reopen_draft(SELECTED)
        .unwrap()
        .unwrap();
    assert_eq!(reopened, draft(SELECTED, &[(4, 1)]));
    assert_eq!(ActiveBillForm::reopen(&reopened).customer_name(), "");
}

#[test]
fn customer_and_lines_roll_back_together_when_line_insert_fails() {
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    open_sales(&path)
        .session(Role::Manager)
        .save_draft(expected(TYPED, false))
        .unwrap();
    run_sql(
        &path,
        false,
        "CREATE TRIGGER reject_customer_edit BEFORE INSERT ON provisional_draft_lines BEGIN SELECT RAISE(ABORT, 'reject'); END;",
    );
    assert!(
        open_sales(&path)
            .session(Role::Manager)
            .save_draft(from_entry(&draft(TYPED, EDITED_LINES), EDIT))
            .is_err()
    );
    assert_eq!(
        open_sales(&path)
            .session(Role::Manager)
            .reopen_draft(TYPED)
            .unwrap(),
        Some(expected(TYPED, false))
    );
}
