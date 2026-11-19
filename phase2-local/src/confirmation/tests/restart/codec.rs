use super::super::Review;
use super::{DraftId, Money, ProductId, Receipt, RequestId};
use crate::{DraftCustomer, DraftLine, ProvisionalDraft};
use std::num::NonZeroU32;

// Test-only: one Owner request for one unassigned draft with exactly one line.
// Every variable Review field is carried through; the shape and role are fixed.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct OwnerRequest {
    pub(super) id: RequestId,
    pub(super) review: Review,
}

impl OwnerRequest {
    pub(super) fn encode(&self) -> String {
        let Review {
            draft,
            draft_revision,
            terms_revision,
            unit_price,
        } = &self.review;
        format!(
            "PENDING_OWNER {} {draft_revision} {terms_revision} {} {}",
            self.id.0,
            unit_price.minor_units(),
            draft_fields(draft)
        )
    }

    pub(super) fn decode(frame: &str) -> Self {
        let fields: Vec<_> = frame.split(' ').collect();
        let [
            "PENDING_OWNER",
            request,
            draft_revision,
            terms_revision,
            price,
            id,
            product,
            quantity,
            "unassigned",
        ] = fields.as_slice()
        else {
            panic!("expected the synthetic single-line Owner request: {frame:?}");
        };
        let line = DraftLine::new(
            ProductId::new(product.parse().expect("product identity")),
            quantity.parse::<NonZeroU32>().expect("positive quantity"),
        );
        Self {
            id: RequestId::new(request.parse().expect("request identity")),
            review: Review {
                draft: ProvisionalDraft::new(
                    DraftId::new(id.parse().expect("draft identity")),
                    vec![line],
                ),
                draft_revision: draft_revision.parse().expect("draft revision"),
                terms_revision: terms_revision.parse().expect("terms revision"),
                unit_price: Money::from_minor_units(price.parse().expect("unit price")),
            },
        }
    }
}

pub(super) fn draft_fields(draft: &ProvisionalDraft) -> String {
    assert_eq!(draft.customer(), &DraftCustomer::Unassigned);
    let [line] = draft.lines() else {
        panic!("expected exactly one synthetic product line")
    };
    format!(
        "{} {} {} unassigned",
        draft.id().value(),
        line.product_id().value(),
        line.quantity()
    )
}

pub(super) fn receipt_frame(receipt: &Receipt) -> String {
    let Receipt {
        draft_id,
        invoice,
        quantity,
        total,
    } = receipt;
    format!(
        "RECEIPT {} {invoice} {quantity} {}",
        draft_id.value(),
        total.minor_units()
    )
}

pub(super) fn read_receipt(frame: &str) -> Option<Receipt> {
    let fields: Vec<_> = frame.split(' ').collect();
    let ["RECEIPT", draft_id, invoice, quantity, total] = fields.as_slice() else {
        return None;
    };
    Some(Receipt {
        draft_id: DraftId::new(draft_id.parse().ok()?),
        invoice: invoice.parse().ok()?,
        quantity: quantity.parse().ok()?,
        total: Money::from_minor_units(total.parse().ok()?),
    })
}
