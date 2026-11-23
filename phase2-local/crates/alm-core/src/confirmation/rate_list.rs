//! Rate List (legacy VIEWS.ratelist, L17): tick brands and items, then send today's sell rates.
//! Ported from the L17 replay model: new products start ticked, filters affect only what is
//! shown, group toggles act on shown rows, output groups by brand ("Other" when blank).
use std::collections::{BTreeMap, BTreeSet};

use super::estimate::display_date;
use super::image_handoff::{append_uri_component, format_amount};
use super::{ConfirmationFixture, ProductEntryError};
use crate::{BillDate, Money, ProductId, Role};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateListRow {
    pub product: ProductId,
    pub brand: String,
    pub item: String,
    pub rate: Money,
    pub selected: bool,
}

/// Per-device picker state (`rateSel` / `rateKnown`); never stored with shop data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RateListSelection {
    selected: BTreeSet<u64>,
    known: BTreeSet<u64>,
    pub brands: BTreeSet<String>,
    pub query: String,
}

fn brand(name: &str) -> &str {
    if name.is_empty() { "Other" } else { name }
}

impl RateListSelection {
    /// `rateSync`: ticks products seen for the first time.
    fn sync(&mut self, rows: &[(ProductId, String, String, Money)]) {
        for (id, ..) in rows {
            if self.known.insert(id.value()) {
                self.selected.insert(id.value());
            }
        }
    }

    pub fn toggle(&mut self, product: ProductId) {
        if !self.selected.remove(&product.value()) {
            self.selected.insert(product.value());
        }
    }

    pub fn toggle_brand_filter(&mut self, name: &str) {
        if !self.brands.remove(name) {
            self.brands.insert(name.to_owned());
        }
    }
}

impl ConfirmationFixture {
    fn rate_rows(
        &self,
        role: Role,
    ) -> Result<Vec<(ProductId, String, String, Money)>, ProductEntryError> {
        Ok(self
            .product_list(role, "", None)?
            .rows
            .into_iter()
            .map(|row| {
                (
                    row.product,
                    brand(&row.brand).to_owned(),
                    row.item,
                    row.sell,
                )
            })
            .collect())
    }

    /// Shown rows sorted by brand then item, after ticking new products.
    pub fn rate_list_rows(
        &self,
        role: Role,
        selection: &mut RateListSelection,
    ) -> Result<Vec<RateListRow>, ProductEntryError> {
        let rows = self.rate_rows(role)?;
        selection.sync(&rows);
        let query = selection.query.to_lowercase();
        let mut shown: Vec<_> = rows
            .into_iter()
            .filter(|(_, brand, item, _)| {
                (selection.brands.is_empty() || selection.brands.contains(brand))
                    && (item.to_lowercase().contains(&query)
                        || brand.to_lowercase().contains(&query))
            })
            .map(|(product, brand, item, rate)| RateListRow {
                selected: selection.selected.contains(&product.value()),
                product,
                brand,
                item,
                rate,
            })
            .collect();
        shown.sort_by(|a, b| (&a.brand, &a.item).cmp(&(&b.brand, &b.item)));
        Ok(shown)
    }

    /// Ticks or clears every shown row of one brand: all ticked → clear, else tick all.
    pub fn rate_list_group_toggle(
        &self,
        role: Role,
        selection: &mut RateListSelection,
        group: &str,
    ) -> Result<(), ProductEntryError> {
        let ids: Vec<_> = self
            .rate_list_rows(role, selection)?
            .into_iter()
            .filter(|row| row.brand == group)
            .map(|row| (row.product.value(), row.selected))
            .collect();
        let all = ids.iter().all(|(_, on)| *on);
        for (id, _) in ids {
            if all {
                selection.selected.remove(&id);
            } else {
                selection.selected.insert(id);
            }
        }
        Ok(())
    }

    /// Ticked products by brand ('Other' when blank) and the header subtitle rule (≤ 3 brands
    /// and fewer than all), shared by the text and print outputs.
    #[allow(clippy::type_complexity)]
    fn rate_groups(
        &self,
        role: Role,
        selection: &mut RateListSelection,
    ) -> Result<(BTreeMap<String, Vec<(String, Money)>>, String), ProductEntryError> {
        let rows = self.rate_rows(role)?;
        selection.sync(&rows);
        let all: BTreeSet<_> = rows.iter().map(|(_, b, ..)| b.clone()).collect();
        let mut groups: BTreeMap<String, Vec<(String, Money)>> = BTreeMap::new();
        for (id, brand, item, rate) in rows {
            if selection.selected.contains(&id.value()) {
                groups.entry(brand).or_default().push((item, rate));
            }
        }
        let sub = if !groups.is_empty() && groups.len() <= 3 && groups.len() < all.len() {
            groups.keys().cloned().collect::<Vec<_>>().join(", ")
        } else {
            String::new()
        };
        Ok((groups, sub))
    }

