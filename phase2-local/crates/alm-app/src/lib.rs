//! One-screen Tauri trial: a Manager draft, an Owner review and a standalone GST invoice.
//! Every rule lives in `alm-core`/`alm-store`; the page only displays returned HTML.
#![forbid(unsafe_code)]

mod backup;
#[cfg(test)]
mod batch2;
#[cfg(test)]
mod batch3;
mod bill_entry;
mod billing;
#[cfg(test)]
mod billing_tests;
mod cashbook;
mod commands;
mod copy;
mod crates;
mod customers;
mod device;
#[cfg(test)]
mod device_tests;
mod godown;
mod gst_print;
mod gst_sales;
mod image;
mod ipc;
mod order;
mod outputs;
mod products;
mod purchases;
mod receipts;
mod render;
#[cfg(test)]
mod screen_tests;
mod settings;
#[cfg(test)]
mod settings_tests;
mod stock;
mod suppliers;
mod trial;
mod views;

#[cfg(test)]
use std::num::NonZeroU32;
use std::sync::Mutex;

#[cfg(test)]
use alm_core::confirmation::ConfirmationFixture;
#[cfg(test)]
use alm_core::{BillDate, DraftId, ProvisionalDraft, Role};
#[cfg(test)]
use alm_core::{DraftCustomer, DraftLine, ProductId};
use tauri::{Manager, State};

#[cfg(test)]
pub(crate) use trial::TRIAL_BILL;
pub(crate) use trial::{AppState, SELLER, Trial};

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn with<T>(
    state: &State<'_, AppState>,
    tab: &str,
    change: impl FnOnce(&mut Trial) -> Result<T, String>,
) -> Result<String, String> {
    let mut trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    change(&mut trial)?;
    trial.render(tab)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            #[cfg_attr(desktop, allow(unused_mut))]
            let mut trial = Trial::open(&root, None)?;
            #[cfg(mobile)]
            {
                trial.outputs = app.path().app_cache_dir()?.join("outputs");
            }
            app.manage(Mutex::new(trial));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start,
            commands::screen,
            bill_entry::commands::bill_fresh,
            bill_entry::commands::bill_header,
            bill_entry::commands::bill_add,
            bill_entry::commands::bill_quantity,
            bill_entry::commands::bill_remove,
            bill_entry::commands::bill_open,
            bill_entry::commands::bill_live,
            bill_entry::commands::bill_customer_search,
            bill_entry::commands::bill_item_search,
            commands::review_role,
            device::outbox_check,
            device::outbox_retry,
            device::account_switch,
            billing::sale_prepare,
            billing::sale_confirm,
            billing::sales_filter,
            billing::sale_cancel,
            billing::sale_restore,
            billing::sale_edit_open,
            billing::sale_edit_close,
            billing::sale_edit_save,
            outputs::platform::bill_image_share,
            outputs::platform::open_url,
            outputs::export::export_file,
            outputs::export::print_open,
            settings::settings_profile,
            settings::settings_theme,
            settings::settings_balance,
            settings::settings_privacy,
            backup::backup_create,
            backup::backup_restore,
            receipts::receipt_save,
            customers::customers_filter,
            customers::customer_open,
            customers::customer_opening,
            customers::customer_close,
            cashbook::cash_entry,
            cashbook::cash_new,
            cashbook::cash_edit,
            cashbook::cash_close,
            cashbook::cash_live,
            commands::dash_period,
            copy::reorder_copy_text,
            copy::rate_copy_text,
            copy::rate_whatsapp_url,
            copy::godown_copy_text,
            copy::receipt_customer_search,
            copy::gst_item_search,
            copy::godown_item_search,
            stock::stock_filter,
            stock::stock_lots,
            stock::stock_lots_close,
            godown::godown_save,
            godown::godown_filter,
            gst_sales::gst_new,
            gst_sales::gst_edit,
            gst_sales::gst_submit,
            gst_sales::gst_back,
            gst_sales::gst_cancel,
            gst_sales::gst_delete,
            gst_sales::gst_view,
            gst_sales::gst_customer_search,
            products::products_filter,
            products::product_new,
            products::product_edit,
            products::product_save,
            products::product_close,
            products::product_preview,
            suppliers::supplier_add,
            suppliers::supplier_quotes,
            order::order_add,
            order::order_qty,
            order::order_remove,
            order::order_low,
            order::order_checkout,
            order::order_po,
            order::order_discard,
            purchases::po_receive,
            purchases::po_receive_close,
            purchases::po_receive_save,
            purchases::po_close,
            crates::crate_new,
            crates::crate_edit,
            crates::crate_submit,
            crates::crate_close,
            crates::crate_delete,
            commands::rate_brand,
            commands::rate_group,
            commands::rate_search,
            commands::rate_toggle,
            commands::print_page,
            commands::share_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ALM trial");
}
