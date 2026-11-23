//! One-screen Tauri trial: a Manager draft, an Owner review and a standalone GST invoice.
//! Every rule lives in `alm-core`/`alm-store`; the page only displays returned HTML.
#![forbid(unsafe_code)]

mod backup;
#[cfg(test)]
mod batch2;
mod bill_entry;
mod billing;
#[cfg(test)]
mod billing_tests;
mod cashbook;
mod copy;
mod crates;
mod customers;
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
#[cfg(test)]
mod screen_tests;
mod settings;
mod stock;
mod suppliers;
mod views;

#[cfg(test)]
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Mutex;

use alm_core::confirmation::demo_fixture;
use alm_core::confirmation::manual_cashbook::{ManualCashBook, ManualCashBookDraft};
use alm_core::confirmation::owner_billing::{self, BillingDirectory, OwnerPosting};
use alm_core::confirmation::{
    ConfirmationFixture, GstBillDraft, GstBillId, GstBillLineInput, GstUnit,
};
use alm_core::{BillDate, DraftId, LocalSales, Money, ProvisionalDraft, Role};
#[cfg(test)]
use alm_core::{DraftCustomer, DraftLine, ProductId};
use alm_store::{GstBillDatabase, ManagerDraftStore, SqliteSales};
use tauri::{Manager, State};

/// One synthetic account per device for the trial.
const ACCOUNT: &str = "trial";
const SELLER: gst_print::Seller<'static> = gst_print::Seller {
    name: "ALM Enterprises (synthetic)",
    address: "Synthetic Market Road, Lucknow",
    fssai: "",
    gstin: "09ABCDE1234F1Z5",
};
const TRIAL_BILL: GstBillId = GstBillId(1);
const GST_LINES: [GstBillLineInput<'static>; 2] = [
    GstBillLineInput {
        item: "Synthetic Cola 600 ml",
        hsn: "22029090",
        quantity: 2,
        inclusive_rate: Money::from_minor_units(14_000),
        gst_basis_points: 4_000,
        packet_qty: 24,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Synthetic Water 1 L",
        unit: GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    },
];

struct Trial {
    drafts: ManagerDraftStore,
    directory: BillingDirectory,
    /// The device's local date, sent by the page at start; tests pass a fixed date.
    today: Option<BillDate>,
    authority: ConfirmationFixture,
    current: Option<(ProvisionalDraft, bool)>,
    owner: bool,
    message: String,
    posting: OwnerPosting,
    sales_filter: views::Filter,
    customers: views::CustomersState,
    /// In-memory manual Cash Book (no alm-store table; Supabase later).
    cashbook: ManualCashBook,
    cash_form: Option<(ManualCashBookDraft, Option<u64>)>,
    dash_period: alm_core::confirmation::DashboardPeriod,
    stock: views::StockState,
    godown_query: String,
    gst_db: GstBillDatabase,
    gst_form: Option<alm_core::confirmation::GstBillFields>,
    /// The bill shown on the invoice view.
    gst_view: GstBillId,
    products: views::ProductsState,
    order: views::OrderState,
    receiving: Option<alm_core::confirmation::PurchaseOrderId>,
    crate_form: Option<views::CrateForm>,
    rates: alm_core::confirmation::rate_list::RateListSelection,
    sale_edit: Option<DraftId>,
    /// Generated files (bill images); on Android the cache dir, shareable via FileProvider.
    outputs: std::path::PathBuf,
    /// Device-local presentation preferences (one file per key).
    prefs: alm_store::preferences::Preferences,
    /// The shop archive holding shared settings (business profile, default reorder level).
    shop: alm_store::synthetic_read_model::Store,
    device_root: std::path::PathBuf,
    /// The last restored backup, opened read-only at a fresh path.
    restored: Option<(String, alm_store::synthetic_read_model::Store)>,
}

type AppState = Mutex<Trial>;

