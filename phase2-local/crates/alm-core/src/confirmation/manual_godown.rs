use std::collections::btree_map::Entry;

use super::ConfirmationFixture;
use crate::Role;

/// A whole-case count for this synthetic fixture, not a production quantity restriction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GodownCases(u32);

impl GodownCases {
    pub const fn new(whole_cases: u32) -> Self {
        Self(whole_cases)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualGodownRow {
    pub item: String,
    pub cases: GodownCases,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualGodownInput<'a> {
    pub item: &'a str,
    pub cases: &'a str,
    pub note: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManualGodownError {
    AccessDenied,
    ItemNameRequired,
    InvalidCases,
}

const fn owner(role: Role) -> Result<(), ManualGodownError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(ManualGodownError::AccessDenied),
    }
}

/// The Godown screen's filtered list: rows by name and the total of their whole cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualGodownList<'a> {
    pub rows: Vec<&'a ManualGodownRow>,
    pub total_cases: u64,
}

impl ConfirmationFixture {
    /// Lists manual rows whose name contains `query` (case-insensitive), sorted by name, with
    /// their case total. Billing stock is neither read nor changed.
    pub fn manual_godown_list(
        &self,
        role: Role,
        query: &str,
    ) -> Result<ManualGodownList<'_>, ManualGodownError> {
        let needle = query.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .open_manual_godown(role)?
            .into_iter()
            .filter(|row| row.item.to_lowercase().contains(&needle))
            .collect();
        rows.sort_by(|a, b| a.item.cmp(&b.item));
        let total_cases = rows.iter().map(|row| u64::from(row.cases.get())).sum();
        Ok(ManualGodownList { rows, total_cases })
    }

    /// Reopens the Owner's manual list, without reading or changing billing stock.
    pub fn open_manual_godown(
        &self,
        role: Role,
    ) -> Result<Vec<&ManualGodownRow>, ManualGodownError> {
        owner(role)?;
        Ok(self.manual_godown.values().collect())
    }

    /// Adds or updates a manual row by trimmed, case-insensitive name. A blank update note
    /// leaves the previous note intact. The synthetic form accepts only whole decimal cases.
    pub fn save_manual_godown(
        &mut self,
        role: Role,
        input: ManualGodownInput<'_>,
    ) -> Result<(), ManualGodownError> {
        owner(role)?;
        let item = input.item.trim();
        if item.is_empty() {
            return Err(ManualGodownError::ItemNameRequired);
        }
        let text = input.cases.trim();
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ManualGodownError::InvalidCases);
        }
        let cases = GodownCases::new(text.parse().map_err(|_| ManualGodownError::InvalidCases)?);
        let note = input.note.trim();
        match self.manual_godown.entry(item.to_lowercase()) {
            Entry::Occupied(mut entry) => {
                let row = entry.get_mut();
                row.cases = cases;
                if !note.is_empty() {
                    row.note = Some(note.to_owned());
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(ManualGodownRow {
                    item: item.to_owned(),
                    cases,
                    note: (!note.is_empty()).then(|| note.to_owned()),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod list_tests {
    use super::{ManualGodownError, ManualGodownInput};
    use crate::confirmation::demo_fixture::fixture;
    use crate::{ProductId, Role};

    #[test]
    fn godown_list_filters_sorts_and_totals_without_touching_billing_stock() {
        let mut fixture = fixture().unwrap();
        for (item, cases) in [
            ("Synthetic Soda", "4"),
            ("synthetic lime", "7"),
            ("Ice", "2"),
        ] {
            fixture
                .save_manual_godown(
                    Role::Owner,
                    ManualGodownInput {
                        item,
                        cases,
                        note: "",
                    },
                )
                .unwrap();
        }
        let list = fixture.manual_godown_list(Role::Owner, "SYNTH").unwrap();
        let rows: Vec<_> = list.rows.iter().map(|row| row.item.as_str()).collect();
        assert_eq!(rows, ["Synthetic Soda", "synthetic lime"]);
        assert_eq!(list.total_cases, 11);
        assert_eq!(fixture.product_stock(ProductId::new(2)), Some(40));
        assert_eq!(
            fixture.manual_godown_list(Role::Manager, ""),
            Err(ManualGodownError::AccessDenied)
        );
    }
}
