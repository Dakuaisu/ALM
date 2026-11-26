//! The canonical text of intents and the hosted `post_sale` terms.
use std::num::NonZeroU32;

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft,
};

use super::codec::{self, escape, unescape};
use super::{Intent, SaleIntent};

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
                let load = match draft.loading_rate().minor_units() {
                    0 => String::new(),
                    rate => format!(" load={rate}"),
                };
                // Crate rows beside the lines, each as the core's crate row text, joined by `/`.
                let crates = if draft.crate_rows().is_empty() {
                    String::new()
                } else {
                    format!(
                        " crates={}",
                        draft
                            .crate_rows()
                            .iter()
                            .map(alm_core::encode_crate_row)
                            .collect::<Vec<_>>()
                            .join("/")
                    )
                };
                format!(
                    "sale draft={} customer={customer} billdate={} date={} paid={} lines={lines}{load}{crates}",
                    draft.id(),
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
        let draft_id = |key: &str| DraftId::parse(get(key)?).ok();
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
                let load = match get("load") {
                    Some(text) => Money::from_minor_units(text.parse().ok()?),
                    None => Money::from_minor_units(0),
                };
                let mut draft = ProvisionalDraft::new(draft_id("draft")?, lines)
                    .with_customer(customer)
                    .with_bill_date(bill_date)
                    .with_loading_rate(load);
                if let Some(rows) = get("crates") {
                    for row in rows.split('/') {
                        draft.add_crate_row(alm_core::decode_crate_row(row).ok()?);
                    }
                }
                Self::PostSale(SaleIntent {
                    draft,
                    date: date("date")?,
                    paid: Money::from_minor_units(get("paid")?.parse().ok()?),
                    prices,
                })
            }
            "cancel" => Self::CancelSale {
                draft: draft_id("draft")?,
            },
            "restore" => Self::RestoreSale {
                draft: draft_id("draft")?,
            },
            "edit" => Self::EditSale {
                draft: draft_id("draft")?,
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
        let load = match sale.draft.loading_rate().minor_units() {
            0 => String::new(),
            rate => format!(r#", "loading_minor": {rate}"#),
        };
        Some(format!(
            r#"{{"draft_id": "{}", "lines": [{lines}]{load}}}"#,
            sale.draft.id()
        ))
    }
}
