use std::num::{NonZeroU32, NonZeroUsize};

use crate::{BillDate, Money};

/// Longest origin (`<account>:<device>`) a draft ID carries; with `-` and a 20-digit sequence
/// the text stays within the 64 bytes a request ID may use.
pub const DRAFT_ORIGIN_MAX: usize = 43;

/// Identity of a draft; not an invoice number. A draft made on a device carries its origin
/// (`<account>:<device>`, as request IDs do) and that device's local sequence, so drafts are
/// unique across the shop. An origin-less ID is the earlier per-device number, kept readable.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct DraftId {
    origin: [u8; DRAFT_ORIGIN_MAX],
    origin_len: u8,
    seq: u64,
}

/// Draft ID text that is neither a decimal number nor `<origin>-<sequence>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDraftId;

impl DraftId {
    /// An origin-less ID: the earlier per-device numbering, also used by fixtures.
    pub const fn new(value: u64) -> Self {
        Self {
            origin: [0; DRAFT_ORIGIN_MAX],
            origin_len: 0,
            seq: value,
        }
    }

    /// A shop-wide ID: `origin` is `<account>:<device>` (ASCII letters, digits, `_`, `-`, one
    /// `:` inside), `seq` the device's local draft number.
    ///
    /// # Errors
    /// Rejects an empty, malformed or over-long origin.
    pub fn scoped(origin: &str, seq: u64) -> Result<Self, InvalidDraftId> {
        let bytes = origin.as_bytes();
        let valid_byte = |b: &u8| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':');
        let colon = origin.split_once(':');
        if bytes.is_empty()
            || bytes.len() > DRAFT_ORIGIN_MAX
            || !bytes.iter().all(valid_byte)
            || bytes.iter().filter(|b| **b == b':').count() != 1
            || colon.is_none_or(|(_, device)| device.is_empty())
        {
            return Err(InvalidDraftId);
        }
        let mut stored = [0; DRAFT_ORIGIN_MAX];
        stored[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            origin: stored,
            origin_len: bytes.len() as u8,
            seq,
        })
    }

    /// Reads the text [`Display`](std::fmt::Display) writes: digits, or `<origin>-<sequence>`.
    ///
    /// # Errors
    /// Rejects any other text.
    pub fn parse(text: &str) -> Result<Self, InvalidDraftId> {
        let digits = |t: &str| {
            !t.is_empty()
                && t.len() <= 20
                && t.bytes().all(|b| b.is_ascii_digit())
                && (t == "0" || !t.starts_with('0'))
        };
        if digits(text) {
            return text.parse().map(Self::new).map_err(|_| InvalidDraftId);
        }
        let (origin, seq) = text.rsplit_once('-').ok_or(InvalidDraftId)?;
        if !digits(seq) {
            return Err(InvalidDraftId);
        }
        Self::scoped(origin, seq.parse().map_err(|_| InvalidDraftId)?)
    }

    /// The device's local draft number (what the bill form shows as `Draft #n`).
    #[doc(hidden)]
    pub const fn value(self) -> u64 {
        self.seq
    }

    /// `<account>:<device>`, or empty for an origin-less ID.
    pub fn origin(&self) -> &str {
        std::str::from_utf8(&self.origin[..usize::from(self.origin_len)]).unwrap_or_default()
    }

    /// Row form: an origin-less ID stays the number it always was, a scoped one is its text.
    #[doc(hidden)]
    pub fn to_json(self) -> serde_json::Value {
        match self.origin() {
            "" => serde_json::Value::from(self.seq),
            _ => serde_json::Value::from(self.to_string()),
        }
    }

    /// Reads [`Self::to_json`], or a number written as text.
    #[doc(hidden)]
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Number(n) => n.as_u64().map(Self::new),
            serde_json::Value::String(text) => Self::parse(text).ok(),
            _ => None,
        }
    }

    /// The same origin with another local sequence.
    #[must_use]
    pub const fn with_value(self, seq: u64) -> Self {
        Self { seq, ..self }
    }
}

impl Ord for DraftId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.origin(), self.seq).cmp(&(other.origin(), other.seq))
    }
}

impl PartialOrd for DraftId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for DraftId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.origin() {
            "" => write!(f, "{}", self.seq),
            origin => write!(f, "{origin}-{}", self.seq),
        }
    }
}

impl std::fmt::Debug for DraftId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DraftId({self})")
    }
}

impl std::str::FromStr for DraftId {
    type Err = InvalidDraftId;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// An origin-less ID serializes as the number it always was; a scoped one as its text. Both
/// forms are read back, so journals written by earlier builds stay readable.
#[cfg(feature = "demo-fixture")]
impl serde::Serialize for DraftId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.origin() {
            "" => serializer.serialize_u64(self.seq),
            _ => serializer.collect_str(self),
        }
    }
}

