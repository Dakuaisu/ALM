//! Control totals over a legacy `DB`, with the HTML's own arithmetic: `nextBillNo`,
//! `nextGstNo`, `custBalanceMap`, `qtyMaps`/`stockOf`, `lotsFor`/`lotsRemaining` and
//! `lotsRemainValue`, the Cash Book day sums. The same function reads the legacy file and the
//! records the authority holds after import (rebuilt into the same shape), so any loss in the
//! mapping shows as a difference. Amounts are shown in rupees and paise, quantities in packets.
use std::collections::BTreeMap;

use super::LegacyBackup;
use super::records::LegacyProduct;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlLine {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlSection {
    pub name: &'static str,
    pub lines: Vec<ControlLine>,
}

/// `₹1,234.50` from rupees, rounded to the paisa.
pub(super) fn rupees(x: f64) -> String {
    let paise = (x * 100.0).round() as i64;
    let sign = if paise < 0 { "-" } else { "" };
    let abs = paise.unsigned_abs();
    format!(
        "{sign}₹{}.{:02}",
        crate::projection::book::indian_number(i64::try_from(abs / 100).unwrap_or(i64::MAX)),
        abs % 100
    )
}

fn quantity(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        let text = format!("{x:.3}");
        text.trim_end_matches('0').to_owned()
    }
}

fn line(lines: &mut Vec<ControlLine>, key: impl Into<String>, value: impl Into<String>) {
    lines.push(ControlLine {
        key: key.into(),
        value: value.into(),
    });
}

fn avg_cost(p: Option<&LegacyProduct>) -> f64 {
    p.map_or(0.0, |p| {
        if p.avg_cost > 0.0 {
            p.avg_cost
        } else {
            p.buy_rate
        }
    })
}

