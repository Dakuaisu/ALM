//! Canonical text for the catalog, purchasing, stock, opening, godown and GST intents.
//! Values are escaped; lists join items with `,` and item parts with `:`; `~` is "none".
use alm_core::Money;

use super::Intent;

mod parse;
pub(super) use parse::parse;

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

pub(super) const NONE: &str = "~";

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
        Intent::SetReorderDefault { level } => format!("reorderdefault level={level}"),
        Intent::SetLoadingDefault { rate } => format!("loadingdefault rate={}", money(*rate)),
        Intent::RecordRestore => "restorelog".into(),
        Intent::ImportLegacy(plan) => format!("legacyimport plan={}", escape(&plan.to_json())),
        Intent::ResolveLegacyQuarantine(d) => format!(
            "legacyresolve decision={}",
            escape(&serde_json::to_string(d.as_ref()).unwrap_or_default())
        ),
        Intent::SellCrate(c) => format!(
            "cratesale draft={} template={} qty={} customer={} name={} date={} paid={} rate={} items={}",
            c.draft,
            c.template,
            c.quantity,
            escape(&c.customer.0),
            escape(&c.customer.1),
            c.date,
            money(c.paid),
            money(c.rate),
            list(&c.components, |(p, q, piece)| vec![
                p.to_string(),
                q.to_string(),
                if *piece { "piece" } else { "case" }.into(),
            ])
        ),
        Intent::SaveCustomer(c) => format!(
            "customer id={} name={} mobile={} addr1={} addr2={}",
            escape(c.id.as_deref().unwrap_or("")),
            escape(&c.name),
            escape(&c.mobile),
            escape(&c.addr1),
            escape(&c.addr2)
        ),
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
