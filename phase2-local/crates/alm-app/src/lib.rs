//! One-screen Tauri trial: a Manager draft, an Owner review and a standalone GST invoice.
//! Every rule lives in `alm-core`/`alm-store`; the page only displays returned HTML.
#![forbid(unsafe_code)]

mod backup;
#[cfg(test)]
mod batch2;
#[cfg(test)]
mod batch3;
#[cfg(test)]
mod batch4;
mod bill_entry;
mod billing;
#[cfg(test)]
mod billing_tests;
mod cashbook;
mod commands;
mod copy;
mod crates;
mod credentials;
#[cfg(test)]
mod credentials_tests;
mod customers;
mod dev_authority;
mod device;
#[cfg(test)]
mod device_tests;
mod file_pick;
mod godown;
mod gst_migration;
mod gst_print;
mod gst_sales;
mod image;
mod ipc;
mod legacy_import;
mod order;
mod outputs;
mod products;
mod purchases;
mod receipts;
mod refusals;
mod render;
#[cfg(test)]
mod rest_e2e;
#[cfg(test)]
mod restart_tests;
#[cfg(test)]
mod scale_tests;
#[cfg(test)]
mod screen_tests;
mod settings;
#[cfg(test)]
mod settings_tests;
mod stock;
mod suppliers;
#[cfg(test)]
mod total_invariant_tests;
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

fn refusal(error: impl std::fmt::Debug) -> String {
    refusals::user_message(&format!("{error:?}"))
}

fn with<T>(
    state: &State<'_, AppState>,
    tab: &str,
    change: impl FnOnce(&mut Trial) -> Result<T, String>,
) -> Result<String, String> {
    let mut trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    if trial.revoked {
        // Blocked: nothing runs; the page says why and offers another account.
        return trial.render(tab);
    }
    change(&mut trial)?;
    trial.render(tab)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(target_os = "android")]
    let builder = builder.plugin(credentials::plugin());
    builder
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            let mut trial = Trial::open(&root, None).inspect_err(|error| {
                eprintln!("ALM setup failed at {}: {error}", root.display());
            })?;
            #[cfg(mobile)]
            {
                trial.outputs = app.path().app_cache_dir()?.join("outputs");
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                trial.credentials = Box::new(credentials::PlatformStore::new(credentials::APP_ID));
            }
            #[cfg(target_os = "android")]
            {
                let handle = app.state::<credentials::SecretsHandle>().0.clone();
                trial.credentials = Box::new(credentials::PlatformStore(handle));
            }
            // Signing in again talks to the auth service: done by the first `start`, not here,
            // so the page gets the app's state at once.
            trial.resume_pending = true;
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
            bill_entry::crate_row::bill_crate_add,
            bill_entry::crate_row::bill_crate_quantity,
            bill_entry::crate_row::bill_crate_remove,
            bill_entry::crate_row::bill_crate_post,
            commands::review_role,
            dev_authority::settings_rest,
            dev_authority::settings_rest_off,
            dev_authority::settings_rest_off_signed_out,
            dev_authority::rest_login,
            dev_authority::rest_logout,
            device::account::outbox_check,
            device::account::outbox_retry,
            device::account::account_switch,
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
            stock::stock_count,
            stock::stock_adjust,
            stock::stock_adjust_close,
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
            products::product_delete,
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
            legacy_import::legacy_dry_run,
            legacy_import::legacy_import_confirm,
            legacy_import::legacy_close,
            legacy_import::legacy_resolve,
            file_pick::file_pick,
            file_pick::file_picked,
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
