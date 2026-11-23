//! One-screen Tauri trial: a Manager draft, an Owner review and a standalone GST invoice.
//! Every rule lives in `alm-core`/`alm-store`; the page only displays returned HTML.
#![forbid(unsafe_code)]

mod bill_entry;
mod billing;
#[cfg(test)]
mod billing_tests;
mod gst_print;
mod ipc;
mod receipts;
mod views;

#[cfg(test)]
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Mutex;

use alm_core::confirmation::demo_fixture;
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
}

type AppState = Mutex<Trial>;

impl Trial {
    fn open(device_root: &Path, today: Option<BillDate>) -> Result<Self, String> {
        let drafts = LocalSales::open_manager_account(device_root, ACCOUNT).map_err(text)?;
        let mut gst = GstBillDatabase::open_scoped(device_root, ACCOUNT).map_err(text)?;
        let mut authority = demo_fixture::fixture().map_err(debug)?;
        authority
            .load_gst_bills_from(Role::Owner, &gst)
            .map_err(debug)?;
        if authority
            .next_gst_bill_number(Role::Owner)
            .map_err(debug)?
            .0
            == 1
        {
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
        Ok(Self {
            drafts,
            directory: demo_fixture::directory(),
            today,
            authority,
            current: None,
            owner: true,
            message: String::new(),
            posting: Default::default(),
            sales_filter: views::Filter::default(),
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
            "counter" => {
                views::counter_screen(&self.authority, self.role(), self.today()?).map_err(debug)?
            }
            "receipt" => {
                views::receipt_screen(&self.authority, &self.directory, self.role(), self.today()?)
                    .map_err(debug)?
            }
            "sales" => views::sales_screen(
                &self.authority,
                self.role(),
                &self.sales_filter,
                self.today()?,
            )
            .map_err(debug)?,
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
            "gst" if !self.owner => views::denied("GST invoice"),
            "gst" => {
                let mut page = Err("GST bill missing".to_owned());
                self.authority
                    .emit_gst_bill(Role::Owner, TRIAL_BILL, &mut |invoice| {
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
            "{}{}{body}",
            views::tabs(tab),
            views::notice(&message)
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
        .emit_gst_bill(role, TRIAL_BILL, &mut |invoice| {
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
            app.manage(Mutex::new(Trial::open(&root, None)?));
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
            review_role,
            billing::sale_prepare,
            billing::sale_confirm,
            billing::sales_filter,
            billing::sale_cancel,
            billing::sale_restore,
            receipts::receipt_save,
            print_page,
            share_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ALM trial");
}
