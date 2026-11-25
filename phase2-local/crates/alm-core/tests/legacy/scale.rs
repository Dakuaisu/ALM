//! A synthetic legacy backup at shop scale, generated (never real data): 500 products, 1,000
//! customers, 20,000 bills (one in ten mixes a crate line with item lines; one in twenty is a
//! crate bill; some cancelled, some walk-in), 2,000 purchases, 300 GST bills, 2,000 payments,
//! 20 crate templates, a year of Cash Book days and stock openings. Deterministic.
#![allow(dead_code)]

use std::fmt::Write as _;

pub struct Scale {
    pub products: usize,
    pub customers: usize,
    pub bills: usize,
    pub purchases: usize,
    pub gst_bills: usize,
    pub payments: usize,
    pub crates: usize,
    pub days: usize,
}

pub const SHOP: Scale = Scale {
    products: 500,
    customers: 1_000,
    bills: 20_000,
    purchases: 2_000,
    gst_bills: 300,
    payments: 2_000,
    crates: 20,
    days: 365,
};

/// `YYYY-MM-DD` for day `n` counted from 2025-01-01 (30-day months, synthetic).
fn date(n: usize) -> String {
    let n = n % 360;
    format!("2025-{:02}-{:02}", n / 30 + 1, n % 30 + 1)
}

fn product_line(p: usize, qty: usize, rate: usize) -> String {
    format!(
        r#"{{"item": "Synthetic Product {p}", "pid": "p_{p}", "packSize": "250 ml", "uom": "Case", "packetQty": 24, "unit": "Packet", "mrp": 0, "rate": {rate}, "qty": {qty}, "amount": {}, "costAtSale": {}}}"#,
        rate * qty,
        rate / 2
    )
}

fn crate_line(c: usize, qty: usize, products: usize) -> String {
    let (a, b) = (c % products, (c * 7 + 3) % products);
    format!(
        r#"{{"isCrate": true, "name": "Synthetic Crate {c}", "components": [{{"item": "Synthetic Product {a}", "pid": "p_{a}", "unit": "Packet", "qty": 2}}, {{"item": "Synthetic Product {b}", "pid": "p_{b}", "unit": "Packet", "qty": 1}}], "rate": 300, "qty": {qty}, "amount": {}, "costAtSale": 150}}"#,
        300 * qty
    )
}

