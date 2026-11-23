//! Read accessors for the storage crate's restart replays; compiled only with `test-support`.
use std::collections::BTreeMap;

use super::catalog::Catalog;
use super::{ConfirmationAck, ConfirmationFixture, Receipt, RequestId, Review};
use crate::DraftId;

#[doc(hidden)]
impl ConfirmationFixture {
    pub const fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Lets a replay seed lots the way the fixture's own tests do.
    pub fn catalog_mut(&mut self) -> &mut Catalog {
        &mut self.catalog
    }

    pub const fn checkout_approvals(&self) -> &BTreeMap<DraftId, Review> {
        &self.checkout_approvals
    }

    pub const fn customer_balances(&self) -> &BTreeMap<String, i64> {
        &self.customer_balances
    }

    pub const fn next_invoice(&self) -> u64 {
        self.next_invoice
    }

    pub const fn requests(&self) -> &BTreeMap<RequestId, (Review, Receipt)> {
        &self.requests
    }

    pub const fn posted(&self) -> &BTreeMap<DraftId, RequestId> {
        &self.posted
    }
}

#[doc(hidden)]
impl ConfirmationAck {
    /// Rebuilds an acknowledgement that a replay carried over its own transport.
    pub const fn from_parts(request: RequestId, review: Review, receipt: Receipt) -> Self {
        Self {
            request,
            review,
            receipt,
        }
    }

    pub const fn request(&self) -> RequestId {
        self.request
    }

    pub const fn review(&self) -> &Review {
        &self.review
    }
}
