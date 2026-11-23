use std::num::NonZeroU32;

use alm_core::{
    BillDate, CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftId, DraftLine,
    ProductId, ProvisionalDraft, SaleUnit,
};

pub(super) const DRAFT: DraftId = DraftId::new(21);
pub(super) const MISSING: DraftId = DraftId::new(99);
pub(super) const PRODUCTS: [ProductId; 2] = [ProductId::new(7), ProductId::new(9)];

#[derive(Clone, Copy)]
pub(super) enum Account {
    A,
    B,
}

impl Account {
    pub const fn key(self) -> &'static str {
        match self {
            Self::A => "synthetic-manager-a",
            Self::B => "synthetic-manager-b",
        }
    }

    pub const fn marker(self) -> &'static str {
        match self {
            Self::A => "SYNTHETIC-A",
            Self::B => "SYNTHETIC-B",
        }
    }

    pub const fn saved(self) -> &'static DraftSpec {
        match self {
            Self::A => &A_EDITED,
            Self::B => &B_SAVED,
        }
    }

    pub const fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

pub(super) struct DraftSpec {
    pub account: Account,
    pub buyer: &'static str,
    pub name: &'static str,
    pub date: &'static str,
    pub lines: [(u64, u32); 2],
    crate_qty: u32,
    components: [(u64, u32, SaleUnit); 2],
    pub crate_sql: &'static str,
}

impl DraftSpec {
    pub fn date(&self) -> BillDate {
        self.date.parse().expect("valid fixture date")
    }

    pub fn customer_id(&self) -> DraftCustomerId {
        DraftCustomerId::new(self.buyer.into())
    }

    pub fn draft(&self) -> ProvisionalDraft {
        let lines = self
            .lines
            .map(|(id, qty)| DraftLine::new(ProductId::new(id), positive(qty)));
        let components = self
            .components
            .map(|(id, qty, unit)| CrateComponent::new(ProductId::new(id), positive(qty), unit));
        let mut draft = ProvisionalDraft::new(DRAFT, lines.into())
            .with_bill_date(Some(self.date()))
            .with_customer(DraftCustomer::Selected {
                id: self.customer_id(),
                name: self.name.into(),
            });
        draft
            .add_crate_row(CrateRow::new(positive(self.crate_qty), components.into()))
            .unwrap();
        draft
    }
}

pub(super) const fn positive(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("positive fixture quantity")
}

pub(super) const A_INITIAL: DraftSpec = DraftSpec {
    account: Account::A,
    buyer: "SYNTHETIC-A-buyer",
    name: "SYNTHETIC-A First's Shop",
    date: "2025-02-11",
    lines: [(7, 2), (9, 4)],
    crate_qty: 2,
    components: [(7, 3, SaleUnit::Piece), (9, 1, SaleUnit::Case)],
    crate_sql: "2;7,3,0;9,1,1",
};

pub(super) const A_EDITED: DraftSpec = DraftSpec {
    name: "SYNTHETIC-A Revised's Shop",
    date: "2025-02-12",
    lines: [(9, 3), (7, 1)],
    crate_qty: 1,
    components: [(9, 2, SaleUnit::Case), (7, 4, SaleUnit::Piece)],
    crate_sql: "1;9,2,1;7,4,0",
    ..A_INITIAL
};

pub(super) const B_SAVED: DraftSpec = DraftSpec {
    account: Account::B,
    buyer: "SYNTHETIC-B-buyer",
    name: "SYNTHETIC-B Second's Shop",
    date: "2025-03-01",
    lines: [(7, 5), (9, 2)],
    crate_qty: 3,
    components: [(9, 6, SaleUnit::Piece), (7, 2, SaleUnit::Case)],
    crate_sql: "3;9,6,0;7,2,1",
};

pub(super) const B_LINE_EDITED: DraftSpec = DraftSpec {
    lines: [(7, 5), (9, 8)],
    ..B_SAVED
};
