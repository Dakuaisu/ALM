//! L08-04 static source-derived synthetic comparison, not observed legacy runtime.
//! ALM_Business_Manager.html: Edit gate 6338–6341, draft copy 6521–6532,
//! save reconstruction 6735–6764, quantity 1097–1100, FIFO 1198–1222 and
//! 1308–1337, movement register 10866–10880.
//! The model isolates named, product-linked PO lines with positive rates and no other stock activity.

use super::{BillDate, Money, NonZeroU32, ProductId, PurchaseLine, PurchaseReceipt};

const ALPHA: ProductId = ProductId::new(1);
const BETA: ProductId = ProductId::new(2);

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceLine {
    draft: PurchaseLine,
    received: u32,
    cancelled: u32,
    receipts: Option<Vec<PurchaseReceipt>>,
}

struct SourceOrder {
    date: BillDate,
    note: &'static str,
    lines: Vec<SourceLine>,
}

impl SourceOrder {
    fn rebuild(&self, note: &'static str, draft: &[PurchaseLine]) -> Self {
        let lines = draft.iter().cloned().map(|draft| {
            let previous = self.lines.iter().find(|line| line.draft.item == draft.item);
            SourceLine {
                draft,
                received: previous.map_or(0, |line| line.received),
                cancelled: previous.map_or(0, |line| line.cancelled),
                receipts: None,
            }
        });
        Self {
            date: self.date,
            note,
            lines: lines.collect(),
        }
    }

    fn product_lines(&self, product: ProductId) -> impl Iterator<Item = &SourceLine> {
        self.lines
            .iter()
            .filter(move |line| line.draft.product_id == Some(product))
    }

    fn quantity(&self, product: ProductId) -> u32 {
        self.product_lines(product).map(|line| line.received).sum()
    }

    fn movements(&self, product: ProductId) -> Vec<(BillDate, u32)> {
        let mut movements = Vec::new();
        for line in self.product_lines(product) {
            let mut logged = 0;
            for receipt in line.receipts.as_deref().unwrap_or_default() {
                movements.push((receipt.date, receipt.quantity.get()));
                logged += receipt.quantity.get();
            }
            let remainder = line.received.saturating_sub(logged);
            if remainder > 0 {
                movements.push((self.date, remainder));
            }
        }
        movements
    }

    fn fifo_value(&self, product: ProductId) -> Money {
        let value = self
            .product_lines(product)
            .map(|line| match line.receipts.as_deref() {
                Some(receipts) if !receipts.is_empty() => receipts
                    .iter()
                    .map(|receipt| i64::from(receipt.quantity.get()) * receipt.rate.minor_units())
                    .sum(),
                Some(_) | None => i64::from(line.received) * line.draft.rate.minor_units(),
            });
        Money::from_minor_units(value.sum())
    }
}

fn jan(day: u8) -> BillDate {
    BillDate::new(2025, 1, day).unwrap()
}

fn prior_order() -> SourceOrder {
    SourceOrder {
        date: jan(10),
        note: "Original",
        lines: vec![
            SourceLine {
                draft: PurchaseLine {
                    item: "Alpha".into(),
                    product_id: Some(ALPHA),
                    ordered: NonZeroU32::new(5).unwrap(),
                    rate: Money::from_minor_units(1_000),
                },
                received: 1,
                cancelled: 1,
                receipts: Some(vec![PurchaseReceipt {
                    date: jan(12),
                    quantity: NonZeroU32::MIN,
                    rate: Money::from_minor_units(1_400),
                }]),
            },
            SourceLine {
                draft: PurchaseLine {
                    item: "Beta".into(),
                    product_id: Some(BETA),
                    ordered: NonZeroU32::new(4).unwrap(),
                    rate: Money::from_minor_units(2_000),
                },
                received: 2,
                cancelled: 1,
                receipts: None,
            },
        ],
    }
}

#[test]
fn l08_04_static_legacy_note_only_keeps_quantity_but_rebooks_and_reprices_fifo() {
    let before = prior_order();
    let [alpha, beta] = before.lines.as_slice() else {
        panic!("expected Alpha and Beta")
    };
    assert_eq!(before.quantity(ALPHA), 1);
    assert_eq!(before.movements(ALPHA), vec![(jan(12), 1)]);
    assert_eq!(before.fifo_value(ALPHA), Money::from_minor_units(1_400));

    let after = before.rebuild("Revised", &[alpha.draft.clone(), beta.draft.clone()]);

    assert_eq!(after.note, "Revised");
    assert_eq!(after.date, jan(10));
    assert_eq!(
        after.lines,
        [
            SourceLine {
                draft: alpha.draft.clone(),
                received: 1,
                cancelled: 1,
                receipts: None,
            },
            beta.clone(),
        ]
    );
    assert_eq!(after.lines[0].draft.ordered.get(), 5);
    assert_eq!(after.quantity(ALPHA), 1);
    assert_eq!(after.movements(ALPHA), vec![(jan(10), 1)]);
    assert_eq!(after.fifo_value(ALPHA), Money::from_minor_units(1_000));
}

#[test]
fn l08_04_static_legacy_case_rename_with_same_id_loses_counts_and_stock_effects() {
    let before = prior_order();
    let renamed = PurchaseLine {
        item: "alpha".into(),
        ..before.lines[0].draft.clone()
    };
    let beta = &before.lines[1];

    let after = before.rebuild(before.note, &[renamed.clone(), beta.draft.clone()]);

    assert_eq!(renamed.product_id, Some(ALPHA));
    assert_eq!(
        after.lines,
        [
            SourceLine {
                draft: renamed,
                received: 0,
                cancelled: 0,
                receipts: None,
            },
            beta.clone(),
        ]
    );
    assert_eq!(after.lines[0].draft.ordered.get(), 5);
    assert_eq!(after.quantity(ALPHA), 0);
    assert!(after.movements(ALPHA).is_empty());
    assert_eq!(after.fifo_value(ALPHA), Money::from_minor_units(0));
}

#[test]
fn l08_04_static_legacy_removed_line_is_not_carried_forward() {
    let before = prior_order();
    let beta = &before.lines[1];

    let after = before.rebuild(before.note, std::slice::from_ref(&beta.draft));

    assert_eq!(after.lines.as_slice(), std::slice::from_ref(beta));
    assert_eq!(after.quantity(ALPHA), 0);
    assert!(after.movements(ALPHA).is_empty());
    assert_eq!(after.fifo_value(ALPHA), Money::from_minor_units(0));
}

#[test]
fn l08_04_static_legacy_duplicate_names_both_copy_the_first_exact_matches_counts() {
    let mut before = prior_order();
    before.lines[1].draft.item = "Alpha".into();
    before.lines[1].cancelled = 2;
    let draft = [before.lines[0].draft.clone(), before.lines[1].draft.clone()];

    let after = before.rebuild(before.note, &draft);
    before.lines.swap(0, 1);
    let reversed = before.rebuild(before.note, &draft);

    for (saved, received, cancelled) in [(after, 1, 1), (reversed, 2, 2)] {
        assert_eq!(
            saved.lines,
            draft.clone().map(|draft| SourceLine {
                draft,
                received,
                cancelled,
                receipts: None,
            })
        );
    }
}
