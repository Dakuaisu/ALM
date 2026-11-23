//! Synthetic, Owner-scoped read model; it cannot post sales or authorize transactions.
pub(crate) mod date;
mod estimate;
mod export;
mod gst;
mod purchase_order;
mod scope;
mod storage;

use serde::{Deserialize, Serialize};

pub use crate::confirmation::estimate::{EstimateDocument, EstimateRow};
pub use gst::{GstArchiveError, HistoricalGst, HistoricalGstLine};
pub(crate) use purchase_order::money;
#[doc(hidden)]
pub use purchase_order::validate_orders;
pub use purchase_order::{
    ArchivedPurchaseBusiness, ArchivedPurchaseLine, ArchivedPurchaseOrder, ArchivedPurchaseReceipt,
    ArchivedPurchaseStatus, PurchaseCapture, PurchaseDisplay, ScopedArchiveInput,
};
pub use scope::{AccountId, AccountScope, ReceiptId, SaleId};
pub use storage::{Error, ReadModel};

/// Local synthetic profile and legacy-shaped metadata; not a sync authority.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BusinessProfile {
    pub fssai: String,
    pub gstin: String,
    pub show_fssai: Option<bool>,
    pub show_gstin: Option<bool>,
    pub version: u64,
    pub modified_at: u64,
    pub device: String,
}

/// Settings "Default Reorder Level": blank means 5, otherwise a whole number of packets
/// (`String(ro.value).trim() === "" ? 5 : Math.max(0, +ro.value || 0)`).
pub fn parse_reorder_default(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        return Some(5);
    }
    text.bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| text.parse().ok())
        .flatten()
}

/// Which settings stay on this device and which travel with the shop archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingScope {
    /// `alm_core::preferences` keys (theme, privacy, balance display): never in the archive.
    DeviceLocal,
    /// Business profile and default reorder level: saved in the archive, backed up and restored.
    Shared,
}

impl BusinessProfile {
    /// The form's checkbox defaults: FSSAI shown unless turned off, GSTIN hidden unless on.
    pub const fn shows(&self) -> (bool, bool) {
        (
            !matches!(self.show_fssai, Some(false)),
            matches!(self.show_gstin, Some(true)),
        )
    }

    pub fn estimate_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.fssai.is_empty() && self.show_fssai != Some(false) {
            lines.push(format!("FSSAI: {}", self.fssai));
        }
        if !self.gstin.is_empty() && self.show_gstin == Some(true) {
            lines.push(format!("GSTIN: {}", self.gstin));
        }
        lines
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Records {
    pub reorder_default: u32,
    pub products: Vec<Product>,
    pub customers: Vec<Customer>,
    pub sales: Vec<Sale>,
    pub receipts: Vec<Receipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub item: String,
    pub company: String,
    pub pack: String,
    pub uom: String,
    pub packet_quantity: u32,
    pub mrp_minor: i64,
    pub buy_minor: i64,
    pub sell_minor: i64,
    pub purchased: u32,
    pub sold: u32,
    pub on_hand: i64,
    pub reorder: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    pub name: String,
    pub mobile: String,
    pub address: String,
    pub balance_minor: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sale {
    pub id: String,
    #[serde(with = "date")]
    pub date: crate::BillDate,
    pub number: u64,
    pub customer: String,
    pub mode: String,
    pub lines: Vec<Line>,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub item: String,
    pub pack: String,
    pub uom: String,
    pub packet_quantity: u32,
    pub kind: LineKind,
    pub rate_minor: i64,
    pub quantity: u32,
    pub amount_minor: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LineKind {
    Item,
    Crate { components: Vec<Component> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub item: String,
    pub quantity: u32,
}

impl Line {
    fn display_item(&self) -> String {
        match self.kind {
            LineKind::Item => self.item.clone(),
            LineKind::Crate { .. } => format!("Crate: {}", self.item),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub id: String,
    #[serde(with = "date")]
    pub date: crate::BillDate,
    pub mode: String,
    pub customer: String,
    pub amount_minor: i64,
}

#[cfg(test)]
mod settings_tests {
    use super::{BusinessProfile, parse_reorder_default};

    #[test]
    fn reorder_default_and_profile_checkbox_defaults_follow_the_html() {
        assert_eq!(parse_reorder_default(" "), Some(5));
        assert_eq!(parse_reorder_default("0"), Some(0));
        assert_eq!(parse_reorder_default("12"), Some(12));
        assert_eq!(parse_reorder_default("-1"), None);
        assert_eq!(parse_reorder_default("1.5"), None);
        assert_eq!(BusinessProfile::default().shows(), (true, false));
        let set = BusinessProfile {
            show_fssai: Some(false),
            show_gstin: Some(true),
            ..BusinessProfile::default()
        };
        assert_eq!(set.shows(), (false, true));
    }
}
