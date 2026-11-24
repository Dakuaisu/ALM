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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tagline: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub owner: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mobile: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
    /// "Default Loading ₹/pkt" (`loadingCharge`), in paise.
    #[serde(skip_serializing_if = "is_zero")]
    pub loading_charge_minor: i64,
    pub fssai: String,
    pub gstin: String,
    pub show_fssai: Option<bool>,
    pub show_gstin: Option<bool>,
    pub version: u64,
    pub modified_at: u64,
    pub device: String,
}

const fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// The Settings "Business Profile" form as typed (checkboxes are `"on"` when ticked).
#[derive(Clone, Copy, Debug, Default)]
pub struct ProfileInput<'a> {
    pub name: &'a str,
    pub tagline: &'a str,
    pub owner: &'a str,
    pub mobile: &'a str,
    pub address: &'a str,
    pub gstin: &'a str,
    pub fssai: &'a str,
    pub show_fssai: &'a str,
    pub show_gstin: &'a str,
    pub loading: &'a str,
}

/// A profile form field the core rejects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileError {
    LoadingCharge,
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

    /// Applies the Save Profile form (`Object.assign(m, {...})`): text fields are kept as typed
    /// except that GSTIN/FSSAI are trimmed; the loading charge is blank-as-zero rupees.
    ///
    /// # Errors
    /// Rejects a loading charge that is not exact non-negative rupees.
    pub fn with_input(&self, input: &ProfileInput<'_>) -> Result<Self, ProfileError> {
        let loading = if input.loading.trim().is_empty() {
            0
        } else {
            crate::Money::parse_rupees(input.loading)
                .map_err(|_| ProfileError::LoadingCharge)?
                .minor_units()
        };
        Ok(Self {
            name: input.name.to_owned(),
            tagline: input.tagline.to_owned(),
            owner: input.owner.to_owned(),
            mobile: input.mobile.to_owned(),
            address: input.address.to_owned(),
            loading_charge_minor: loading,
            gstin: input.gstin.trim().to_owned(),
            fssai: input.fssai.trim().to_owned(),
            show_fssai: Some(input.show_fssai == "on"),
            show_gstin: Some(input.show_gstin == "on"),
            ..self.clone()
        })
    }

    /// The loading charge as money, for display.
    pub const fn loading_charge(&self) -> crate::Money {
        crate::Money::from_minor_units(self.loading_charge_minor)
    }

    /// The seller header (name, tagline, address, mobile): each saved field, or the given
    /// fallback while the field is blank.
    pub fn header<'a>(&'a self, fallback: [&'a str; 4]) -> [&'a str; 4] {
        let pick = |own: &'a str, other: &'a str| if own.trim().is_empty() { other } else { own };
        [
            pick(&self.name, fallback[0]),
            pick(&self.tagline, fallback[1]),
            pick(&self.address, fallback[2]),
            pick(&self.mobile, fallback[3]),
        ]
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

    #[test]
    fn profile_form_saves_every_html_field_and_keeps_blank_fields_out_of_the_archive() {
        use super::{ProfileError, ProfileInput};
        let base = BusinessProfile {
            version: 3,
            ..BusinessProfile::default()
        };
        assert_eq!(
            serde_json::to_string(&base).unwrap(),
            r#"{"fssai":"","gstin":"","show_fssai":null,"show_gstin":null,"version":3,"modified_at":0,"device":""}"#
        );
        let input = ProfileInput {
            name: "ALM Test",
            tagline: "Cold drinks",
            owner: "Owner A",
            mobile: "9000000000",
            address: "Line 1\nLine 2",
            gstin: " 09AAAAA0000A1Z5 ",
            fssai: "",
            show_fssai: "",
            show_gstin: "on",
            loading: "1.50",
        };
        let saved = base.with_input(&input).unwrap();
        assert_eq!(saved.version, 3);
        assert_eq!(saved.gstin, "09AAAAA0000A1Z5");
        assert_eq!(saved.loading_charge().minor_units(), 150);
        assert_eq!(saved.shows(), (false, true));
        assert_eq!(
            saved.header(["a", "b", "c", "d"]),
            ["ALM Test", "Cold drinks", "Line 1\nLine 2", "9000000000"]
        );
        assert_eq!(
            BusinessProfile::default().header(["a", "b", "c", "d"]),
            ["a", "b", "c", "d"]
        );
        let json = serde_json::to_string(&saved).unwrap();
        let back: BusinessProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, saved);
        for bad in ["-1", "x", "1.234"] {
            let input = ProfileInput {
                loading: bad,
                ..input
            };
            assert_eq!(base.with_input(&input), Err(ProfileError::LoadingCharge));
        }
        let blank = ProfileInput::default();
        assert_eq!(base.with_input(&blank).unwrap().loading_charge_minor, 0);
    }
}
