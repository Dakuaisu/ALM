use std::cmp::Reverse;
use std::collections::BTreeSet;

use alm_phase2_local::confirmation::{
    CustomerLedgerKind, DashboardPeriod, ReceiptMode, SalesPeriod,
};
use alm_phase2_local::{BillDate, DraftCustomer, Money};
use serde_json::{Value, json};

use super::OWNER;
use super::fixture::{CATALOG, Fixture};

pub(super) fn cutoff(today: BillDate) -> BillDate {
    // UTC replay of shiftDate(todayISO(), -45); local-DST behavior is not modeled.
    let iso = today.to_string();
    let mut year = iso[..4].parse().unwrap();
    let mut month = iso[5..7].parse().unwrap();
    let mut day: u8 = iso[8..].parse().unwrap();
    for _ in 0..45 {
        if day > 1 {
            day -= 1;
        } else {
            (year, month) = if month == 1 {
                (year - 1, 12)
            } else {
                (year, month - 1)
            };
            day = (28..=31)
                .rev()
                .find(|&d| BillDate::new(year, month, d).is_ok())
                .unwrap();
        }
    }
    BillDate::new(year, month, day).unwrap()
}

const fn rupees(amount: Money) -> i64 {
    let paise = amount.minor_units();
    paise.div_euclid(100) + if paise.rem_euclid(100) >= 50 { 1 } else { 0 }
}

pub(super) const fn mode_name(mode: ReceiptMode) -> &'static str {
    match mode {
        ReceiptMode::Cash => "Cash",
        ReceiptMode::Online => "Online",
    }
}

pub(super) fn tabs(fixture: &Fixture, today: BillDate) -> Value {
    let app = &fixture.app;
    let cut = cutoff(today);
    let mut sales = app
        .sales_history(OWNER, today, SalesPeriod::Custom((Some(cut), None)), "")
        .unwrap()
        .rows;
    // Sales History gives newest post first; preserve that deterministic tie order locally.
    sales.sort_by_key(|row| Reverse(row.date.to_string()));
    let mut dates = BTreeSet::new();
    let sale_rows: Vec<_> = sales
        .iter()
        .map(|row| {
            let (review, bill) = app.current_sale(OWNER, row.bill.draft_id).unwrap();
            let DraftCustomer::Selected { name, .. } = review.draft().customer() else {
                panic!("named sale fixture")
            };
            let date = row.date.to_string();
            dates.insert(date.clone());
            json!([
                date,
                bill.invoice,
                name,
                rupees(bill.total),
                rupees(bill.received.unwrap()),
                mode_name(review.payment_mode()),
                if row.cancelled { "CANCELLED" } else { "" }
            ])
        })
        .collect();

    let cut = cut.to_string();
    let mut payments = Vec::new();
    let mut balances = Vec::new();
    for contact in &fixture.contacts {
        let ledger = app.customer_ledger(OWNER, &contact.id).unwrap();
        let balance = rupees(ledger.closing);
        if balance != 0 {
            balances.push((contact, balance));
        }
        for row in ledger.entries {
            if let CustomerLedgerKind::Receipt { mode } = row.kind {
                let date = row.date.to_string();
                if date >= cut {
                    dates.insert(date.clone());
                    payments.push((date, contact.name, rupees(row.credit), mode_name(mode)));
                }
            }
        }
    }
    payments.sort_by_key(|row| Reverse(row.0.clone()));
    balances.sort_by_key(|(_, balance)| Reverse(*balance));
    let debtor_rows: Vec<_> = balances
        .into_iter()
        .map(|(c, bal)| json!([c.name, c.mobile, bal]))
        .collect();
    let stock_rows = CATALOG.map(|(id, item)| {
        let lots = app.open_stock_lots(OWNER, id).unwrap();
        json!([
            item,
            lots.on_hand.to_string(),
            rupees(lots.total_value.unwrap())
        ])
    });
    let day_rows: Vec<_> = dates
        .into_iter()
        .rev()
        .filter_map(|date| {
            let day = app
                .dashboard(OWNER, date.parse().unwrap(), DashboardPeriod::Today)
                .unwrap()
                .current;
            (day.bill_count != 0 || !day.receipt_inputs.is_empty()).then(|| {
                json!([
                    date,
                    day.bill_count,
                    rupees(day.sales),
                    rupees(day.collected)
                ])
            })
        })
        .collect();

    json!({
        "Day Summary": { "headers": ["Date", "Bills", "Sales ₹", "Collected ₹"], "rows": day_rows },
        "Sales": { "headers": ["Date", "Bill", "Customer", "Amount ₹", "Paid ₹", "Mode", "Status"], "rows": sale_rows },
        "Udhaar": { "headers": ["Customer", "Mobile", "Balance ₹"], "rows": debtor_rows },
        "Stock": { "headers": ["Item", "On Hand (pkt)", "FIFO Value ₹"], "rows": stock_rows },
        "Payments": { "headers": ["Date", "Customer", "Amount ₹", "Mode"], "rows": payments },
    })
}