#[cfg(feature = "demo-fixture")]
impl<'de> serde::Deserialize<'de> for DraftId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = DraftId;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a draft number or <origin>-<sequence>")
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<DraftId, E> {
                Ok(DraftId::new(value))
            }
            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<DraftId, E> {
                DraftId::parse(text).map_err(|_| E::custom("invalid draft ID"))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod id_tests {
    use super::DraftId;

    #[test]
    fn draft_ids_carry_their_origin_and_read_back_from_text() {
        let a = DraftId::scoped("owner:dev-00000000000000aa", 1).unwrap();
        let b = DraftId::scoped("owner:dev-00000000000000bb", 1).unwrap();
        assert_ne!(a, b);
        assert_ne!(a, DraftId::new(1));
        assert_eq!(a.value(), 1);
        assert_eq!(a.to_string(), "owner:dev-00000000000000aa-1");
        assert_eq!(DraftId::parse(&a.to_string()), Ok(a));
        assert_eq!(DraftId::parse("7"), Ok(DraftId::new(7)));
        assert_eq!(DraftId::new(7).to_string(), "7");
        assert_eq!(a.with_value(9).to_string(), "owner:dev-00000000000000aa-9");
        for bad in [
            "",
            "-1",
            "07",
            "owner:dev-x-01",
            "a-1",
            "owner:dev-x-",
            "owner:dev-x-+1",
            ":-1",
            "owner:-1",
            "a:b:c-1",
            "owner:dev x-1",
            "18446744073709551616",
        ] {
            assert!(DraftId::parse(bad).is_err(), "{bad}");
        }
        let longest = format!("{}:{}", "a".repeat(22), "dev-0123456789abcdef");
        let id = DraftId::scoped(&longest, u64::MAX).unwrap();
        assert!(id.to_string().len() <= 64);
        assert!(DraftId::scoped(&format!("{longest}x"), 1).is_err());
        assert!(DraftId::new(1) < a);
    }
}

#[cfg(test)]
mod quantity_tests {
    use super::*;

    #[test]
    fn replaces_only_the_selected_ordinary_line_quantity() {
        // Given an ordinary draft with two lines and independent customer, date, and crate data.
        let one = NonZeroU32::MIN;
        let two = NonZeroU32::new(2).unwrap();
        let first = DraftLine::new(ProductId::new(1), two);
        let second = DraftLine::new(ProductId::new(2), one);
        let mut draft = ProvisionalDraft::new(DraftId::new(7), vec![first, second])
            .with_bill_date(Some(BillDate::new(2025, 2, 12).unwrap()))
            .with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new("customer-1".into()),
                name: "Synthetic Customer".into(),
            });
        draft.add_crate_row(CrateRow::new(one, vec![]));
        let mut expected = draft.clone();
        expected.lines[1] = DraftLine::new(ProductId::new(2), two);

        // When the second ordinary line is edited.
        let previous = draft.replace_line_quantity(NonZeroUsize::new(2).unwrap(), two);

        // Then only its quantity changes.
        assert_eq!(previous, Some(one));
        assert_eq!(draft, expected);
    }
}

/// Identity of a product referenced by a draft line.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct ProductId(u64);

impl ProductId {
    /// Wraps a synthetic product identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[doc(hidden)]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

/// A requested quantity, not a stock reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct DraftLine {
    product_id: ProductId,
    quantity: NonZeroU32,
}

impl DraftLine {
    /// Builds a line with a positive whole-unit quantity.
    pub const fn new(product_id: ProductId, quantity: NonZeroU32) -> Self {
        Self {
            product_id,
            quantity,
        }
    }

    /// Returns the requested product.
    pub const fn product_id(self) -> ProductId {
        self.product_id
    }

    /// Returns the requested quantity.
    pub const fn quantity(self) -> NonZeroU32 {
        self.quantity
    }
}

/// Case corresponds to the legacy Packet unit, not a multiple of the pack size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub enum SaleUnit {
    Piece,
    Case,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct CrateComponent {
    pub product: ProductId,
    pub quantity: NonZeroU32,
    pub unit: SaleUnit,
}

impl CrateComponent {
    pub const fn new(product: ProductId, quantity: NonZeroU32, unit: SaleUnit) -> Self {
        Self {
            product,
            quantity,
            unit,
        }
    }
}

/// Editable crate row. Order and repeated products are preserved until review.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct CrateRow {
    quantity: NonZeroU32,
    components: Vec<CrateComponent>,
    price: Option<CrateRowPrice>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
struct CrateRowPrice {
    name: String,
    rate: Money,
}

impl CrateRow {
    pub const fn new(quantity: NonZeroU32, components: Vec<CrateComponent>) -> Self {
        Self {
            quantity,
            components,
            price: None,
        }
    }

    /// The crate's copied name and rate per crate (`li.name`, `li.rate`).
    #[must_use]
    pub fn with_price(mut self, name: String, rate: Money) -> Self {
        self.price = Some(CrateRowPrice { name, rate });
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.price.as_ref().map(|price| price.name.as_str())
    }

