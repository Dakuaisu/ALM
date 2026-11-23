//! SQLite draft storage and file-backed archives for the local synthetic slice.
#![forbid(unsafe_code)]

pub use alm_core::*;

#[cfg(test)]
mod manager_account_tests;
mod manager_drafts;
pub mod preferences;
mod sqlite;
pub mod synthetic_read_model;

pub use manager_drafts::{ManagerDraftStore, SqliteSales};
pub use sqlite::GstBillDatabase;
#[doc(hidden)]
pub use sqlite::SqliteDraftStore;
