//! Screen dispatch: renders a tab's HTML from the current state.
use alm_core::confirmation::owner_billing;
use alm_core::{DraftId, Money, Role};

use crate::{SELLER, Trial, backup, debug, gst_print, views};

impl Trial {
    pub(crate) fn render(&mut self, tab: &str) -> Result<String, String> {
        self.refresh()?;
        let tab = if tab == "draft" { "bill" } else { tab };
        let body = match tab {
            _ if self.revoked => format!(
                r#"<div class="card" role="alert"><b>{}</b><div class="help">Nothing of this account is kept on the device. Choose another account:</div></div>{}"#,
                crate::trial::REVOKED,
                self.account_pills()
            ),
            "customers" => {
                let stored = self.projected()?;
                self.freshness(&stored)?
                    + &views::customers_screen(
                        &stored.snapshot,
                        &self.posted,
                        &self.directory,
                        self.role(),
                        &self.customers,
                    )
                    .map_err(debug)?
            }
            "expenses" => views::cashbook_screen(&views::CashBookView {
                fixture: &self.posted,
                book: self.posted.cashbook(),
                entry: self.cash_form.as_ref().map(|(draft, _)| draft),
                editing: self.cash_form.as_ref().and_then(|(_, id)| *id),
                role: self.role(),
                today: self.today()?,
            })
            .map_err(debug)?,
            "dashboard" => views::dashboard_screen(
                &self.posted,
                &self.directory,
                self.role(),
                self.today()?,
                self.dash_period,
            )?,
            "reorder" => views::reorder_screen(&self.posted, self.role()).map_err(debug)?,
            "stock" => {
                let stored = self.projected()?;
                self.freshness(&stored)?
                    + &views::stock_screen(&stored.snapshot, &self.posted, self.role(), &self.stock)
                        .map_err(debug)?
            }
            "godown" => views::godown_screen(&self.posted, self.role(), &self.godown_query)
                .map_err(debug)?,
            "gstsales" => {
                views::gst_sales_screen(&self.posted, self.role(), self.gst_form.as_ref())
                    .map_err(debug)?
            }
            "products" => {
                views::products_screen(&self.posted, self.role(), &self.products).map_err(debug)?
            }
            "suppliers" => views::suppliers_screen(&self.posted, self.role())?,
            "order" => {
                views::order_screen(&self.posted, self.role(), &self.order).map_err(debug)?
            }
            "purchases" => {
                views::purchases_screen(&self.posted, self.role(), self.receiving, self.today()?)?
            }
            "crates" => views::crates_screen(&self.posted, self.role(), self.crate_form.as_ref())
                .map_err(debug)?,
            "ratelist" => {
                let today = self.today()?;
                views::ratelist_screen(
                    &self.posted,
                    self.role(),
                    &mut self.rates,
                    SELLER.name,
                    today,
                )
                .map_err(debug)?
            }
            "reports" => views::reports_screen(
                &self.posted,
                self.posted.cashbook(),
                self.role(),
                self.today()?,
            )?,
            "settings" => {
                let role = self.role();
                let profile = alm_core::synthetic_read_model::BusinessProfile::default();
                let reorder = match role {
                    Role::Owner => self.reorder_default()?,
                    Role::Manager => 0,
                };
                let activity = match role {
                    Role::Owner => self
                        .projected()?
                        .snapshot
                        .owner
                        .map(|owner| owner.book.activity)
                        .unwrap_or_default(),
                    Role::Manager => Vec::new(),
                };
                let profile = match role {
                    Role::Owner => self.shop.business_profile(role).map_err(debug)?,
                    Role::Manager => &profile,
                };
                let backup = if role == Role::Owner {
                    let restored = match &self.restored {
                        Some((name, store)) => Some((name.as_str(), backup::summary(store)?)),
                        None => None,
                    };
                    views::backup_section(&self.backups(), restored.as_ref().map(|(n, s)| (*n, s)))
                        + &crate::dev_authority::card(&self.device_root)
                } else {
                    String::new()
                };
                views::settings_screen(&views::SettingsView {
                    role,
                    theme: self.prefs.theme(),
                    balance: self.prefs.balance_mode(),
                    privacy: self
                        .prefs
                        .present_money(Money::from_minor_units(0))
                        .is_none(),
                    profile,
                    reorder_default: reorder,
                    backup: &backup,
                    activity: &activity,
                })
            }
            "counter" => {
                views::counter_screen(&self.posted, self.role(), self.today()?).map_err(debug)?
            }
            "receipt" => {
                views::receipt_screen(&self.posted, &self.directory, self.role(), self.today()?)
                    .map_err(debug)?
            }
            "sales" => {
                let stored = self.projected()?;
                let mut html = self.freshness(&stored)?;
                if let (Some(id), Role::Owner) = (self.sale_edit, self.role()) {
                    html += &views::sale_edit_form(&self.posted, &self.directory, Role::Owner, id)
                        .map_err(debug)?;
                }
                html + &views::sales_screen(
                    &stored.snapshot,
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
                    Some(draft) => owner_billing::priced_draft(&self.posted, role, draft),
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
                let stored = self.projected()?;
                html += &self.freshness(&stored)?;
                html + &self.account_pills()
            }
            "rateprint" => {
                let role = self.require_owner()?;
                let today = self.today()?;
                let html = self
                    .posted
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
                self.posted
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
            "estimate" => {
                let id = self.print_view.ok_or("No bill selected.")?;
                let html = self.estimate_html(DraftId::new(id))?;
                views::gst_screen(&html, &format!("Estimate-{id}")).replace(
                    "Standalone GST tax invoice — not linked to stock or dues.",
                    "Estimate (not a tax invoice).",
                )
            }
            "poprint" => {
                let number = self.print_view.ok_or("No purchase order selected.")?;
                let html = self.po_print_html(number)?;
                views::gst_screen(&html, &format!("PO-{number:03}")).replace(
                    "Standalone GST tax invoice — not linked to stock or dues.",
                    "Purchase order for the supplier.",
                )
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