impl Trial {
    fn open(device_root: &Path, today: Option<BillDate>) -> Result<Self, String> {
        let drafts = LocalSales::open_manager_account(device_root, ACCOUNT).map_err(text)?;
        let mut gst = GstBillDatabase::open_scoped(device_root, ACCOUNT).map_err(text)?;
        let mut authority = demo_fixture::fixture().map_err(debug)?;
        demo_fixture::seed_purchasing(&mut authority).map_err(debug)?;
        authority
            .load_gst_bills_from(Role::Owner, &gst)
            .map_err(debug)?;
        let ledger = authority.gst_bill_ledger(Role::Owner).map_err(debug)?;
        if ledger.bills.is_empty() && ledger.deleted.is_empty() {
            let draft = GstBillDraft {
                customer: "Synthetic GST Buyer",
                cust_addr: "12 Synthetic Lane, Kanpur",
                transporter: "Synthetic Roadways",
                lines: &GST_LINES,
                ..authority
                    .new_gst_bill_draft(
                        Role::Owner,
                        TRIAL_BILL,
                        BillDate::new(2025, 4, 2).map_err(debug)?,
                    )
                    .map_err(debug)?
            };
            authority
                .save_gst_bill_to(Role::Owner, &draft, &mut gst, &mut |_| {})
                .map_err(debug)?;
        }
        let directory = demo_fixture::directory();
        let prefs_dir = device_root.join("preferences");
        std::fs::create_dir_all(&prefs_dir).map_err(text)?;
        let shop_path = device_root.join("shop.json");
        let shop = if shop_path.exists() {
            alm_store::synthetic_read_model::Store::open(&shop_path, Role::Owner).map_err(debug)?
        } else {
            let records = authority
                .archive_records(Role::Owner, &directory)
                .map_err(debug)?;
            alm_store::synthetic_read_model::Store::create(&shop_path, Role::Owner, records)
                .map_err(debug)?
        };
        shop.apply_product_settings(Role::Owner, &mut authority)
            .map_err(debug)?;
        Ok(Self {
            drafts,
            directory,
            today,
            authority,
            current: None,
            owner: true,
            message: String::new(),
            posting: Default::default(),
            sales_filter: views::Filter::default(),
            customers: views::CustomersState::default(),
            cashbook: ManualCashBook::default(),
            cash_form: None,
            dash_period: alm_core::confirmation::DashboardPeriod::Month,
            stock: views::StockState::default(),
            godown_query: String::new(),
            gst_db: gst,
            gst_form: None,
            gst_view: TRIAL_BILL,
            products: views::ProductsState::default(),
            order: views::OrderState::default(),
            receiving: None,
            crate_form: None,
            rates: Default::default(),
            sale_edit: None,
            outputs: device_root.join("outputs"),
            prefs: alm_store::preferences::Preferences::load(&prefs_dir),
            shop,
            device_root: device_root.to_path_buf(),
            restored: None,
        })
    }

    fn saved_ids(&self) -> Result<Vec<u64>, String> {
        let mut ids = Vec::new();
        let mut id = 1;
        while self
            .drafts
            .reopen_draft(DraftId::new(id))
            .map_err(text)?
            .is_some()
        {
            ids.push(id);
            id += 1;
        }
        Ok(ids)
    }

    fn today(&self) -> Result<BillDate, String> {
        self.today
            .ok_or_else(|| "The device date has not been received.".to_owned())
    }

    const fn role(&self) -> Role {
        if self.owner {
            Role::Owner
        } else {
            Role::Manager
        }
    }

    fn require_owner(&self) -> Result<Role, String> {
        match self.role() {
            Role::Owner => Ok(Role::Owner),
            Role::Manager => Err("Access denied: Owner only.".into()),
        }
    }

