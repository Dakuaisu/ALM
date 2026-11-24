//! Per-line PACK/MRP details and the customer's contact for the bill image, from the live
//! directory or from a reopened archive. Both read the same directory-derived values.
use super::bill_image::BillImageLine;
use super::owner_billing::BillingDirectory;
use crate::synthetic_read_model::Records;
use crate::{Money, ProductId};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BillImageDetails {
    pub lines: Vec<BillImageLine>,
    pub customer_mobile: String,
    pub customer_address: String,
}

impl BillingDirectory {
    /// Details for a bill to `customer` (by name, as the bill shows it) over `products`.
    pub fn bill_image_details(
        &self,
        customer: &str,
        products: impl IntoIterator<Item = ProductId>,
    ) -> BillImageDetails {
        let lines = products
            .into_iter()
            .map(|id| {
                self.products
                    .iter()
                    .find(|product| product.id == id)
                    .map_or_else(BillImageLine::default, |product| BillImageLine {
                        pack: product.pack(),
                        packet_quantity: product.packet_quantity.max(1),
                        mrp: product.mrp,
                    })
            })
            .collect();
        let contact = self.customers.iter().find(|c| c.name == customer);
        BillImageDetails {
            lines,
            customer_mobile: contact.map_or_else(String::new, |c| c.mobile.to_owned()),
            customer_address: contact.map_or_else(String::new, |c| c.address()),
        }
    }
}

impl Records {
    /// Details for archived sale `sale_id`; the MRP comes from the product of the same item
    /// name (`prodOfLine`). `None` when the sale is not archived.
    pub fn bill_image_details(&self, sale_id: &str) -> Option<BillImageDetails> {
        let sale = self.sales.iter().find(|sale| sale.id == sale_id)?;
        let lines = sale
            .lines
            .iter()
            .map(|line| {
                let product = self.products.iter().find(|p| p.item == line.item);
                BillImageLine {
                    pack: format!("{} {}", line.pack, line.uom).trim().to_owned(),
                    packet_quantity: line.packet_quantity.max(1),
                    mrp: product
                        .map(|p| Money::from_minor_units(p.mrp_minor))
                        .filter(|mrp| mrp.minor_units() > 0),
                }
            })
            .collect();
        let contact = self.customers.iter().find(|c| c.name == sale.customer);
        Some(BillImageDetails {
            lines,
            customer_mobile: contact.map_or_else(String::new, |c| c.mobile.clone()),
            customer_address: contact.map_or_else(String::new, |c| c.address.clone()),
        })
    }
}