    pub fn rate(&self) -> Option<Money> {
        self.price.as_ref().map(|price| price.rate)
    }

    pub const fn quantity(&self) -> NonZeroU32 {
        self.quantity
    }

    pub fn components(&self) -> &[CrateComponent] {
        &self.components
    }

    /// The same crate line (contents, name and rate) at another number of crates.
    #[must_use]
    pub fn with_quantity(&self, quantity: NonZeroU32) -> Self {
        Self {
            quantity,
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct DraftCustomerId(String);

impl DraftCustomerId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

/// Customer input owned by a provisional draft.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub enum DraftCustomer {
    Unassigned,
    Typed(String),
    Selected { id: DraftCustomerId, name: String },
}

/// Editable local draft with no payment, stock, or final invoice state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvisionalDraft {
    id: DraftId,
    lines: Vec<DraftLine>,
    /// Crate rows beside the ordinary lines (`draft.items` entries with `isCrate`), in the order
    /// added; a bill lists its ordinary lines first, then its crate rows.
    #[cfg_attr(
        feature = "demo-fixture",
        serde(alias = "crate_row", default, deserialize_with = "crate_rows_or_one")
    )]
    crate_rows: Vec<CrateRow>,
    customer: DraftCustomer,
    bill_date: Option<BillDate>,
    /// "Loading/Delivery ₹ per packet" (`draft.loadingRate`), charged per billed packet.
    loading_rate: Money,
}

/// Journals written before mixed bills hold one optional crate row.
#[cfg(feature = "demo-fixture")]
fn crate_rows_or_one<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CrateRow>, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Many(Vec<CrateRow>),
        One(Option<CrateRow>),
    }
    Ok(
        match <Stored as serde::Deserialize>::deserialize(deserializer)? {
            Stored::Many(rows) => rows,
            Stored::One(row) => row.into_iter().collect(),
        },
    )
}

impl ProvisionalDraft {
    /// Takes ownership of the draft lines, which may be empty while editing.
    pub const fn new(id: DraftId, lines: Vec<DraftLine>) -> Self {
        Self {
            id,
            lines,
            crate_rows: Vec::new(),
            customer: DraftCustomer::Unassigned,
            bill_date: None,
            loading_rate: Money::from_minor_units(0),
        }
    }

    /// Sets the per-packet loading charge (`newDraft` starts it at the business default).
    pub const fn with_loading_rate(mut self, rate: Money) -> Self {
        self.loading_rate = rate;
        self
    }

    pub const fn loading_rate(&self) -> Money {
        self.loading_rate
    }

    pub fn with_customer(mut self, customer: DraftCustomer) -> Self {
        self.customer = customer;
        self
    }

    /// Sets or clears the explicit bill date while editing.
    pub const fn with_bill_date(mut self, bill_date: Option<BillDate>) -> Self {
        self.bill_date = bill_date;
        self
    }

    pub const fn bill_date(&self) -> Option<BillDate> {
        self.bill_date
    }

    /// Adds a crate row after the existing ones (`draft.items.push` of a crate).
    pub fn add_crate_row(&mut self, row: CrateRow) {
        self.crate_rows.push(row);
    }

    /// Replaces the crate row at `index`, returning the previous row. A missing row is unchanged.
    pub fn replace_crate_row(&mut self, index: usize, row: CrateRow) -> Option<CrateRow> {
        self.crate_rows
            .get_mut(index)
            .map(|existing| std::mem::replace(existing, row))
    }

    /// Removes the crate row at `index`, returning it.
    pub fn remove_crate_row(&mut self, index: usize) -> Option<CrateRow> {
        (index < self.crate_rows.len()).then(|| self.crate_rows.remove(index))
    }

    pub fn crate_rows(&self) -> &[CrateRow] {
        &self.crate_rows
    }

    pub const fn customer(&self) -> &DraftCustomer {
        &self.customer
    }

    /// Returns the draft identity.
    pub const fn id(&self) -> DraftId {
        self.id
    }

    /// Returns draft lines without owner-only data.
    pub fn lines(&self) -> &[DraftLine] {
        &self.lines
    }

    pub fn add_line(&mut self, line: DraftLine) {
        self.lines.push(line);
    }

    pub fn remove_line(&mut self, line_number: NonZeroUsize) -> Option<DraftLine> {
        let index = line_number.get() - 1;
        (index < self.lines.len()).then(|| self.lines.remove(index))
    }

    /// Replaces the quantity of an ordinary line by its one-based position.
    /// Returns the previous quantity, or `None` if the line does not exist.
    pub fn replace_line_quantity(
        &mut self,
        line_number: NonZeroUsize,
        quantity: NonZeroU32,
    ) -> Option<NonZeroU32> {
        let line = self.lines.get_mut(line_number.get() - 1)?;
        Some(std::mem::replace(&mut line.quantity, quantity))
    }
}
