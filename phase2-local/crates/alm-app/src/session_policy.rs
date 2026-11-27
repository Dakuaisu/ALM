//! Who may run each registered command: the DEC-034 matrix (`alm_core::permissions`) through one
//! capability per command. The authority checks the same matrix again; this gate only keeps the
//! app from offering what it would refuse.
use alm_core::permissions::{self, Capability as C};

use crate::Trial;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Access {
    Public,
    Simulation,
    Switch,
    /// Signed in, not revoked, and the account's role may use the capability.
    Gate(C),
}

pub(crate) fn classify(command: &str) -> Option<Access> {
    let capability = match command {
        "start" | "screen" | "rest_login" | "rest_logout" | "settings_rest_off_signed_out" => {
            return Some(Access::Public);
        }
        "account_switch" => return Some(Access::Switch),
        "review_role" => return Some(Access::Simulation),
        "bill_fresh"
        | "bill_header"
        | "bill_add"
        | "bill_quantity"
        | "bill_remove"
        | "bill_open"
        | "bill_live"
        | "bill_customer_search"
        | "bill_item_search"
        | "bill_crate_add"
        | "bill_crate_quantity"
        | "bill_crate_remove" => C::NewBill,
        "bill_crate_post" => C::SellCrate,
        "bill_post" => C::PostSale,
        "settings_theme" | "settings_balance" | "settings_privacy" => C::DeviceAppearance,
        "settings_rest" | "settings_rest_off" => C::AuthorityConnection,
        "outbox_check" | "outbox_retry" => C::RequestStatus,
        "sale_prepare" | "sale_confirm" => C::OwnerReview,
        "sales_filter" => C::Sales,
        "sale_cancel" => C::CancelSale,
        "sale_restore" => C::RestoreSale,
        "sale_edit_open" | "sale_edit_close" | "sale_edit_save" => C::EditSale,
        "bill_image_share" | "open_url" => C::ShareBill,
        "export_file" => C::Export,
        // The PO print view is checked again for its own row (`PrintPurchaseOrder`).
        "print_open" => C::PrintEstimate,
        "print_page" => C::PrintBill,
        "settings_profile" => C::Settings,
        "backup_create" | "backup_restore" => C::Backup,
        "receipt_save" | "receipt_customer_search" => C::RecordReceipt,
        "customers_filter" | "customer_close" => C::Customers,
        "customer_edit" | "customer_save" => C::SaveCustomer,
        "customer_open" => C::CustomerLedger,
        "customer_opening" => C::SaveOpening,
        "cash_entry" | "cash_new" | "cash_close" | "cash_live" => C::CashBook,
        "cash_edit" => C::EditCashDay,
        "dash_period" => C::Dashboard,
        "reorder_copy_text" => C::Reorder,
        "rate_copy_text" | "rate_whatsapp_url" | "rate_brand" | "rate_group" | "rate_search"
        | "rate_toggle" => C::RateList,
        "godown_copy_text" | "godown_item_search" | "godown_filter" => C::Godown,
        "godown_save" => C::SaveGodown,
        "gst_item_search"
        | "gst_new"
        | "gst_edit"
        | "gst_submit"
        | "gst_back"
        | "gst_cancel"
        | "gst_delete"
        | "gst_view"
        | "gst_customer_search"
        | "share_text" => C::GstSales,
        "stock_filter" => C::Stock,
        "stock_lots" | "stock_lots_close" => C::StockLots,
        "stock_count" | "stock_adjust" | "stock_adjust_close" => C::CountStock,
        "products_filter" | "product_new" | "product_edit" | "product_save" | "product_close"
        | "product_delete" | "product_preview" => C::Products,
        "supplier_add" => C::AddSupplier,
        "supplier_quotes" => C::SetSupplierPrices,
        "order_add" | "order_qty" | "order_remove" | "order_low" | "order_checkout"
        | "order_discard" => C::OrderPlanner,
        "order_po" => C::SavePurchaseOrder,
        "po_receive" | "po_receive_close" => C::Purchases,
        "po_receive_save" => C::ReceiveGoods,
        "po_close" => C::ClosePurchaseOrder,
        "crate_new" | "crate_edit" | "crate_close" => C::Crates,
        "crate_submit" => C::SaveCrate,
        "crate_delete" => C::DeleteCrate,
        "legacy_dry_run"
        | "legacy_import_confirm"
        | "legacy_close"
        | "legacy_resolve"
        | "file_pick"
        | "file_picked" => C::LegacyImport,
        _ => return None,
    };
    Some(Access::Gate(capability))
}

/// The capability a screen tab needs (`None`: not a screen).
pub(crate) fn screen(tab: &str) -> Option<C> {
    Some(match tab {
        "bill" | "draft" => C::NewBill,
        "review" => C::OwnerReview,
        "sales" => C::Sales,
        "counter" => C::Counter,
        "customers" => C::Customers,
        "receipt" => C::RecordReceipt,
        "expenses" => C::CashBook,
        "dashboard" => C::Dashboard,
        "reorder" => C::Reorder,
        "stock" => C::Stock,
        "godown" => C::Godown,
        "gstsales" | "gst" => C::GstSales,
        "products" => C::Products,
        "suppliers" => C::Suppliers,
        "order" => C::OrderPlanner,
        "purchases" => C::Purchases,
        "crates" => C::Crates,
        "ratelist" | "rateprint" => C::RateList,
        "reports" => C::Reports,
        // A Manager's Settings tab holds this device's appearance only.
        "settings" => C::DeviceAppearance,
        "estimate" => C::PrintEstimate,
        "poprint" => C::PrintPurchaseOrder,
        _ => return None,
    })
}

pub(crate) fn authorize(trial: &Trial, access: Access) -> Result<(), String> {
    match access {
        Access::Public => Ok(()),
        Access::Switch if trial.rest_config.is_none() && trial.session.is_none() => Ok(()),
        Access::Switch => Err("Sign out to switch accounts.".into()),
        Access::Simulation if trial.rest_config.is_some() || trial.session.is_some() => {
            Err("The role comes from your account.".into())
        }
        Access::Simulation => trial.require_session(),
        Access::Gate(capability) => trial.require(capability).map(drop),
    }
}

impl Trial {
    /// Signed in and not revoked.
    pub(crate) fn require_session(&self) -> Result<(), String> {
        if self.revoked {
            return Err(crate::trial::REVOKED.into());
        }
        if self.signed_out() {
            return Err("Sign in first.".into());
        }
        Ok(())
    }

    /// The role, when it may use `capability` (DEC-034); a signed-in session acts with the
    /// lesser of the account's role and the role the device shows.
    pub(crate) fn require(&self, capability: C) -> Result<alm_core::Role, String> {
        self.require_session()?;
        let role = match &self.session {
            Some(session) if session.role() != Some(self.role()) => alm_core::Role::Manager,
            _ => self.role(),
        };
        if permissions::allows(role, capability) {
            Ok(role)
        } else {
            Err(crate::refusals::OWNER_ONLY.into())
        }
    }
}
