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

impl ConfirmationFixture {
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
