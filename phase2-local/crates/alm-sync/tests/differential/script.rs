//! Intent builders for the scripted scenarios.
use std::num::NonZeroU32;

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::ProjectionSnapshot;
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft,
};
use alm_sync::{
    CrateTerms, Intent, ProductTerms, PurchaseLineTerms, PurchaseOrderTerms, SaleIntent,
};

pub const LIME: u64 = 1;
pub const SODA: u64 = 2;
pub const CUSTOMER: (&str, &str) = ("batch1-customer", "Synthetic Customer");
pub const MARKET: (&str, &str) = ("batch1-market", "Synthetic Market");

pub fn day(text: &str) -> BillDate {
    text.parse().unwrap()
}

pub fn rs(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

/// Who a bill is for.
pub enum For<'a> {
    Customer((&'a str, &'a str)),
    Typed(&'a str),
    Walkin,
}

pub struct Bill<'a> {
    pub draft: u64,
    pub to: For<'a>,
    pub lines: &'a [(u64, u32)],
    pub date: &'a str,
    pub paid: i64,
    pub loading: i64,
}

/// A reviewed sale priced at the core's current sell rates.
pub fn sale(core: &ProjectionSnapshot, bill: &Bill<'_>) -> Intent {
    let lines = bill
        .lines
        .iter()
        .map(|(p, q)| DraftLine::new(ProductId::new(*p), NonZeroU32::new(*q).unwrap()))
        .collect();
    let prices = bill
        .lines
        .iter()
        .map(|(p, _)| core.products.iter().find(|x| x.id == *p).unwrap().sell)
        .collect();
    let customer = match bill.to {
        For::Customer((id, name)) => DraftCustomer::Selected {
            id: DraftCustomerId::new(id.into()),
            name: name.into(),
        },
        For::Typed(name) => DraftCustomer::Typed(name.into()),
        For::Walkin => DraftCustomer::Unassigned,
    };
    Intent::PostSale(SaleIntent {
        draft: ProvisionalDraft::new(DraftId::new(bill.draft), lines)
            .with_customer(customer)
            .with_bill_date(Some(day(bill.date)))
            .with_loading_rate(rs(bill.loading)),
        date: day(bill.date),
        paid: rs(bill.paid),
        prices,
    })
}

pub fn receipt(customer: &str, date: &str, paise: i64, online: bool) -> Intent {
    Intent::RecordReceipt {
        customer: customer.into(),
        date: day(date),
        amount: rs(paise),
        online,
    }
}

pub fn cash_day(id: Option<u64>, date: &str, cash: i64, paytm: i64, misc: i64) -> Intent {
    Intent::SaveCashDay {
        id,
        date: day(date),
        amounts: CashBookAmounts {
            cash: rs(cash),
            paytm: rs(paytm),
            misc: rs(misc),
            ..CashBookAmounts::ZERO
        },
    }
}

pub fn product(
    id: u64,
    new: bool,
    item: &str,
    buy: i64,
    sell: i64,
    was: Option<(i64, i64)>,
) -> Intent {
    Intent::SaveProduct(ProductTerms {
        id,
        new,
        item: item.into(),
        brand: String::new(),
        buy: rs(buy),
        sell: rs(sell),
        reorder: None,
        reviewed: was.map(|(b, s)| (rs(b), rs(s))),
    })
}

/// `(item, product, ordered, rate)` lines.
pub fn purchase(supplier: &str, date: &str, lines: &[(&str, Option<u64>, u32, i64)]) -> Intent {
    Intent::SavePurchaseOrder(PurchaseOrderTerms {
        supplier: Some((None, supplier.into())),
        date: Some(day(date)),
        note: String::new(),
        lines: lines
            .iter()
            .map(|(item, product, ordered, rate)| PurchaseLineTerms {
                item: (*item).into(),
                product: *product,
                ordered: *ordered,
                rate: rs(*rate),
            })
            .collect(),
    })
}

/// The PO's remaining quantities as the core shows them.
pub fn remaining(core: &ProjectionSnapshot, po: u64) -> Vec<u32> {
    core.owner
        .as_ref()
        .unwrap()
        .purchase_orders
        .iter()
        .find(|o| o.id == po)
        .map(|o| o.lines.iter().map(|l| l.remaining).collect())
        .unwrap_or_default()
}

pub fn receive(
    core: &ProjectionSnapshot,
    po: u64,
    date: Option<&str>,
    today: &str,
    lines: &[(u32, i64)],
) -> Intent {
    Intent::ReceivePurchaseOrder {
        po,
        date: date.map(day),
        today: day(today),
        lines: lines.iter().map(|(q, r)| (*q, rs(*r))).collect(),
        remaining: remaining(core, po),
    }
}

pub fn crate_terms(
    id: Option<u64>,
    name: &str,
    price: i64,
    components: &[(u64, u32, bool)],
) -> Intent {
    Intent::SaveCrate(CrateTerms {
        id,
        name: name.into(),
        price: rs(price),
        components: components.to_vec(),
    })
}

pub fn on_hand(core: &ProjectionSnapshot, product: u64) -> u32 {
    core.stock
        .iter()
        .find(|s| s.product == product)
        .map_or(0, |s| s.on_hand)
}

pub fn count(
    core: &ProjectionSnapshot,
    product: u64,
    counted: u32,
    fridge: Option<u32>,
    date: &str,
) -> Intent {
    Intent::CountStock {
        product,
        counted,
        fridge,
        date: day(date),
        reason: Some("count".into()),
        note: None,
        on_hand: on_hand(core, product),
    }
}
