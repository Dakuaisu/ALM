use super::{DraftId, Money, Receipt, RequestId};
use alm_store::ProvisionalDraft;
use alm_store::confirmation::{Review, pending};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct OwnerRequest {
    pub(super) id: RequestId,
    pub(super) review: Review,
}

impl OwnerRequest {
    pub(super) fn encode(&self) -> String {
        format!(
            "PENDING_OWNER {} {}",
            self.id.value(),
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
    let [line] = draft.lines() else {
        panic!("expected exactly one synthetic product line")
    };
    format!(
        "{} {} {} {:?} {:?}",
        draft.id().value(),
        line.product_id().value(),
        line.quantity(),
        draft.customer(),
        draft.bill_date(),
    )
}

pub(super) fn receipt_frame(receipt: &Receipt) -> String {
    let Receipt {
        draft_id,
        invoice,
        quantity,
        total,
        received,
        due,
        customer_balance,
    } = receipt;
    format!(
        "RECEIPT {} {invoice} {quantity} {} {} {} {}",
        draft_id.value(),
        total.minor_units(),
        received.map_or_else(|| "-".to_owned(), |value| value.minor_units().to_string()),
        due.map_or_else(|| "-".to_owned(), |value| value.minor_units().to_string()),
        customer_balance.map_or_else(|| "-".to_owned(), |value| value.minor_units().to_string())
    )
}

pub(super) fn read_receipt(frame: &str) -> Option<Receipt> {
    let fields: Vec<_> = frame.split(' ').collect();
    let [
        "RECEIPT",
        draft_id,
        invoice,
        quantity,
        total,
        received,
        due,
        balance,
    ] = fields.as_slice()
    else {
        return None;
    };
    Some(Receipt {
        draft_id: DraftId::new(draft_id.parse().ok()?),
        invoice: invoice.parse().ok()?,
        quantity: quantity.parse().ok()?,
        total: Money::from_minor_units(total.parse().ok()?),
        received: if *received == "-" {
            None
        } else {
            Some(Money::from_minor_units(received.parse().ok()?))
        },
        due: if *due == "-" {
            None
        } else {
            Some(Money::from_minor_units(due.parse().ok()?))
        },
        customer_balance: if *balance == "-" {
            None
        } else {
            Some(Money::from_minor_units(balance.parse().ok()?))
        },
    })
}
