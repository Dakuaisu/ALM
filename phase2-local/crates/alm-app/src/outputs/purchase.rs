//! Purchase-order outputs: the archived capture and the supplier print.
use alm_core::confirmation::{PurchaseBusiness, PurchaseOutputRequest, PurchaseProductDisplay};
use alm_core::synthetic_read_model::{ArchivedPurchaseOrder, PurchaseCapture};

use crate::{Trial, debug};

impl Trial {
    fn purchase_displays(&self) -> Vec<PurchaseProductDisplay<'_>> {
        self.directory
            .products(&self.posted)
            .map(|(id, item)| PurchaseProductDisplay {
                id,
                item,
                pack_size: "",
                uom: "",
                mrp: None,
            })
            .collect()
    }

    /// Every live purchase order as the archive stores it (`ArchivedPurchaseOrder::capture`).
    pub(crate) fn archived_orders(&self) -> Result<Vec<ArchivedPurchaseOrder>, String> {
        let role = self.require_owner()?;
        let displays = self.purchase_displays();
        let orders = self.posted.purchase_orders(role).map_err(debug)?;
        let mut archived = Vec::new();
        for row in self.posted.purchase_order_list(role).map_err(debug)?.rows {
            let order = orders
                .iter()
                .find(|order| order.id == row.id)
                .ok_or("Purchase order not found")?;
            let view = self
                .posted
                .view_purchase_order(row.selection, &displays)
                .map_err(debug)?;
            archived.push(
                ArchivedPurchaseOrder::capture(
                    role,
                    PurchaseCapture {
                        order,
                        number: row.selection.number,
                        view: &view,
                        business: business(),
                        products: &displays,
                    },
                )
                .map_err(debug)?,
            );
        }
        archived.sort_by_key(|order| order.number);
        Ok(archived)
    }

    /// The live supplier print of PO number `number`.
    pub(crate) fn po_print_html(&self, number: u64) -> Result<String, String> {
        let role = self.require_owner()?;
        let displays = self.purchase_displays();
        let row = self
            .posted
            .purchase_order_list(role)
            .map_err(debug)?
            .rows
            .into_iter()
            .find(|row| row.selection.number == number)
            .ok_or("Purchase order not found")?;
        let mut html = None;
        self.posted
            .print_purchase_order(
                PurchaseOutputRequest {
                    selection: row.selection,
                    business: business(),
                    products: &displays,
                },
                &mut |doc| html = Some(doc.html),
            )
            .map_err(|error| error.to_string())?;
        html.ok_or_else(|| "Purchase order unavailable.".into())
    }
}

pub(crate) const fn business() -> PurchaseBusiness<'static> {
    PurchaseBusiness {
        name: crate::SELLER.name,
        address: crate::SELLER.address,
        mobile: "",
    }
}
