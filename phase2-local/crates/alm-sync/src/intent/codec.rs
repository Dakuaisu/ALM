//! Canonical text for the catalog, purchasing, stock, opening, godown and GST intents.
//! Values are escaped; lists join items with `,` and item parts with `:`; `~` is "none".
use alm_core::{BillDate, Money};

use super::{CrateTerms, Intent, ProductTerms, PurchaseLineTerms, PurchaseOrderTerms};

pub(super) fn escape(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub(super) fn unescape(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

const NONE: &str = "~";

fn opt<T>(value: Option<T>, text: impl FnOnce(T) -> String) -> String {
    value.map_or_else(|| NONE.into(), text)
}

fn money(value: Money) -> String {
    value.minor_units().to_string()
}

fn list<T>(items: &[T], item: impl Fn(&T) -> Vec<String>) -> String {
    items
        .iter()
        .map(|i| item(i).join(":"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The canonical text of the intents this module owns; `None` for the others.
pub(super) fn encode(intent: &Intent) -> Option<String> {
    Some(match intent {
        Intent::SaveProduct(p) => format!(
            "product id={} new={} item={} brand={} buy={} sell={} reorder={} was={}",
            p.id,
            u8::from(p.new),
            escape(&p.item),
            escape(&p.brand),
            money(p.buy),
            money(p.sell),
            opt(p.reorder, |r| r.to_string()),
            opt(p.reviewed, |(b, s)| format!("{}:{}", money(b), money(s)))
        ),
        Intent::DeleteProduct { id } => format!("productdel id={id}"),
        Intent::AddSupplier { name } => format!("supplier name={}", escape(name)),
        Intent::SetSupplierPrices { product, rates } => format!(
            "quotes product={product} rates={}",
            list(rates, |(s, r)| vec![s.to_string(), opt(*r, money)])
        ),
        Intent::SavePurchaseOrder(po) => format!(
            "po supplier={} supplierid={} date={} note={} lines={}",
            opt(po.supplier.as_ref(), |(_, n)| escape(n)),
            opt(po.supplier.as_ref().and_then(|(id, _)| *id), |id| id
                .to_string()),
            opt(po.date, |d| d.to_string()),
            escape(&po.note),
            list(&po.lines, |l| vec![
                escape(&l.item),
                opt(l.product, |p| p.to_string()),
                l.ordered.to_string(),
                money(l.rate),
            ])
        ),
        Intent::ReceivePurchaseOrder {
            po,
            date,
            today,
            lines,
            remaining,
        } => format!(
            "receive po={po} date={} today={today} lines={} remaining={}",
            opt(*date, |d| d.to_string()),
            list(lines, |(q, r)| vec![q.to_string(), money(*r)]),
            list(remaining, |r| vec![r.to_string()])
        ),
        Intent::ClosePurchaseOrder { po, remaining } => format!(
            "poclose po={po} remaining={}",
            list(remaining, |r| vec![r.to_string()])
        ),
        Intent::SaveCrate(c) => format!(
            "crate id={} name={} price={} items={}",
            opt(c.id, |id| id.to_string()),
            escape(&c.name),
            money(c.price),
            list(&c.components, |(p, q, piece)| vec![
                p.to_string(),
                q.to_string(),
                if *piece { "piece" } else { "case" }.into(),
            ])
        ),
        Intent::DeleteCrate { id } => format!("cratedel id={id}"),
        Intent::SaveOpening {
            customer,
            amount,
            date,
        } => format!(
            "opening customer={} amount={} date={}",
            escape(customer),
            opt(*amount, money),
            opt(*date, |d| d.to_string())
        ),
        Intent::SaveGodown { item, cases, note } => format!(
            "godown item={} cases={} note={}",
            escape(item),
            escape(cases),
            escape(note)
        ),
        Intent::CountStock {
            product,
            counted,
            fridge,
            date,
            reason,
            note,
            on_hand,
        } => format!(
            "count product={product} counted={counted} fridge={} date={date} reason={} note={} onhand={on_hand}",
            opt(*fridge, |f| f.to_string()),
            opt(reason.as_deref(), escape),
            opt(note.as_deref(), escape)
        ),
        Intent::SaveGstBill { bill } => format!("gst bill={}", escape(bill)),
        Intent::CancelGstBill { id } => format!("gstcancel id={id}"),
        Intent::DeleteGstBill { id } => format!("gstdel id={id}"),
        Intent::ImportGstBill { bill, deleted } => format!(
            "gstimport bill={} deleted={}",
            opt(bill.as_deref(), escape),
            opt(*deleted, |id| id.to_string())
        ),
        _ => return None,
    })
}

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

fn parse_money(text: &str) -> Option<Money> {
    text.parse().ok().map(Money::from_minor_units)
}

/// Reads the intents [`encode`] writes; the caller checks the round trip.
pub(super) fn parse(kind: &str, fields: Vec<(&str, &str)>) -> Option<Intent> {
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
            components: f.list("items", |p| match p {
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
            })?,
        }),
        "cratedel" => Intent::DeleteCrate { id: f.num("id")? },
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
