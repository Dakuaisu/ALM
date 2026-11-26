use crate::Trial;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Access {
    Public,
    Session,
    Owner,
    Simulation,
    Switch,
}

pub(crate) fn classify(command: &str) -> Option<Access> {
    Some(match command {
        "start" | "screen" | "rest_login" | "rest_logout" | "settings_rest_off_signed_out" => {
            Access::Public
        }
        "account_switch" => Access::Switch,
        "review_role" => Access::Simulation,
        "bill_fresh"
        | "bill_header"
        | "bill_add"
        | "bill_quantity"
        | "bill_remove"
        | "bill_open"
        | "bill_live"
        | "bill_customer_search"
        | "bill_item_search"
        | "settings_theme"
        | "settings_balance"
        | "settings_privacy" => Access::Session,
        "bill_crate_add"
        | "bill_crate_quantity"
        | "bill_crate_remove"
        | "bill_crate_post"
        | "settings_rest"
        | "settings_rest_off"
        | "outbox_check"
        | "outbox_retry"
        | "sale_prepare"
        | "sale_confirm"
        | "sales_filter"
        | "sale_cancel"
        | "sale_restore"
        | "sale_edit_open"
        | "sale_edit_close"
        | "sale_edit_save"
        | "bill_image_share"
        | "open_url"
        | "export_file"
        | "print_open"
        | "settings_profile"
        | "backup_create"
        | "backup_restore"
        | "receipt_save"
        | "customers_filter"
        | "customer_open"
        | "customer_opening"
        | "customer_close"
        | "cash_entry"
        | "cash_new"
        | "cash_edit"
        | "cash_close"
        | "cash_live"
        | "dash_period"
        | "reorder_copy_text"
        | "rate_copy_text"
        | "rate_whatsapp_url"
        | "godown_copy_text"
        | "receipt_customer_search"
        | "gst_item_search"
        | "godown_item_search"
        | "stock_filter"
        | "stock_lots"
        | "stock_lots_close"
        | "stock_count"
        | "stock_adjust"
        | "stock_adjust_close"
        | "godown_save"
        | "godown_filter"
        | "gst_new"
        | "gst_edit"
        | "gst_submit"
        | "gst_back"
        | "gst_cancel"
        | "gst_delete"
        | "gst_view"
        | "gst_customer_search"
        | "products_filter"
        | "product_new"
        | "product_edit"
        | "product_save"
        | "product_close"
        | "product_delete"
        | "product_preview"
        | "supplier_add"
        | "supplier_quotes"
        | "order_add"
        | "order_qty"
        | "order_remove"
        | "order_low"
        | "order_checkout"
        | "order_po"
        | "order_discard"
        | "po_receive"
        | "po_receive_close"
        | "po_receive_save"
        | "po_close"
        | "crate_new"
        | "crate_edit"
        | "crate_submit"
        | "crate_close"
        | "crate_delete"
        | "legacy_dry_run"
        | "legacy_import_confirm"
        | "legacy_close"
        | "legacy_resolve"
        | "file_pick"
        | "file_picked"
        | "rate_brand"
        | "rate_group"
        | "rate_search"
        | "rate_toggle"
        | "print_page"
        | "share_text" => Access::Owner,
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
        Access::Session | Access::Owner | Access::Simulation => {
            if trial.revoked {
                return Err(crate::trial::REVOKED.into());
            }
            if trial.signed_out() {
                return Err("Sign in first.".into());
            }
            if access == Access::Owner {
                trial.require_owner()?;
                if let Some(session) = &trial.session
                    && session.role() != Some(alm_core::Role::Owner)
                {
                    return Err("Access denied: Owner only.".into());
                }
            }
            Ok(())
        }
    }
}
