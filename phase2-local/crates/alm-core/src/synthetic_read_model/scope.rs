use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{Error, Records};

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub const fn new(value: String) -> Self {
                Self(value)
            }
            pub(super) fn value(&self) -> &str {
                &self.0
            }
        }
    };
}
id!(AccountId);
id!(SaleId);
id!(ReceiptId);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountScope {
    pub sales: Vec<(SaleId, AccountId)>,
    pub receipts: Vec<(ReceiptId, AccountId)>,
    /// Explicit assertion that each referenced account has a zero opening balance.
    pub zero_openings: Vec<AccountId>,
}

pub(super) struct ValidatedScope<'a> {
    pub sales: HashMap<&'a str, &'a AccountId>,
    pub receipts: HashMap<&'a str, &'a AccountId>,
}

impl AccountScope {
    pub(super) fn validate<'a>(
        &'a self,
        records: &'a Records,
    ) -> Result<ValidatedScope<'a>, Error> {
        let mut sales = HashMap::with_capacity(self.sales.len());
        let mut receipts = HashMap::with_capacity(self.receipts.len());
        let mut accounts = HashSet::new();
        for (id, account) in &self.sales {
            if !valid_id(id.value())
                || !valid_id(account.value())
                || sales.insert(id.value(), account).is_some()
            {
                return Err(Error::InvalidArchive);
            }
            accounts.insert(account);
        }
        for (id, account) in &self.receipts {
            if !valid_id(id.value())
                || !valid_id(account.value())
                || receipts.insert(id.value(), account).is_some()
            {
                return Err(Error::InvalidArchive);
            }
            accounts.insert(account);
        }
        let openings: HashSet<_> = self.zero_openings.iter().collect();
        if openings.len() != self.zero_openings.len() || openings != accounts {
            return Err(Error::InvalidArchive);
        }
        let mut seen_sales = HashSet::new();
        for sale in &records.sales {
            if !sales.contains_key(sale.id.as_str()) || !seen_sales.insert(sale.id.as_str()) {
                return Err(Error::InvalidArchive);
            }
        }
        let mut seen_receipts = HashSet::new();
        for receipt in &records.receipts {
            if !receipts.contains_key(receipt.id.as_str())
                || !seen_receipts.insert(receipt.id.as_str())
            {
                return Err(Error::InvalidArchive);
            }
        }
        if seen_sales.len() != sales.len() || seen_receipts.len() != receipts.len() {
            return Err(Error::InvalidArchive);
        }
        Ok(ValidatedScope { sales, receipts })
    }
}
