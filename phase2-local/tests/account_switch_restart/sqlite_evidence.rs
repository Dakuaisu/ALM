use super::fixtures::{Account, DraftSpec};
use super::manager_sqlite::{SqliteEvidence, account_hex};

impl SqliteEvidence {
    pub(super) fn assert_empty(&self, account: Account) {
        self.assert_scope(account);
        assert!(
            self.rows().is_empty(),
            "expected no draft rows: {}",
            self.rows()
        );
        assert!(!self.dump().contains(account.marker()));
    }

    pub(super) fn assert_draft(&self, spec: &DraftSpec) {
        self.assert_scope(spec.account);
        let [(first_id, first_qty), (second_id, second_qty)] = spec.lines;
        assert_eq!(
            self.rows(),
            format!(
                "draft|21\ncustomer|21|2|{}|{}\ndate|21|{}\nline|21|0|{first_id}|{first_qty}\nline|21|1|{second_id}|{second_qty}\ncrate|21|{}\n",
                spec.buyer, spec.name, spec.date, spec.crate_sql
            ),
        );
        assert!(self.dump().contains(spec.account.marker()));
    }

    fn assert_scope(&self, account: Account) {
        self.assert_account_binding(account.key());
        assert!(!self.dump().contains(account.other().marker()));
        assert!(!self.dump().contains(&account_hex(account.other().key())));
        self.assert_no_markers(&["cost", "margin", "profit", "invoice", "payment", "stock"]);
    }
}
