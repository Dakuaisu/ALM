use crate::BillDate;

use super::{PurchaseBook, PurchaseSupplier};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshPurchaseOrderLine {
    pub item: String,
    pub product_id: Option<crate::ProductId>,
    pub ordered: String,
    pub rate: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshPurchaseOrderForm {
    pub preview_number: String,
    pub date: BillDate,
    pub supplier: Option<PurchaseSupplier>,
    pub note: String,
    pub items: Vec<FreshPurchaseOrderLine>,
}

pub(super) fn begin(book: &PurchaseBook, date: BillDate) -> FreshPurchaseOrderForm {
    FreshPurchaseOrderForm {
        preview_number: format!("PO-{:03}", book.orders.len() + 1),
        date,
        supplier: None,
        note: String::new(),
        items: vec![FreshPurchaseOrderLine {
            item: String::new(),
            product_id: None,
            ordered: String::new(),
            rate: String::new(),
        }],
    }
}

#[cfg(test)]
#[path = "fresh_form/tests.rs"]
mod tests;