/// The backup file's text.
pub fn legacy_file(scale: &Scale) -> String {
    let mut out = String::with_capacity(scale.bills * 700);
    out.push_str(r#"{"meta": {"name": "Synthetic Scale Shop", "tagline": "Synthetic data only", "loadingCharge": 1, "defaultReorder": 6, "gstRate": 18, "gstHsn": "2202", "schema": 3, "created": 1735689600000}, "products": ["#);
    for p in 0..scale.products {
        let rate = 40 + p % 160;
        let _ = write!(
            out,
            r#"{}{{"id": "p_{p}", "item": "Synthetic Product {p}", "company": "Synthetic Co", "packSize": "250 ml", "uom": "Case", "packetQty": 24, "mrp": 0, "buyRate": {}, "avgCost": {}, "openingCost": {}, "margin": {}, "sellRate": {rate}, "reorderLevel": 5, "gstRate": null, "cess": null, "hsn": null}}"#,
            if p == 0 { "" } else { ", " },
            rate / 2,
            rate / 2,
            rate / 2,
            rate - rate / 2
        );
    }
    out.push_str(r#"], "customers": ["#);
    for c in 0..scale.customers {
        let _ = write!(
            out,
            r#"{}{{"id": "c_{c}", "name": "Synthetic Customer {c}", "mobile": "90000 {c:05}", "addr1": "{c} Synthetic Lane", "addr2": "Kanpur", "openingBalance": {}, "gstin": ""}}"#,
            if c == 0 { "" } else { ", " },
            c % 7 * 10
        );
    }
    out.push_str(r#"], "suppliers": [{"id": "s_1", "name": "Synthetic Wholesale"}, {"id": "s_2", "name": "Synthetic Depot"}], "sales": ["#);
    for b in 0..scale.bills {
        let p = b * 31 % scale.products;
        let mut items = vec![product_line(p, 1 + b % 5, 40 + p % 160)];
        if b % 3 == 0 {
            let q = (p + 11) % scale.products;
            items.push(product_line(q, 2, 40 + q % 160));
        }
        if b % 10 == 0 {
            items.push(crate_line(b % scale.crates, 1 + b % 2, scale.products));
        }
        if b % 20 == 5 {
            items = vec![crate_line(b % scale.crates, 2, scale.products)];
        }
        let total: usize = items
            .iter()
            .map(|i| {
                let at = i.find(r#""amount": "#).unwrap() + 10;
                i[at..].split(',').next().unwrap().parse::<usize>().unwrap()
            })
            .sum();
        let (customer, name) = if b % 9 == 0 {
            (String::new(), "Walk-in".to_owned())
        } else {
            let c = b * 13 % scale.customers;
            (format!("c_{c}"), format!("Synthetic Customer {c}"))
        };
        let _ = write!(
            out,
            r#"{}{{"id": "b_{b}", "no": {}, "date": "{}", "customerId": "{customer}", "customerName": "{name}", "paymentMode": "Cash", "items": [{}], "loading": 0, "loadingRate": 0, "subtotal": {total}, "total": {total}, "paid": {}, "cancelled": {}, "note": "", "created": {}}}"#,
            if b == 0 { "" } else { ", " },
            1001 + b,
            date(b * scale.days / scale.bills.max(1)),
            items.join(", "),
            if b % 4 == 0 { total } else { total / 2 },
            b % 50 == 7,
            1_735_689_600_000_u64 + b as u64 * 60_000
        );
    }
    out.push_str(r#"], "purchases": ["#);
    for u in 0..scale.purchases {
        let p = u * 17 % scale.products;
        let _ = write!(
            out,
            r#"{}{{"id": "pu_{u}", "date": "{}", "supplier": "Synthetic Wholesale", "items": [{{"item": "Synthetic Product {p}", "pid": "p_{p}", "unit": "Packet", "qty": 20, "rate": {}, "amount": {}}}], "total": {}, "created": {}}}"#,
            if u == 0 { "" } else { ", " },
            date(u * scale.days / scale.purchases.max(1)),
            20 + p % 80,
            20 * (20 + p % 80),
            20 * (20 + p % 80),
            1_735_689_600_000_u64 + u as u64 * 3_600_000
        );
    }
    out.push_str(r#"], "purchaseOrders": [], "expenses": ["#);
    for d in 0..scale.days {
        let _ = write!(
            out,
            r#"{}{{"id": "cb_{d}", "date": "{}", "cash": 1000, "paytm": 200, "phonepe": 100, "debt": 0, "goodsDesc": "", "goodsAmt": 300, "miscDesc": "", "miscAmt": 50, "salary": 0}}"#,
            if d == 0 { "" } else { ", " },
            date(d)
        );
    }
    out.push_str(r#"], "payments": ["#);
    for y in 0..scale.payments {
        let c = y * 29 % scale.customers;
        let _ = write!(
            out,
            r#"{}{{"id": "pay_{y}", "customerId": "c_{c}", "customerName": "Synthetic Customer {c}", "amount": {}, "mode": "Cash", "date": "{}", "note": "", "created": {}}}"#,
            if y == 0 { "" } else { ", " },
            50 + y % 200,
            date(y * scale.days / scale.payments.max(1)),
            1_735_689_600_000_u64 + y as u64 * 1_800_000
        );
    }
    out.push_str(r#"], "stock": {"#);
    for p in 0..scale.products {
        let _ = write!(
            out,
            r#"{}"p_{p}": {{"opening": {}, "fridge": {}}}"#,
            if p == 0 { "" } else { ", " },
            500 + p % 100,
            p % 3
        );
    }
    out.push_str(r#"}, "crates": ["#);
    for c in 0..scale.crates {
        let (a, b) = (c % scale.products, (c * 7 + 3) % scale.products);
        let _ = write!(
            out,
            r#"{}{{"id": "cr_{c}", "name": "Synthetic Crate {c}", "components": [{{"item": "Synthetic Product {a}", "pid": "p_{a}", "unit": "Packet", "qty": 2}}, {{"item": "Synthetic Product {b}", "pid": "p_{b}", "unit": "Packet", "qty": 1}}], "price": 300}}"#,
            if c == 0 { "" } else { ", " }
        );
    }
    out.push_str(r#"], "priceSuppliers": ["Synthetic Wholesale"], "gstBills": ["#);
    for g in 0..scale.gst_bills {
        let p = g * 7 % scale.products;
        let _ = write!(
            out,
            r#"{}{{"id": "gb_{g}", "no": {}, "date": "{}", "customerId": "", "customerName": "Synthetic GST Buyer {g}", "custGstin": "", "cancelled": {}, "items": [{{"item": "Synthetic Product {p}", "pid": "p_{p}", "hsn": "22021010", "unit": "Case", "qty": 2, "rate": 62.5, "gst": 18, "packetQty": 1, "amount": 125}}], "isGstBill": true, "created": {}}}"#,
            if g == 0 { "" } else { ", " },
            g + 1,
            date(g * scale.days / scale.gst_bills.max(1)),
            g % 40 == 3,
            1_735_689_600_000_u64 + g as u64 * 86_400_000
        );
    }
    out.push_str(r#"], "stockLog": [], "log": [], "godown": [], "deleted": []}"#);
    out
}
