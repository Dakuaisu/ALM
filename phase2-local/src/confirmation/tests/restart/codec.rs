use super::{DraftId, Money, Receipt, RequestId};
use crate::confirmation::{Review, pending};
use crate::{DraftCustomer, ProvisionalDraft};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct OwnerRequest {
    pub(super) id: RequestId,
    pub(super) review: Review,
}

impl OwnerRequest {
    pub(super) fn encode(&self) -> String {
        format!(
            "PENDING_OWNER {} {}",
            self.id.0,
            pending::encode(&self.review)
        )
    }

    pub(super) fn decode(frame: &str) -> Self {
        let rest = frame.strip_prefix("PENDING_OWNER ").expect("owner frame");
        let (request, snapshot) = rest.split_once(' ').expect("request and review");
        Self {
            id: RequestId::new(request.parse().expect("request identity")),
            review: pending::decode(snapshot).expect("review snapshot"),
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
