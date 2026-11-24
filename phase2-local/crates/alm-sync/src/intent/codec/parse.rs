//! Reads the canonical text of the intents `encode` writes.
use alm_core::{BillDate, Money};

use super::super::{
    CrateSale, CrateTerms, Intent, ProductTerms, PurchaseLineTerms, PurchaseOrderTerms,
};
use super::{NONE, unescape};

struct Fields<'a>(Vec<(&'a str, &'a str)>);

impl<'a> Fields<'a> {
    fn get(&self, key: &str) -> Option<&'a str> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }

    fn text(&self, key: &str) -> Option<String> {
        unescape(self.get(key)?)
    }

    fn num<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get(key)?.parse().ok()
    }

    fn money(&self, key: &str) -> Option<Money> {
        Some(Money::from_minor_units(self.num(key)?))
    }

    fn date(&self, key: &str) -> Option<BillDate> {
        self.num(key)
    }

    /// `Some(None)` for `~`.
    fn opt<T>(&self, key: &str, read: impl FnOnce(&'a str) -> Option<T>) -> Option<Option<T>> {
        match self.get(key)? {
            NONE => Some(None),
            text => read(text).map(Some),
        }
    }

    fn list<T>(&self, key: &str, item: impl Fn(&[&str]) -> Option<T>) -> Option<Vec<T>> {
        let text = self.get(key)?;
        if text.is_empty() {
            return Some(Vec::new());
        }
        text.split(',')
            .map(|part| item(&part.split(':').collect::<Vec<_>>()))
            .collect()
    }
}

fn component(p: &[&str]) -> Option<(u64, u32, bool)> {
    match p {
        [product, qty, unit] => Some((
            product.parse().ok()?,
            qty.parse().ok()?,
            match *unit {
                "piece" => true,
                "case" => false,
                _ => return None,
            },
        )),
        _ => None,
    }
}

fn parse_money(text: &str) -> Option<Money> {
    text.parse().ok().map(Money::from_minor_units)
}

/// Reads the intents [`encode`] writes; the caller checks the round trip.
pub(in crate::intent) fn parse(kind: &str, fields: Vec<(&str, &str)>) -> Option<Intent> {
    let f = Fields(fields);
    Some(match kind {
        "product" => Intent::SaveProduct(ProductTerms {
            id: f.num("id")?,
            new: match f.get("new")? {
                "1" => true,
                "0" => false,
                _ => return None,
            },
            item: f.text("item")?,
            brand: f.text("brand")?,
            buy: f.money("buy")?,
            sell: f.money("sell")?,
            reorder: f.opt("reorder", |t| t.parse().ok())?,
            reviewed: f.opt("was", |t| {
                let (b, s) = t.split_once(':')?;
                Some((parse_money(b)?, parse_money(s)?))
            })?,
        }),
        "productdel" => Intent::DeleteProduct { id: f.num("id")? },
        "supplier" => Intent::AddSupplier {
            name: f.text("name")?,
        },
        "quotes" => Intent::SetSupplierPrices {
            product: f.num("product")?,
            rates: f.list("rates", |p| match p {
                [s, NONE] => Some((s.parse().ok()?, None)),
                [s, r] => Some((s.parse().ok()?, Some(parse_money(r)?))),
                _ => None,
            })?,
        },
        "po" => {
            let name = f.opt("supplier", unescape)?;
            let id = f.opt("supplierid", |t| t.parse().ok())?;
            Intent::SavePurchaseOrder(PurchaseOrderTerms {
                supplier: match (name, id) {
                    (Some(name), id) => Some((id, name)),
                    (None, None) => None,
                    (None, Some(_)) => return None,
                },
                date: f.opt("date", |t| t.parse().ok())?,
                note: f.text("note")?,
                lines: f.list("lines", |p| match p {
                    [item, product, ordered, rate] => Some(PurchaseLineTerms {
                        item: unescape(item)?,
                        product: match *product {
                            NONE => None,
                            id => Some(id.parse().ok()?),
                        },
                        ordered: ordered.parse().ok()?,
                        rate: parse_money(rate)?,
                    }),
                    _ => None,
                })?,
            })
        }
        "receive" => Intent::ReceivePurchaseOrder {
            po: f.num("po")?,
            date: f.opt("date", |t| t.parse().ok())?,
            today: f.date("today")?,
            lines: f.list("lines", |p| match p {
                [q, r] => Some((q.parse().ok()?, parse_money(r)?)),
                _ => None,
            })?,
            remaining: f.list("remaining", |p| p.first()?.parse().ok())?,
        },
        "poclose" => Intent::ClosePurchaseOrder {
            po: f.num("po")?,
            remaining: f.list("remaining", |p| p.first()?.parse().ok())?,
        },
        "crate" => Intent::SaveCrate(CrateTerms {
            id: f.opt("id", |t| t.parse().ok())?,
            name: f.text("name")?,
            price: f.money("price")?,
            components: f.list("items", component)?,
        }),
        "cratedel" => Intent::DeleteCrate { id: f.num("id")? },
        "cratesale" => Intent::SellCrate(CrateSale {
            draft: f.num("draft")?,
            template: f.num("template")?,
            quantity: f.num("qty")?,
            customer: (f.text("customer")?, f.text("name")?),
            date: f.date("date")?,
            paid: f.money("paid")?,
            rate: f.money("rate")?,
            components: f.list("items", component)?,
        }),
        "opening" => Intent::SaveOpening {
            customer: f.text("customer")?,
            amount: f.opt("amount", parse_money)?,
            date: f.opt("date", |t| t.parse().ok())?,
        },
        "godown" => Intent::SaveGodown {
            item: f.text("item")?,
            cases: f.text("cases")?,
            note: f.text("note")?,
        },
        "count" => Intent::CountStock {
            product: f.num("product")?,
            counted: f.num("counted")?,
            fridge: f.opt("fridge", |t| t.parse().ok())?,
            date: f.date("date")?,
            reason: f.opt("reason", unescape)?,
            note: f.opt("note", unescape)?,
            on_hand: f.num("onhand")?,
        },
        "gst" => Intent::SaveGstBill {
            bill: f.text("bill")?,
        },
        "gstcancel" => Intent::CancelGstBill { id: f.num("id")? },
        "gstdel" => Intent::DeleteGstBill { id: f.num("id")? },
        "gstimport" => Intent::ImportGstBill {
            bill: f.opt("bill", unescape)?,
            deleted: f.opt("deleted", |t| t.parse().ok())?,
        },
        _ => return None,
    })
}
