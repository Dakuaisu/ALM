use std::num::NonZeroU32;

use crate::{BillDate, Money};

use super::{
    PurchaseBook, PurchaseDraft, PurchaseError, PurchaseLine, PurchaseProduct, PurchaseSupplier,
};

/// The draft has already been filled when these notices are returned.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct PurchasePasteWarnings {
    pub unmatched: Vec<String>,
    pub total_mismatches: Vec<String>,
    /// Empty means no party was found; more than one means the paste mixed parties.
    pub parties: Vec<String>,
    pub skipped_rows: usize,
}

impl PurchasePasteWarnings {
    pub fn unmatched_warning_text(&self) -> Option<String> {
        (!self.unmatched.is_empty()).then(|| {
            format!(
                "⚠ Not in your product list: {} — pick the right name from suggestions before saving",
                self.unmatched.join(", ")
            )
        })
    }

    pub fn mixed_party_warning_text(&self) -> Option<String> {
        let count = self.parties.len();
        (count > 1).then(|| format!("⚠ Paste had {count} parties — all rows went into this one PO"))
    }
}

#[derive(Default)]
struct Columns {
    date: Option<usize>,
    party: Option<usize>,
    item: Option<usize>,
    qty: Option<usize>,
    rate: Option<usize>,
    total: Option<usize>,
}

impl Columns {
    fn from_header(header: &str) -> Option<Self> {
        let mut cols = Self::default();
        for (index, heading) in header.split('\t').enumerate() {
            let heading = heading.trim().to_ascii_lowercase();
            if heading.contains("date") {
                cols.date = Some(index);
            } else if heading.contains("party") || heading.contains("supplier") {
                cols.party = Some(index);
            } else if heading.contains("item") {
                cols.item = Some(index);
            } else if heading.contains("packet") && heading.contains("qty") {
                // Packet Qty is pack metadata, not the ordered case count.
            } else if heading.contains("rate") {
                cols.rate = Some(index);
            } else if heading.contains("total") {
                cols.total = Some(index);
            } else if heading == "qty" || heading.contains("packet") || heading.contains("case") {
                cols.qty = Some(index);
            }
        }
        cols.item?;
        cols.qty?;
        Some(cols)
    }
}

pub(super) fn fill(
    book: &PurchaseBook,
    draft: &mut PurchaseDraft,
    text: &str,
) -> Result<PurchasePasteWarnings, PurchaseError> {
    let mut rows = text.lines().filter(|row| !row.trim().is_empty());
    let cols = rows
        .next()
        .and_then(Columns::from_header)
        .ok_or(PurchaseError::NoUsableRows)?;
    let mut date = None;
    let mut items = Vec::new();
    let mut warnings = PurchasePasteWarnings::default();
    for row in rows {
        let cells: Vec<_> = row.split('\t').map(str::trim).collect();
        let cell = |index: Option<usize>| index.and_then(|i| cells.get(i).copied()).unwrap_or("");
        let party = cell(cols.party);
        if !party.is_empty() && !warnings.parties.iter().any(|name| name == party) {
            warnings.parties.push(party.to_owned());
        }
        date = date.or_else(|| cell(cols.date).parse::<BillDate>().ok());

        let item = cell(cols.item);
        let qty = cell(cols.qty).parse::<NonZeroU32>();
        let (Ok(ordered), Ok(rate), Ok(total)) =
            (qty, money(cell(cols.rate)), money(cell(cols.total)))
        else {
            warnings.skipped_rows += 1;
            continue;
        };
        if item.is_empty() {
            warnings.skipped_rows += 1;
            continue;
        }
        let product = product_match(&book.products, item);
        let item = product.map_or(item, |p| p.item.as_str());
        if product.is_none() {
            warnings.unmatched.push(item.to_owned());
        }
        if let (Some(rate), Some(total)) = (rate, total)
            && total_mismatch(ordered, rate, total)
        {
            warnings.total_mismatches.push(item.to_owned());
        }
        items.push(PurchaseLine {
            item: item.to_owned(),
            product_id: product.map(|p| p.id),
            ordered,
            rate: rate
                .or_else(|| product.map(|p| p.buy_rate))
                .unwrap_or(Money::from_minor_units(0)),
        });
    }
    if items.is_empty() {
        return Err(PurchaseError::NoUsableRows);
    }

    if let Some(name) = warnings.parties.first() {
        let key = supplier_key(name);
        let existing = book.suppliers.iter().find(|s| s.name == *name).or_else(|| {
            if key.is_empty() {
                return None;
            }
            book.suppliers.iter().find(|s| supplier_key(&s.name) == key)
        });
        draft.supplier = Some(PurchaseSupplier {
            id: existing.map(|s| s.id),
            name: existing.map_or_else(|| name.clone(), |s| s.name.clone()),
        });
    }
    if let Some(date) = date {
        draft.date = Some(date);
    }
    draft.items = items;
    Ok(warnings)
}

fn product_match<'a>(products: &'a [PurchaseProduct], name: &str) -> Option<&'a PurchaseProduct> {
    products
        .iter()
        .find(|p| p.item.eq_ignore_ascii_case(name))
        .or_else(|| {
            let key = ascii_name_key(name);
            if key.is_empty() {
                return None;
            }
            products.iter().find(|p| ascii_name_key(&p.item) == key)
        })
        .or_else(|| {
            let name = name.to_ascii_lowercase();
            let mut candidates = products.iter().filter(|p| {
                let item = p.item.to_ascii_lowercase();
                item.contains(&name) || name.contains(&item)
            });
            let first = candidates.next()?;
            candidates.next().is_none().then_some(first)
        })
}

fn ascii_name_key(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn supplier_key(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    let name = name
        .strip_prefix("m/s")
        .or_else(|| name.strip_prefix("ms"))
        .unwrap_or(&name);
    ascii_name_key(name)
}

#[derive(Debug)]
struct InvalidMoney;

fn money(text: &str) -> Result<Option<Money>, InvalidMoney> {
    if text.is_empty() {
        return Ok(None);
    }
    let (sign, digits) = text.strip_prefix('-').map_or((1, text), |rest| (-1, rest));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
    {
        return Err(InvalidMoney);
    }
    let whole = i128::from(whole.parse::<u64>().map_err(|_| InvalidMoney)?);
    let fraction = match fraction.as_bytes() {
        [] => 0,
        [tenths] => i128::from(tenths - b'0') * 10,
        [tenths, hundredths] => i128::from(tenths - b'0') * 10 + i128::from(hundredths - b'0'),
        _ => return Err(InvalidMoney),
    };
    let minor = i64::try_from(sign * (whole * 100 + fraction)).map_err(|_| InvalidMoney)?;
    Ok(Some(Money::from_minor_units(minor)))
}

fn total_mismatch(quantity: NonZeroU32, rate: Money, pasted_total: Money) -> bool {
    let total = i128::from(pasted_total.minor_units());
    let actual = i128::from(rate.minor_units()) * i128::from(quantity.get());
    // Compare in hundredths of a minor unit so the 1% tolerance needs no rounding.
    (actual - total).abs() * 100 > 20_000.max(total)
}