    fn render(&mut self, tab: &str) -> Result<String, String> {
        let tab = if tab == "draft" { "bill" } else { tab };
        let body = match tab {
            "customers" => views::customers_screen(
                &self.authority,
                &self.directory,
                self.role(),
                &self.customers,
            )
            .map_err(debug)?,
            "expenses" => views::cashbook_screen(&views::CashBookView {
                fixture: &self.authority,
                book: &self.cashbook,
                entry: self.cash_form.as_ref().map(|(draft, _)| draft),
                editing: self.cash_form.as_ref().and_then(|(_, id)| *id),
                role: self.role(),
                today: self.today()?,
            })
            .map_err(debug)?,
            "dashboard" => views::dashboard_screen(
                &self.authority,
                &self.directory,
                self.role(),
                self.today()?,
                self.dash_period,
            )?,
            "reorder" => views::reorder_screen(&self.authority, self.role()).map_err(debug)?,
            "stock" => {
                views::stock_screen(&self.authority, self.role(), &self.stock).map_err(debug)?
            }
            "godown" => views::godown_screen(&self.authority, self.role(), &self.godown_query)
                .map_err(debug)?,
            "gstsales" => {
                views::gst_sales_screen(&self.authority, self.role(), self.gst_form.as_ref())
                    .map_err(debug)?
            }
            "products" => views::products_screen(&self.authority, self.role(), &self.products)
                .map_err(debug)?,
            "suppliers" => views::suppliers_screen(&self.authority, self.role())?,
            "order" => {
                views::order_screen(&self.authority, self.role(), &self.order).map_err(debug)?
            }
            "purchases" => views::purchases_screen(
                &self.authority,
                self.role(),
                self.receiving,
                self.today()?,
            )?,
            "crates" => views::crates_screen(
                &self.authority,
                &self.directory,
                self.role(),
                self.crate_form.as_ref(),
            )
            .map_err(debug)?,
            "ratelist" => {
                let today = self.today()?;
                views::ratelist_screen(
                    &self.authority,
                    self.role(),
                    &mut self.rates,
                    SELLER.name,
                    today,
                )
                .map_err(debug)?
            }
            "reports" => {
                views::reports_screen(&self.authority, &self.cashbook, self.role(), self.today()?)?
            }
            "settings" => {
                let role = self.role();
                let profile = alm_core::synthetic_read_model::BusinessProfile::default();
                let (profile, reorder) = match role {
                    Role::Owner => (
                        self.shop.business_profile(role).map_err(debug)?,
                        self.shop.records(role).map_err(debug)?.reorder_default,
                    ),
                    Role::Manager => (&profile, 0),
                };
                let mut html = views::settings_screen(&views::SettingsView {
                    role,
                    theme: self.prefs.theme(),
                    balance: self.prefs.balance_mode(),
                    privacy: self
                        .prefs
                        .present_money(Money::from_minor_units(0))
                        .is_none(),
                    profile,
                    reorder_default: reorder,
                });
                if role == Role::Owner {
                    let restored = match &self.restored {
                        Some((name, store)) => Some((name.as_str(), backup::summary(store)?)),
                        None => None,
                    };
                    html += &views::backup_section(
                        &self.backups(),
                        restored.as_ref().map(|(n, s)| (*n, s)),
                    );
                }
                html
            }
            "counter" => {
                views::counter_screen(&self.authority, self.role(), self.today()?).map_err(debug)?
            }
            "receipt" => {
                views::receipt_screen(&self.authority, &self.directory, self.role(), self.today()?)
                    .map_err(debug)?
            }
            "sales" => {
                let mut html = String::new();
                if let (Some(id), Role::Owner) = (self.sale_edit, self.role()) {
                    html +=
                        &views::sale_edit_form(&self.authority, &self.directory, Role::Owner, id)
                            .map_err(debug)?;
                }
                html + &views::sales_screen(
                    &self.authority,
                    self.role(),
                    &self.sales_filter,
                    self.today()?,
                )
                .map_err(debug)?
            }
            "review" => {
                let draft = self.current.as_ref().map(|(draft, _)| draft);
                let role = self.role();
                let priced = match draft {
                    Some(draft) => owner_billing::priced_draft(&self.authority, role, draft),
                    None => Err(alm_core::confirmation::ConfirmationError::DraftNotFound),
                };
                let mut html = views::review_screen(&views::ReviewView {
                    role_is_owner: self.owner,
                    draft,
                    priced,
                });
                if self.owner {
                    html += &views::posting_review(
                        self.today()?,
                        self.posting.pending(role),
                        self.current
                            .as_ref()
                            .and_then(|(draft, saved)| saved.then_some(draft)),
                    );
                }
                html
            }
            "rateprint" => {
                let role = self.require_owner()?;
                let today = self.today()?;
                let html = self
                    .authority
                    .rate_list_html(
                        role,
                        &mut self.rates,
                        (SELLER.name, SELLER.address, ""),
                        today,
                    )
                    .map_err(debug)?
                    .ok_or("Tick at least one item first")?;
                views::gst_screen(&html, "Rate List").replace(
                    "Standalone GST tax invoice — not linked to stock or dues.",
                    "Rate list for the ticked items.",
                )
            }
            "gst" if !self.owner => views::denied("GST invoice"),
            "gst" => {
                let mut page = Err("GST bill missing".to_owned());
                self.authority
                    .emit_gst_bill(Role::Owner, self.gst_view, &mut |invoice| {
                        // printHTML's title: "GST-Invoice-" + fyStr(date) + "-" + no.
                        let number = invoice.invoice_number();
                        let title = format!(
                            "GST-Invoice-{}",
                            number.trim_start_matches("ALM/").replace('/', "-")
                        );
                        page = gst_print::invoice_html(invoice, &SELLER).map(|html| (html, title));
                    })
                    .map_err(debug)?;
                let (html, title) = page?;
                views::gst_screen(&html, &title)
            }
            "bill" => self.render_bill()?,
            _ => return Err("Unknown screen.".into()),
        };
        let message = std::mem::take(&mut self.message);
        Ok(format!(
            r#"{}{}<template id="screen-theme" data-theme="{}"></template>{body}"#,
            views::tabs(tab),
            views::notice(&message),
            self.prefs.theme().data_theme()
        ))
    }
}

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

