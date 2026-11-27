use std::collections::btree_map::Entry;

use super::ConfirmationFixture;
use crate::Role;

/// A whole-case count for this synthetic fixture, not a production quantity restriction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
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

fn gate(role: Role, capability: crate::permissions::Capability) -> Result<(), ManualGodownError> {
    crate::permissions::require(role, capability).map_err(|_| ManualGodownError::AccessDenied)
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

    /// `copyGodownList`: the WhatsApp-style text of every row by name, or `None` when empty.
    pub fn manual_godown_copy_text(
        &self,
        role: Role,
        today: crate::BillDate,
    ) -> Result<Option<String>, ManualGodownError> {
        let list = self.manual_godown_list(role, "")?;
        if list.rows.is_empty() {
            return Ok(None);
        }
        let mut text = format!("*Godown Stock* ({})\n\n", today.display());
        for row in &list.rows {
            let note = row
                .note
                .as_deref()
                .map_or_else(String::new, |note| format!(" ({note})"));
            text.push_str(&format!(
                "{} — {} cases{note}\n",
                row.item,
                super::image_handoff::group_indian_digits(u64::from(row.cases.get()))
            ));
        }
        text.push_str(&format!("\nTotal: {} items", list.rows.len()));
        Ok(Some(text))
    }

    /// Reopens the Owner's manual list, without reading or changing billing stock.
    pub fn open_manual_godown(
        &self,
        role: Role,
    ) -> Result<Vec<&ManualGodownRow>, ManualGodownError> {
        gate(role, crate::permissions::Capability::Godown)?;
        Ok(self.manual_godown.values().collect())
    }

    /// Adds or updates a manual row by trimmed, case-insensitive name. A blank update note
    /// leaves the previous note intact. The synthetic form accepts only whole decimal cases.
    pub fn save_manual_godown(
        &mut self,
        role: Role,
        input: ManualGodownInput<'_>,
    ) -> Result<(), ManualGodownError> {
        gate(role, crate::permissions::Capability::SaveGodown)?;
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
    use super::ManualGodownInput;
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
        // DEC-034: a Manager keeps the manual list too.
        assert_eq!(fixture.manual_godown_list(Role::Manager, "SYNTH"), Ok(list));
    }
}

#[cfg(test)]
mod copy_tests {
    use super::ManualGodownInput;
    use crate::confirmation::demo_fixture::fixture;
    use crate::{BillDate, Role};

    #[test]
    fn copy_text_follows_copy_godown_list() {
        let mut f = fixture().unwrap();
        let day = BillDate::new(2025, 4, 30).unwrap();
        assert_eq!(f.manual_godown_copy_text(Role::Owner, day), Ok(None));
        for (item, cases, note) in [("Soda", "1200", "back rack"), ("Ice", "2", "")] {
            f.save_manual_godown(Role::Owner, ManualGodownInput { item, cases, note })
                .unwrap();
        }
        assert_eq!(
            f.manual_godown_copy_text(Role::Owner, day)
                .unwrap()
                .as_deref(),
            Some(
                "*Godown Stock* (30-Apr-2025)\n\nIce — 2 cases\nSoda — 1,200 cases (back rack)\n\nTotal: 2 items"
            )
        );
        // DEC-034: a Manager copies the same list.
        assert_eq!(
            f.manual_godown_copy_text(Role::Manager, day),
            f.manual_godown_copy_text(Role::Owner, day)
        );
    }
}