#[allow(clippy::too_many_lines)]
pub fn controls(b: &LegacyBackup) -> Vec<ControlSection> {
    let products = b.products();
    let sales = b.sales();
    let mut sections = Vec::new();

    // Invoices: number, status, totals, and `nextBillNo()` (shown, not issued).
    let mut lines = Vec::new();
    let mut ordered: Vec<_> = sales.iter().collect();
    ordered.sort_by(|a, b| a.no.total_cmp(&b.no).then(a.id.cmp(&b.id)));
    let (mut active, mut cancelled, mut total, mut max) = (0, 0, 0.0, 0.0_f64);
    for s in &ordered {
        line(
            &mut lines,
            format!("#{} ({})", quantity(s.no), s.id),
            format!(
                "{} total {} paid {}",
                if s.cancelled { "cancelled" } else { "active" },
                rupees(s.total),
                rupees(s.paid)
            ),
        );
        if s.cancelled {
            cancelled += 1;
        } else {
            active += 1;
            total += s.total;
        }
        max = max.max(s.no);
    }
    line(
        &mut lines,
        "bills",
        format!("{} ({active} active, {cancelled} cancelled)", sales.len()),
    );
    line(&mut lines, "active total", rupees(total));
    let next = if max > 0.0 { max } else { 1000.0 } + 1.0;
    line(
        &mut lines,
        "next invoice (not issued)",
        format!("#{}", quantity(next)),
    );
    sections.push(ControlSection {
        name: "Invoices",
        lines,
    });

    // GST bills and `nextGstNo()`.
    let mut lines = Vec::new();
    let bills = b.collection("gstBills");
    let mut rows: Vec<(f64, String, bool, f64)> = bills
        .iter()
        .map(|g| {
            let amount = g
                .get("items")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .map(|li| super::js::field(li, "amount"))
                .sum();
            (
                super::js::field(g, "no"),
                super::js::text(g, "id"),
                g.get("cancelled").is_some_and(super::js::truthy),
                amount,
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut active_total = 0.0;
    for (no, id, cancelled, amount) in &rows {
        line(
            &mut lines,
            format!("GST {} ({id})", quantity(*no)),
            format!(
                "{} {}",
                if *cancelled { "cancelled" } else { "active" },
                rupees(*amount)
            ),
        );
        if !cancelled {
            active_total += amount;
        }
    }
    line(&mut lines, "active total", rupees(active_total));
    let next = rows.iter().map(|r| r.0).fold(0.0, f64::max) + 1.0;
    line(&mut lines, "next GST number (not issued)", quantity(next));
    sections.push(ControlSection {
        name: "GST bills",
        lines,
    });

    // Payments (receipts).
    let mut lines = Vec::new();
    let payments = b.payments();
    let mut total = 0.0;
    let mut by_id: Vec<_> = payments.iter().collect();
    by_id.sort_by(|a, b| a.id.cmp(&b.id));
    for p in by_id {
        line(
            &mut lines,
            p.id.clone(),
            format!(
                "{} {} {} {}",
                p.date,
                p.customer_name,
                rupees(p.amount),
                p.mode
            ),
        );
        total += p.amount;
    }
    line(&mut lines, "total", rupees(total));
    sections.push(ControlSection {
        name: "Payments",
        lines,
    });

    // Customer balances: `custBalanceMap()` (whole rupees).
    let mut lines = Vec::new();
    let customers = b.customers();
    let mut balance: BTreeMap<&str, f64> = customers
        .iter()
        .map(|c| (c.id.as_str(), c.opening))
        .collect();
    for s in sales.iter().filter(|s| !s.cancelled) {
        if let Some(v) = balance.get_mut(s.customer_id.as_str()) {
            *v += s.total - s.paid;
        }
    }
    for p in &payments {
        if let Some(v) = balance.get_mut(p.customer_id.as_str()) {
            *v -= p.amount;
        }
    }
    let mut receivable = 0.0;
    for c in &customers {
        let rounded = (balance[c.id.as_str()] + 0.5).floor();
        receivable += rounded;
        line(
            &mut lines,
            format!("{} {}", c.id, c.name),
            format!(
                "₹{}",
                crate::projection::book::indian_number(rounded as i64)
            ),
        );
    }
    line(
        &mut lines,
        "sum of balances",
        format!(
            "₹{}",
            crate::projection::book::indian_number(receivable as i64)
        ),
    );
    sections.push(ControlSection {
        name: "Customer balances",
        lines,
    });

    // Cash Book days.
    let mut lines = Vec::new();
    let mut days = b.cash_days();
    days.sort_by(|a, b| a.date.cmp(&b.date).then(a.id.cmp(&b.id)));
    let (mut inflow, mut outflow) = (0.0, 0.0);
    for d in &days {
        let i = d.cash + d.paytm + d.phonepe;
        let o = d.goods + d.misc + d.salary;
        inflow += i;
        outflow += o;
        line(
            &mut lines,
            format!("{} ({})", d.date, d.id),
            format!("in {} out {} debt {}", rupees(i), rupees(o), rupees(d.debt)),
        );
    }
    line(&mut lines, "total in", rupees(inflow));
    line(&mut lines, "total out", rupees(outflow));
    sections.push(ControlSection {
        name: "Cash Book",
        lines,
    });

    // Stock by item and location (shop on hand, fridge, godown).
    let index = b.product_index();
    let mut purchased: BTreeMap<String, f64> = BTreeMap::new();
    let mut sold: BTreeMap<String, f64> = BTreeMap::new();
    let mut adj: BTreeMap<String, f64> = BTreeMap::new();
    for purchase in b.collection("purchases") {
        for li in purchase
            .get("items")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            for (key, _, packets) in index.line_packets(&super::records::line_of(li)) {
                *purchased.entry(key).or_default() += packets;
            }
        }
    }
    for order in b.collection("purchaseOrders") {
        for li in order
            .get("items")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let pid = li
                .get("pid")
                .filter(|v| super::js::truthy(v))
                .map(super::js::string);
            let key = pid.unwrap_or_else(|| index.key_of_name(&super::js::text(li, "item")));
            *purchased.entry(key).or_default() += super::js::field(li, "received");
        }
    }
    for s in sales.iter().filter(|s| !s.cancelled) {
        for li in &s.lines {
            for (key, _, packets) in index.line_packets(li) {
                *sold.entry(key).or_default() += packets;
            }
        }
    }
    for e in b.stock_log().iter().filter(|e| e.eff) {
        *adj.entry(e.key.clone()).or_default() += e.delta;
    }
    let stock = b.stock_entries();
    let mut keys: Vec<String> = products.iter().map(|p| p.id.clone()).collect();
    for key in stock
        .iter()
        .map(|s| s.key.clone())
        .chain(purchased.keys().cloned())
        .chain(sold.keys().cloned())
        .chain(adj.keys().cloned())
    {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let name_of = |key: &str| {
        key.strip_prefix("n:").map_or_else(
            || {
                products
                    .iter()
                    .find(|p| p.id == key)
                    .map(|p| p.item.clone())
                    .unwrap_or_default()
            },
            str::to_owned,
        )
    };
    let mut lines = Vec::new();
    for key in &keys {
        let entry = stock.iter().find(|s| &s.key == key);
        let on_hand = entry.map_or(0.0, |s| s.opening)
            + purchased.get(key).copied().unwrap_or_default()
            - sold.get(key).copied().unwrap_or_default()
            + adj.get(key).copied().unwrap_or_default();
        line(
            &mut lines,
            format!("{key} {}", name_of(key)),
            format!(
                "shop {} · fridge {}",
                quantity(on_hand),
                quantity(entry.map_or(0.0, |s| s.fridge))
            ),
        );
    }
    let mut godown = b.godown();
    godown.sort_by(|a, b| a.id.cmp(&b.id));
    for g in &godown {
        line(
            &mut lines,
            format!("godown {} ({})", g.item, g.id),
            format!("{} cases", quantity(g.qty)),
        );
    }
    sections.push(ControlSection {
        name: "Stock by item and location",
        lines,
    });

    // FIFO lots and value: `lotsRemaining` / `lotsRemainValue`.
    let mut lots_by_key: BTreeMap<String, Vec<(String, String, f64, f64)>> = BTreeMap::new();
    for lot in b.purchase_lots() {
        let product = product_for_key(&products, &lot.key);
        let rate = lot.rate.unwrap_or_else(|| avg_cost(product));
        lots_by_key.entry(lot.key.clone()).or_default().push((
            lot.date,
            lot.legacy_key,
            lot.qty,
            rate,
        ));
    }
    let mut lines = Vec::new();
    let mut total_value = 0.0;
    for key in &keys {
        let product = product_for_key(&products, key);
        let mut lots = Vec::new();
        let opening = stock
            .iter()
            .find(|s| &s.key == key)
            .map_or(0.0, |s| s.opening);
        if opening > 0.0 {
            let rate = product
                .filter(|p| p.opening_cost > 0.0)
                .map_or_else(|| avg_cost(product), |p| p.opening_cost);
            lots.push(("0000-01-01".to_owned(), "opening".to_owned(), opening, rate));
        }
        let mut bought = lots_by_key.remove(key).unwrap_or_default();
        bought.sort_by(|a, b| a.0.cmp(&b.0));
        lots.extend(bought);
        let mut consumed = (sold.get(key).copied().unwrap_or_default()
            - adj.get(key).copied().unwrap_or_default())
        .max(0.0);
        let mut value = 0.0;
        for (date, source, qty, rate) in &lots {
            let mut left = *qty;
            if consumed > 0.0 {
                let c = consumed.min(left);
                left -= c;
                consumed -= c;
            }
            if left > 1e-9 {
                value += left * rate;
                line(
                    &mut lines,
                    format!("{key} lot {date} {source}"),
                    format!(
                        "left {} of {} @ {}",
                        quantity(left),
                        quantity(*qty),
                        rupees(*rate)
                    ),
                );
            }
        }
        if !lots.is_empty() {
            line(&mut lines, format!("{key} value"), rupees(value));
        }
        total_value += value;
    }
    line(&mut lines, "total value", rupees(total_value));
    sections.push(ControlSection {
        name: "FIFO lots and value",
        lines,
    });
    sections
}

fn product_for_key<'a>(products: &'a [LegacyProduct], key: &str) -> Option<&'a LegacyProduct> {
    match key.strip_prefix("n:") {
        Some(item) => products.iter().find(|p| p.item == item),
        None => products.iter().find(|p| p.id == key),
    }
}