// Commands run off the UI thread (`async`): a SQLite fsync on a slow device disk
// otherwise blocks Android's input thread and triggers an ANR.
#[tauri::command(async)]
fn screen(state: State<'_, AppState>, tab: String) -> Result<String, String> {
    with(&state, &tab, |_| Ok(()))
}

#[tauri::command(async)]
fn dash_period(state: State<'_, AppState>, period: String) -> Result<String, String> {
    use alm_core::confirmation::DashboardPeriod;
    let period = match period.as_str() {
        "today" => DashboardPeriod::Today,
        "week" => DashboardPeriod::Week,
        "month" => DashboardPeriod::Month,
        _ => return Err("Unknown period.".into()),
    };
    with(&state, "dashboard", |trial| {
        trial.require_owner()?;
        trial.dash_period = period;
        Ok(())
    })
}

#[tauri::command(async)]
fn rate_brand(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.toggle_brand_filter(&q);
        Ok(())
    })
}

#[tauri::command(async)]
fn rate_group(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        let role = trial.require_owner()?;
        trial
            .authority
            .rate_list_group_toggle(role, &mut trial.rates, &q)
            .map_err(debug)
    })
}

#[derive(serde::Deserialize)]
struct RateSearch {
    q: String,
}

#[tauri::command(async)]
fn rate_search(state: State<'_, AppState>, terms: RateSearch) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.query = terms.q;
        Ok(())
    })
}

#[tauri::command(async)]
fn rate_toggle(state: State<'_, AppState>, id: ipc::IpcId) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.toggle(alm_core::ProductId::new(id.value()));
        Ok(())
    })
}

/// The page sends the device's local calendar date once at start.
#[tauri::command(async)]
fn start(state: State<'_, AppState>, today: String) -> Result<String, String> {
    let today = today
        .parse()
        .map_err(|_| "The device date is not a valid date.".to_owned())?;
    with(&state, "bill", |trial| {
        trial.today = Some(today);
        Ok(())
    })
}

#[tauri::command(async)]
fn review_role(state: State<'_, AppState>, owner: bool) -> Result<String, String> {
    with(&state, "review", |trial| {
        if trial.owner != owner {
            trial.posting.invalidate();
        }
        trial.owner = owner;
        Ok(())
    })
}

/// Opens the platform print dialog for the current page; its print CSS keeps only the invoice.
#[tauri::command]
fn print_page(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(desktop)]
    return window.print().map_err(text);
    #[cfg(mobile)]
    {
        let _ = window;
        Err("Printing on mobile goes through the platform print bridge.".into())
    }
}

/// Plain-text summary for the platform share sheet.
#[tauri::command(async)]
fn share_text(state: State<'_, AppState>) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let role = trial.require_owner()?;
    let mut summary = Err("GST bill missing".to_owned());
    trial
        .authority
        .emit_gst_bill(role, trial.gst_view, &mut |invoice| {
            summary = Ok(format!(
                "GST Invoice {} dated {} for {}: ₹{}",
                invoice.invoice_number(),
                invoice.dated(),
                invoice.record.customer,
                gst_print::inr(invoice.totals.total)
            ));
        })
        .map_err(debug)?;
    summary
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
            start,
            screen,
            bill_entry::bill_fresh,
            bill_entry::bill_header,
            bill_entry::bill_add,
            bill_entry::bill_quantity,
            bill_entry::bill_remove,
            bill_entry::bill_open,
            bill_entry::bill_live,
            bill_entry::bill_customer_search,
            bill_entry::bill_item_search,
            review_role,
            billing::sale_prepare,
            billing::sale_confirm,
            billing::sales_filter,
            billing::sale_cancel,
            billing::sale_restore,
            billing::sale_edit_open,
            billing::sale_edit_close,
            billing::sale_edit_save,
            outputs::bill_image_share,
            outputs::open_url,
            outputs::export_file,
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
            dash_period,
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
            rate_brand,
            rate_group,
            rate_search,
            rate_toggle,
            print_page,
            share_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ALM trial");
}
