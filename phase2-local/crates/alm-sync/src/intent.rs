//! Canonical, immutable confirmation intents and outcomes. The text form is what the outbox
//! stores before sending and what the authority compares on a retry: equal text, equal intent.
use std::num::NonZeroU32;

mod codec;
use codec::{escape, unescape};

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft,
};

/// A reviewed sale: the saved draft, the confirmed date and payment, and the unit price of
/// every line as reviewed. The authority rejects it as stale if any price changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaleIntent {
    pub draft: ProvisionalDraft,
    pub date: BillDate,
    pub paid: Money,
    pub prices: Vec<Money>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Intent {
    PostSale(SaleIntent),
    CancelSale {
        draft: u64,
    },
    RestoreSale {
        draft: u64,
    },
    /// `(quantity, rate in paise)` per existing line.
    EditSale {
        draft: u64,
        lines: Vec<(u32, i64)>,
    },
    RecordReceipt {
        customer: String,
        date: BillDate,
        amount: Money,
        online: bool,
    },
    /// Saves a Cash Book day (`id` edits an existing row).
    SaveCashDay {
        id: Option<u64>,
        date: BillDate,
        amounts: CashBookAmounts,
    },
    DeleteCashDay {
        id: u64,
    },
    /// Products add or edit (`new` for add) with the reviewed buy and sell of an edit.
    SaveProduct(ProductTerms),
    DeleteProduct {
        id: u64,
    },
    AddSupplier {
        name: String,
    },
    /// One product row of supplier quotes: `(supplier index, rate or clear)`.
    SetSupplierPrices {
        product: u64,
        rates: Vec<(u64, Option<Money>)>,
    },
    SavePurchaseOrder(PurchaseOrderTerms),
    /// `(quantity, rate)` per PO line, with each line's remaining quantity as reviewed.
    ReceivePurchaseOrder {
        po: u64,
        date: Option<BillDate>,
        today: BillDate,
        lines: Vec<(u32, Money)>,
        remaining: Vec<u32>,
    },
    /// "Close remaining", with each line's remaining quantity as reviewed.
    ClosePurchaseOrder {
        po: u64,
        remaining: Vec<u32>,
    },
    SaveCrate(CrateTerms),
    DeleteCrate {
        id: u64,
    },
    SaveOpening {
        customer: String,
        amount: Option<Money>,
        date: Option<BillDate>,
    },
    /// A manual godown row as typed (cases are validated by the authority).
    SaveGodown {
        item: String,
        cases: String,
        note: String,
    },
    /// A physical count against the on-hand quantity the form showed.
    CountStock {
        product: u64,
        counted: u32,
        fridge: Option<u32>,
        date: BillDate,
        reason: Option<String>,
        note: Option<String>,
        on_hand: u32,
    },
    /// A standalone GST bill record (`GstBill::to_record_text`), numbered by the authority.
    SaveGstBill {
        bill: String,
    },
    CancelGstBill {
        id: u64,
    },
    DeleteGstBill {
        id: u64,
    },
    /// Moves a bill or deletion marker from the device GST tables into the authority.
    ImportGstBill {
        bill: Option<String>,
        deleted: Option<u64>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductTerms {
    pub id: u64,
    pub new: bool,
    pub item: String,
    pub brand: String,
    pub buy: Money,
    pub sell: Money,
    pub reorder: Option<u32>,
    /// For an edit: the buy and sell rates the form was opened with.
    pub reviewed: Option<(Money, Money)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseLineTerms {
    pub item: String,
    pub product: Option<u64>,
    pub ordered: u32,
    pub rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrderTerms {
    /// `(saved supplier index, name)`.
    pub supplier: Option<(Option<u64>, String)>,
    pub date: Option<BillDate>,
    pub note: String,
    pub lines: Vec<PurchaseLineTerms>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateTerms {
    pub id: Option<u64>,
    pub name: String,
    pub price: Money,
    /// `(product, quantity, piece)`.
    pub components: Vec<(u64, u32, bool)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaleReceipt {
    pub draft: u64,
    pub invoice: u64,
    pub quantity: u32,
    pub total: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Sale(SaleReceipt),
    Done,
    GstNumber(u64),
    /// The authority's identity for a created record (supplier, PO, crate template).
    Created(u64),
}

fn amounts_text(a: &CashBookAmounts) -> String {
    [
        a.cash, a.paytm, a.phonepe, a.goods, a.misc, a.salary, a.debt,
    ]
    .iter()
    .map(|m| m.minor_units().to_string())
    .collect::<Vec<_>>()
    .join(",")
}

impl Intent {
    /// The canonical text: one line, fields in a fixed order, values escaped.
    pub fn canonical(&self) -> String {
        match self {
            Self::PostSale(sale) => {
                let draft = &sale.draft;
                let customer = match draft.customer() {
                    DraftCustomer::Selected { id, name } => {
                        format!("s:{}:{}", escape(id.value()), escape(name))
                    }
                    DraftCustomer::Typed(name) => format!("t:{}", escape(name)),
                    DraftCustomer::Unassigned => "u".into(),
                };
                let lines = draft
                    .lines()
                    .iter()
                    .zip(&sale.prices)
                    .map(|(line, price)| {
                        format!(
                            "{}x{}@{}",
                            line.product_id().value(),
                            line.quantity(),
                            price.minor_units()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "sale draft={} customer={customer} billdate={} date={} paid={} lines={lines}",
                    draft.id().value(),
                    draft
                        .bill_date()
                        .map_or_else(|| "-".into(), |d| d.to_string()),
                    sale.date,
                    sale.paid.minor_units()
                )
            }
            Self::CancelSale { draft } => format!("cancel draft={draft}"),
            Self::RestoreSale { draft } => format!("restore draft={draft}"),
            Self::EditSale { draft, lines } => format!(
                "edit draft={draft} lines={}",
                lines
                    .iter()
                    .map(|(q, r)| format!("{q}@{r}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::RecordReceipt {
                customer,
                date,
                amount,
                online,
            } => format!(
                "receipt customer={} date={date} amount={} mode={}",
                escape(customer),
                amount.minor_units(),
                if *online { "online" } else { "cash" }
            ),
            Self::SaveCashDay { id, date, amounts } => format!(
                "cashday id={} date={date} amounts={}",
                id.map_or_else(|| "-".into(), |id| id.to_string()),
                amounts_text(amounts)
            ),
            Self::DeleteCashDay { id } => format!("cashdel id={id}"),
            other => codec::encode(other).unwrap_or_default(),
        }
    }

    /// Reads a canonical line back; `None` for anything `canonical` would not produce.
    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split(' ');
        let kind = words.next()?;
        let fields: Vec<(&str, &str)> = words.map(|w| w.split_once('=')).collect::<Option<_>>()?;
        let get = |key: &str| fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let num = |key: &str| get(key)?.parse::<u64>().ok();
        let date = |key: &str| get(key)?.parse::<BillDate>().ok();
        let intent = match kind {
            "sale" => {
                let customer = get("customer")?;
                let customer = match customer.split(':').collect::<Vec<_>>().as_slice() {
                    ["s", id, name] => DraftCustomer::Selected {
                        id: DraftCustomerId::new(unescape(id)?),
                        name: unescape(name)?,
                    },
                    ["t", name] => DraftCustomer::Typed(unescape(name)?),
                    ["u"] => DraftCustomer::Unassigned,
                    _ => return None,
                };
                let mut lines = Vec::new();
                let mut prices = Vec::new();
                for part in get("lines")?.split(',').filter(|p| !p.is_empty()) {
                    let (product, rest) = part.split_once('x')?;
                    let (qty, price) = rest.split_once('@')?;
                    lines.push(DraftLine::new(
                        ProductId::new(product.parse().ok()?),
                        qty.parse::<NonZeroU32>().ok()?,
                    ));
                    prices.push(Money::from_minor_units(price.parse().ok()?));
                }
                let bill_date = match get("billdate")? {
                    "-" => None,
                    text => Some(text.parse().ok()?),
                };
                Self::PostSale(SaleIntent {
                    draft: ProvisionalDraft::new(DraftId::new(num("draft")?), lines)
                        .with_customer(customer)
                        .with_bill_date(bill_date),
                    date: date("date")?,
                    paid: Money::from_minor_units(get("paid")?.parse().ok()?),
                    prices,
                })
            }
            "cancel" => Self::CancelSale {
                draft: num("draft")?,
            },
            "restore" => Self::RestoreSale {
                draft: num("draft")?,
            },
            "edit" => Self::EditSale {
                draft: num("draft")?,
                lines: get("lines")?
                    .split(',')
                    .map(|part| {
                        let (q, r) = part.split_once('@')?;
                        Some((q.parse().ok()?, r.parse().ok()?))
                    })
                    .collect::<Option<_>>()?,
            },
            "receipt" => Self::RecordReceipt {
                customer: unescape(get("customer")?)?,
                date: date("date")?,
                amount: Money::from_minor_units(get("amount")?.parse().ok()?),
                online: match get("mode")? {
                    "online" => true,
                    "cash" => false,
                    _ => return None,
                },
            },
            "cashday" => {
                let values: Vec<Money> = get("amounts")?
                    .split(',')
                    .map(|v| v.parse().ok().map(Money::from_minor_units))
                    .collect::<Option<_>>()?;
                let [cash, paytm, phonepe, goods, misc, salary, debt] = values.as_slice() else {
                    return None;
                };
                Self::SaveCashDay {
                    id: match get("id")? {
                        "-" => None,
                        id => Some(id.parse().ok()?),
                    },
                    date: date("date")?,
                    amounts: CashBookAmounts {
                        cash: *cash,
                        paytm: *paytm,
                        phonepe: *phonepe,
                        goods: *goods,
                        misc: *misc,
                        salary: *salary,
                        debt: *debt,
                    },
                }
            }
            "cashdel" => Self::DeleteCashDay { id: num("id")? },
            other => codec::parse(other, fields)?,
        };
        (intent.canonical() == text).then_some(intent)
    }

    /// The `post_sale` terms JSON of the hosted schema: `{"draft_id", "lines": [{"product_id",
    /// "quantity", "unit_price_minor"}]}`; `None` for other intents.
    pub fn supabase_terms(&self) -> Option<String> {
        let Self::PostSale(sale) = self else {
            return None;
        };
        let lines = sale
            .draft
            .lines()
            .iter()
            .zip(&sale.prices)
            .map(|(line, price)| {
                format!(
                    r#"{{"product_id": {}, "quantity": {}, "unit_price_minor": {}}}"#,
                    line.product_id().value(),
                    line.quantity(),
                    price.minor_units()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            r#"{{"draft_id": "{}", "lines": [{lines}]}}"#,
            sale.draft.id().value()
        ))
    }
}

impl Outcome {
    pub fn canonical(&self) -> String {
        match self {
            Self::Sale(r) => format!(
                "sale draft={} invoice={} quantity={} total={}",
                r.draft,
                r.invoice,
                r.quantity,
                r.total.minor_units()
            ),
            Self::Done => "done".into(),
            Self::GstNumber(n) => format!("gst number={n}"),
            Self::Created(id) => format!("created id={id}"),
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split(' ');
        let kind = words.next()?;
        let fields: Vec<(&str, &str)> = words.map(|w| w.split_once('=')).collect::<Option<_>>()?;
        let num = |key: &str| {
            fields
                .iter()
                .find(|(k, _)| *k == key)
                .and_then(|(_, v)| v.parse::<i64>().ok())
        };
        let outcome = match kind {
            "sale" => Self::Sale(SaleReceipt {
                draft: u64::try_from(num("draft")?).ok()?,
                invoice: u64::try_from(num("invoice")?).ok()?,
                quantity: u32::try_from(num("quantity")?).ok()?,
                total: Money::from_minor_units(num("total")?),
            }),
            "done" => Self::Done,
            "gst" => Self::GstNumber(u64::try_from(num("number")?).ok()?),
            "created" => Self::Created(u64::try_from(num("id")?).ok()?),
            _ => return None,
        };
        (outcome.canonical() == text).then_some(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_text_round_trips_and_rejects_other_text() {
        let draft = ProvisionalDraft::new(
            DraftId::new(7),
            vec![DraftLine::new(
                ProductId::new(1),
                NonZeroU32::new(2).unwrap(),
            )],
        )
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("batch1-customer".into()),
            name: "Synthetic Customer: A&B".into(),
        })
        .with_bill_date(Some(BillDate::new(2025, 4, 30).unwrap()));
        let sale = Intent::PostSale(SaleIntent {
            draft,
            date: BillDate::new(2025, 4, 30).unwrap(),
            paid: Money::from_minor_units(2_000),
            prices: vec![Money::from_minor_units(12_050)],
        });
        let text = sale.canonical();
        assert_eq!(
            text,
            "sale draft=7 customer=s:batch1-customer:Synthetic%20Customer%3A%20A%26B billdate=2025-04-30 date=2025-04-30 paid=2000 lines=1x2@12050"
        );
        assert_eq!(Intent::parse(&text), Some(sale.clone()));
        assert_eq!(
            sale.supabase_terms().unwrap(),
            r#"{"draft_id": "7", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 12050}]}"#
        );
        for intent in [
            Intent::CancelSale { draft: 7 },
            Intent::EditSale {
                draft: 7,
                lines: vec![(3, 11_800)],
            },
            Intent::RecordReceipt {
                customer: "batch1-market".into(),
                date: BillDate::new(2025, 5, 1).unwrap(),
                amount: Money::from_minor_units(3_050),
                online: true,
            },
            Intent::SaveCashDay {
                id: None,
                date: BillDate::new(2025, 4, 30).unwrap(),
                amounts: CashBookAmounts::ZERO,
            },
        ] {
            assert_eq!(Intent::parse(&intent.canonical()), Some(intent));
        }
        let day = BillDate::new(2025, 4, 30).unwrap();
        for intent in [
            Intent::SaveProduct(ProductTerms {
                id: 3,
                new: false,
                item: "Orange: 1,5 L".into(),
                brand: String::new(),
                buy: Money::from_minor_units(3_000),
                sell: Money::from_minor_units(4_700),
                reorder: None,
                reviewed: Some((
                    Money::from_minor_units(3_000),
                    Money::from_minor_units(4_500),
                )),
            }),
            Intent::SetSupplierPrices {
                product: 3,
                rates: vec![(0, None), (2, Some(Money::from_minor_units(2_950)))],
            },
            Intent::SavePurchaseOrder(PurchaseOrderTerms {
                supplier: Some((None, "~".into())),
                date: None,
                note: "a=b c".into(),
                lines: vec![PurchaseLineTerms {
                    item: "Soda, 1:2".into(),
                    product: None,
                    ordered: 10,
                    rate: Money::from_minor_units(3_600),
                }],
            }),
            Intent::ReceivePurchaseOrder {
                po: 0,
                date: Some(day),
                today: day,
                lines: vec![(4, Money::from_minor_units(2_950))],
                remaining: vec![10],
            },
            Intent::ClosePurchaseOrder {
                po: 0,
                remaining: Vec::new(),
            },
            Intent::SaveCrate(CrateTerms {
                id: Some(1),
                name: "Party".into(),
                price: Money::from_minor_units(40_000),
                components: vec![(1, 2, false), (2, 1, true)],
            }),
            Intent::SaveOpening {
                customer: "batch1-market".into(),
                amount: Some(Money::from_minor_units(-500)),
                date: None,
            },
            Intent::SaveGodown {
                item: "Soda".into(),
                cases: " 9".into(),
                note: String::new(),
            },
            Intent::CountStock {
                product: 2,
                counted: 37,
                fridge: None,
                date: day,
                reason: Some("damage".into()),
                note: None,
                on_hand: 40,
            },
            Intent::SaveGstBill {
                bill: r#"{"id":2}"#.into(),
            },
            Intent::ImportGstBill {
                bill: None,
                deleted: Some(6),
            },
            Intent::DeleteProduct { id: 4 },
            Intent::AddSupplier { name: "X Y".into() },
            Intent::DeleteCrate { id: 1 },
            Intent::CancelGstBill { id: 2 },
            Intent::DeleteGstBill { id: 2 },
        ] {
            assert_eq!(Intent::parse(&intent.canonical()), Some(intent));
        }
        assert_eq!(Outcome::parse("created id=2"), Some(Outcome::Created(2)));
        assert_eq!(Intent::parse("cancel draft=7 extra=1"), None);
        assert_eq!(Intent::parse("cancel draft=07"), None);
        let outcome = Outcome::Sale(SaleReceipt {
            draft: 7,
            invoice: 3,
            quantity: 2,
            total: Money::from_minor_units(24_100),
        });
        assert_eq!(Outcome::parse(&outcome.canonical()), Some(outcome));
    }
}
