//! DEC-034: the Owner/Manager permission matrix, the one table the core, the simulated authority,
//! the app and the SQL role checks (migration `20261204000000_permission_matrix.sql`, compared by
//! a test) are read from. Anything not listed is denied to both roles.
use crate::Role;

/// The refusal a Manager reads for an Owner-only action (DEC-034; the HTML has no roles).
pub const OWNER_ONLY: &str = "Only the owner can do this.";

/// Who may use a capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Allowed {
    Owner,
    OwnerAndManager,
}

/// The rule the authority applies to a Manager on top of the matrix row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Limit {
    None,
    /// The authority's date in the shop's time zone (Asia/Kolkata), never an earlier day.
    TodayOnly,
    /// Quantities only: the purchase order's own rates are used; none is sent or shown.
    QuantitiesOnly,
    /// Name, mobile and address; never the opening balance.
    ContactOnly,
    /// Only requests the same account sent.
    OwnRequests,
    /// Rows without costs, buy rates, supplier prices, FIFO lots, stock value or profit.
    NoCosts,
}

/// Every command and screen of the app and the authority.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    // Authority commands.
    PostSale,
    SellCrate,
    EditSale,
    CancelSale,
    RestoreSale,
    RecordReceipt,
    SaveCustomer,
    SaveOpening,
    CloseCashDay,
    EditCashDay,
    DeleteCashDay,
    SaveProduct,
    DeleteProduct,
    AddSupplier,
    SetSupplierPrices,
    SavePurchaseOrder,
    ReceiveGoods,
    ReceiveGoodsAtRates,
    ClosePurchaseOrder,
    SaveCrate,
    DeleteCrate,
    SetReorderDefault,
    RecordRestore,
    ImportLegacy,
    ResolveLegacyQuarantine,
    SaveGodown,
    CountStock,
    SaveGstBill,
    SetGstBillState,
    ImportGstBill,
    AllocateGstNumber,
    RequestStatus,
    RequestLookup,
    Pull,
    // Screens and device actions.
    NewBill,
    OwnerReview,
    Counter,
    Sales,
    Customers,
    CustomerLedger,
    CashBook,
    CashBookHistory,
    Stock,
    StockLots,
    Reorder,
    Godown,
    Crates,
    RateList,
    Purchases,
    ShareBill,
    PrintBill,
    PrintEstimate,
    PrintPurchaseOrder,
    DeviceAppearance,
    Dashboard,
    Reports,
    GstSales,
    Products,
    Suppliers,
    OrderPlanner,
    Settings,
    Backup,
    Export,
    LegacyImport,
    ActivityLog,
    ClearAll,
    AuthorityConnection,
}

/// One matrix row: the capability, who may use it, the Manager's limit and the SQL functions
/// whose role check carries it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Row {
    pub capability: Capability,
    pub allowed: Allowed,
    pub limit: Limit,
    pub sql: &'static [&'static str],
}

const fn row(
    capability: Capability,
    allowed: Allowed,
    limit: Limit,
    sql: &'static [&'static str],
) -> Row {
    Row {
        capability,
        allowed,
        limit,
        sql,
    }
}

use Allowed::{Owner as O, OwnerAndManager as B};
use Capability as C;