    /// `rateListHTML`: the printable rate list for the ticked products, or `None` if none.
    /// The catalog has no pack size or MRP, so those cells are blank.
    pub fn rate_list_html(
        &self,
        role: Role,
        selection: &mut RateListSelection,
        header: (&str, &str, &str),
        today: BillDate,
    ) -> Result<Option<String>, ProductEntryError> {
        use super::estimate::escape_text as esc;
        let (groups, sub) = self.rate_groups(role, selection)?;
        if groups.is_empty() {
            return Ok(None);
        }
        let (name, tagline, mobile) = header;
        let sub = if sub.is_empty() {
            String::new()
        } else {
            format!("— {}", esc(&sub))
        };
        let mut html = format!(
            "<div style=\"background:#fff;color:#1a1a1a;padding:22px;max-width:760px;margin:0 auto\">\n    <div style=\"text-align:center;border-bottom:3px solid #2563eb;padding-bottom:10px;margin-bottom:14px\">\n      <div style=\"font-size:22px;font-weight:800;color:#1d4ed8\">{}</div>\n      <div style=\"font-size:12px;color:#555\">{} · Mob: {}</div>\n      <div style=\"font-size:13px;font-weight:700;margin-top:6px\">RATE LIST {sub} <span style=\"font-weight:400;color:#888\">({})</span></div>\n    </div>",
            esc(name),
            esc(tagline),
            esc(mobile),
            display_date(today)
        );
        for (group, items) in groups {
            html.push_str(&format!("<div style=\"font-weight:700;color:#1d4ed8;margin:10px 0 4px;font-size:14px\">{}</div>\n    <table style=\"width:100%;border-collapse:collapse;font-size:12.5px;margin-bottom:8px\">\n    <thead><tr style=\"background:#eef2fb\"><th style=\"text-align:left;padding:5px 8px;border:1px solid #dbe3f4\">Item</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Pack</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">MRP</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Our Rate</th></tr></thead><tbody>", esc(&group)));
            for (item, rate) in items {
                html.push_str(&format!("<tr><td style=\"padding:4px 8px;border:1px solid #eee\">{}</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:center\"></td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right\"></td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right;font-weight:700\">₹{}</td></tr>", esc(&item), format_amount(rate)));
            }
            html.push_str("</tbody></table>");
        }
        html.push_str("<div style=\"font-size:11px;color:#888;margin-top:8px\">Rates are per case/packet and subject to change. * E.&amp;O.E.</div></div>");
        Ok(Some(html))
    }

    /// The WhatsApp text and its `wa.me` link for the ticked products, or `None` if none.
    pub fn rate_list_text(
        &self,
        role: Role,
        selection: &mut RateListSelection,
        business: &str,
        today: BillDate,
    ) -> Result<Option<(String, String)>, ProductEntryError> {
        let (groups, sub) = self.rate_groups(role, selection)?;
        if groups.is_empty() {
            return Ok(None);
        }
        let sub = if sub.is_empty() {
            String::new()
        } else {
            format!(" ({sub})")
        };
        let mut text = format!("*{business} — Rate List{sub}*\n{}\n\n", display_date(today));
        for (group, items) in groups {
            text.push_str(&format!("*{group}*\n"));
            for (item, rate) in items {
                text.push_str(&format!("{item} — ₹{}\n", format_amount(rate)));
            }
            text.push('\n');
        }
        let mut uri = String::from("https://wa.me/?text=");
        append_uri_component(&mut uri, &text);
        Ok(Some((text, uri)))
    }
}

#[cfg(test)]
mod tests {
    use super::RateListSelection;
    use crate::confirmation::ProductEntryError;
    use crate::confirmation::demo_fixture::fixture;
    use crate::{BillDate, ProductId, Role};

    #[test]
    fn selection_filters_groups_and_text_follow_the_l17_model() {
        let mut fixture = fixture().unwrap();
        let mut form = fixture
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap();
        form.brand = "Fizz Co".into();
        fixture.save_product_entry(Role::Owner, &form).unwrap();
        let mut selection = RateListSelection::default();
        let rows = fixture.rate_list_rows(Role::Owner, &mut selection).unwrap();
        let shown: Vec<_> = rows
            .iter()
            .map(|r| (r.brand.as_str(), r.item.as_str(), r.selected))
            .collect();
        assert_eq!(
            shown,
            [
                ("Fizz Co", "Synthetic Soda", true),
                ("Other", "Synthetic Lime", true)
            ]
        );
        let day = BillDate::new(2025, 2, 21).unwrap();
        let (text, uri) = fixture
            .rate_list_text(Role::Owner, &mut selection, "Synthetic & Co", day)
            .unwrap()
            .unwrap();
        assert_eq!(
            text,
            "*Synthetic & Co — Rate List*\n21-Feb-2025\n\n*Fizz Co*\nSynthetic Soda — ₹62.5\n\n*Other*\nSynthetic Lime — ₹120.5\n\n"
        );
        assert!(uri.starts_with("https://wa.me/?text=*Synthetic%20%26%20Co"));
        let html = fixture
            .rate_list_html(
                Role::Owner,
                &mut selection,
                ("Synthetic & Co", "Test <only>", ""),
                day,
            )
            .unwrap()
            .unwrap();
        assert!(html.contains(
            "<div style=\"font-size:22px;font-weight:800;color:#1d4ed8\">Synthetic &amp; Co</div>"
        ));
        assert!(
            html.contains(
                "<div style=\"font-size:12px;color:#555\">Test &lt;only&gt; · Mob: </div>"
            )
        );
        assert!(html.contains(
            "RATE LIST  <span style=\"font-weight:400;color:#888\">(21-Feb-2025)</span>"
        ));
        assert!(html.contains("font-weight:700\">₹62.5</td>"));
        selection.query = "soda".into();
        fixture
            .rate_list_group_toggle(Role::Owner, &mut selection, "Fizz Co")
            .unwrap();
        selection.query.clear();
        let (text, _) = fixture
            .rate_list_text(Role::Owner, &mut selection, "Synthetic & Co", day)
            .unwrap()
            .unwrap();
        assert!(text.starts_with("*Synthetic & Co — Rate List (Other)*"));
        selection.toggle(ProductId::new(1));
        assert_eq!(
            fixture.rate_list_text(Role::Owner, &mut selection, "x", day),
            Ok(None)
        );
        assert_eq!(
            fixture.rate_list_rows(Role::Manager, &mut selection),
            Err(ProductEntryError::AccessDenied)
        );
    }
}