/// The matrix (DEC-034), one row per capability.
pub const MATRIX: &[Row] = &[
    row(C::PostSale, B, Limit::None, &["post_sale"]),
    row(C::SellCrate, B, Limit::None, &["sell_crate"]),
    row(C::EditSale, B, Limit::None, &["edit_sale"]),
    row(C::CancelSale, B, Limit::None, &["set_sale_cancelled"]),
    row(C::RestoreSale, O, Limit::None, &["set_sale_cancelled"]),
    row(C::RecordReceipt, B, Limit::None, &["record_receipt"]),
    row(C::SaveCustomer, B, Limit::ContactOnly, &[]),
    row(C::SaveOpening, O, Limit::None, &["save_opening"]),
    row(C::CloseCashDay, B, Limit::TodayOnly, &["save_cash_day"]),
    row(C::EditCashDay, O, Limit::None, &["save_cash_day"]),
    row(C::DeleteCashDay, O, Limit::None, &["save_cash_day"]),
    row(C::SaveProduct, O, Limit::None, &["save_product"]),
    row(C::DeleteProduct, O, Limit::None, &["delete_product"]),
    row(C::AddSupplier, O, Limit::None, &["add_supplier"]),
    row(
        C::SetSupplierPrices,
        O,
        Limit::None,
        &["set_supplier_prices"],
    ),
    row(
        C::SavePurchaseOrder,
        O,
        Limit::None,
        &["save_purchase_order"],
    ),
    row(
        C::ReceiveGoods,
        B,
        Limit::QuantitiesOnly,
        &["receive_purchase_order"],
    ),
    row(
        C::ReceiveGoodsAtRates,
        O,
        Limit::None,
        &["receive_purchase_order"],
    ),
    row(
        C::ClosePurchaseOrder,
        O,
        Limit::None,
        &["close_purchase_order"],
    ),
    row(C::SaveCrate, B, Limit::None, &["save_crate"]),
    row(C::DeleteCrate, O, Limit::None, &["delete_crate"]),
    row(
        C::SetReorderDefault,
        O,
        Limit::None,
        &["set_reorder_default"],
    ),
    row(C::RecordRestore, O, Limit::None, &["record_restore"]),
    row(C::ImportLegacy, O, Limit::None, &["import_legacy"]),
    row(
        C::ResolveLegacyQuarantine,
        O,
        Limit::None,
        &["resolve_legacy_quarantine"],
    ),
    row(C::SaveGodown, B, Limit::None, &["save_godown"]),
    row(C::CountStock, B, Limit::None, &["count_stock"]),
    row(C::SaveGstBill, O, Limit::None, &["save_gst_bill"]),
    row(C::SetGstBillState, O, Limit::None, &["set_gst_bill_state"]),
    row(C::ImportGstBill, O, Limit::None, &["import_gst_bill"]),
    row(
        C::AllocateGstNumber,
        O,
        Limit::None,
        &["allocate_gst_bill_number"],
    ),
    row(C::RequestStatus, B, Limit::OwnRequests, &["request_status"]),
    row(C::RequestLookup, B, Limit::OwnRequests, &["request_lookup"]),
    row(
        C::Pull,
        B,
        Limit::NoCosts,
        &["pull_projection", "pull_changes"],
    ),
    row(C::NewBill, B, Limit::None, &[]),
    row(C::OwnerReview, O, Limit::None, &[]),
    row(C::Counter, B, Limit::NoCosts, &[]),
    row(C::Sales, B, Limit::NoCosts, &[]),
    row(C::Customers, B, Limit::None, &[]),
    row(C::CustomerLedger, O, Limit::None, &[]),
    row(C::CashBook, B, Limit::TodayOnly, &[]),
    row(C::CashBookHistory, O, Limit::None, &[]),
    row(C::Stock, B, Limit::NoCosts, &[]),
    row(C::StockLots, O, Limit::None, &[]),
    row(C::Reorder, B, Limit::NoCosts, &[]),
    row(C::Godown, B, Limit::None, &[]),
    row(C::Crates, B, Limit::None, &[]),
    row(C::RateList, B, Limit::None, &[]),
    row(C::Purchases, B, Limit::QuantitiesOnly, &[]),
    row(C::ShareBill, B, Limit::None, &[]),
    row(C::PrintBill, B, Limit::None, &[]),
    row(C::PrintEstimate, B, Limit::None, &[]),
    row(C::PrintPurchaseOrder, O, Limit::None, &[]),
    row(C::DeviceAppearance, B, Limit::None, &[]),
    row(C::Dashboard, O, Limit::None, &[]),
    row(C::Reports, O, Limit::None, &[]),
    row(C::GstSales, O, Limit::None, &[]),
    row(C::Products, O, Limit::None, &[]),
    row(C::Suppliers, O, Limit::None, &[]),
    row(C::OrderPlanner, O, Limit::None, &[]),
    row(C::Settings, O, Limit::None, &[]),
    row(C::Backup, O, Limit::None, &[]),
    row(C::Export, O, Limit::None, &[]),
    row(C::LegacyImport, O, Limit::None, &[]),
    row(C::ActivityLog, O, Limit::None, &[]),
    row(C::ClearAll, O, Limit::None, &[]),
    row(C::AuthorityConnection, O, Limit::None, &[]),
];

/// The matrix row of `capability`.
pub fn rule(capability: Capability) -> Row {
    MATRIX
        .iter()
        .copied()
        .find(|row| row.capability == capability)
        .unwrap_or(Row {
            capability,
            allowed: Allowed::Owner,
            limit: Limit::None,
            sql: &[],
        })
}

/// True when `role` may use `capability`.
pub fn allows(role: Role, capability: Capability) -> bool {
    match role {
        Role::Owner => MATRIX.iter().any(|row| row.capability == capability),
        Role::Manager => rule(capability).allowed == Allowed::OwnerAndManager,
    }
}

/// [`allows`] as a result.
///
/// # Errors
/// [`crate::AccessDenied`] when the role may not use the capability.
pub fn require(role: Role, capability: Capability) -> Result<(), crate::AccessDenied> {
    if allows(role, capability) {
        Ok(())
    } else {
        Err(crate::AccessDenied)
    }
}

/// Each SQL function's role check: `(function, Manager may call it)`, sorted by name. A
/// function carrying several capabilities admits a Manager when any of them does; the
/// function itself then applies the narrower rule (cancel not restore, today's Cash Book only,
/// quantities only).
pub fn sql_functions() -> Vec<(&'static str, bool)> {
    let mut out: Vec<(&'static str, bool)> = Vec::new();
    for row in MATRIX {
        for function in row.sql {
            let manager = row.allowed == Allowed::OwnerAndManager;
            match out.iter_mut().find(|(name, _)| name == function) {
                Some((_, any)) => *any |= manager,
                None => out.push((function, manager)),
            }
        }
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::{Allowed, Capability, MATRIX, allows, sql_functions};
    use crate::Role;

    #[test]
    fn every_capability_has_exactly_one_row_and_the_owner_may_use_all() {
        for (i, row) in MATRIX.iter().enumerate() {
            assert!(
                MATRIX[i + 1..]
                    .iter()
                    .all(|other| other.capability != row.capability),
                "{:?} twice",
                row.capability
            );
            assert!(allows(Role::Owner, row.capability));
            assert_eq!(
                allows(Role::Manager, row.capability),
                row.allowed == Allowed::OwnerAndManager
            );
        }
        assert_eq!(MATRIX.len(), Capability::AuthorityConnection as usize + 1);
    }

    #[test]
    fn shared_sql_functions_admit_a_manager_when_any_of_their_capabilities_does() {
        let functions = sql_functions();
        let get = |name: &str| functions.iter().find(|(n, _)| *n == name).map(|f| f.1);
        assert_eq!(get("set_sale_cancelled"), Some(true));
        assert_eq!(get("save_cash_day"), Some(true));
        assert_eq!(get("receive_purchase_order"), Some(true));
        assert_eq!(get("save_opening"), Some(false));
        assert_eq!(get("pull_changes"), Some(true));
        assert_eq!(get("allocate_gst_bill_number"), Some(false));
    }
}
