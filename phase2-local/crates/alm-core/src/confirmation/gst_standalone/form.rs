//! GST Sales screen state: the list projection and the New/Edit form as typed text, with the
//! HTML's parse and default rules (`saveGstBill`, `gstRow`, `fillGstItem`) applied in core.
use std::cmp::Reverse;

use super::{
    GstBill, GstBillDraft, GstBillError, GstBillId, GstBillLineInput, GstBillNumber, GstUnit, owner,
};
use crate::confirmation::ConfirmationFixture;
use crate::{BillDate, Money, ProductId, Role};

/// One row of the GST Sales list (`VIEWS.gstsales`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstBillListRow {
    pub id: GstBillId,
    pub number: String,
    pub date: BillDate,
    pub customer: String,
    pub items: usize,
    /// Sum of the line amounts (`incl`), before any invoice rounding.
    pub total: Money,
    pub cancelled: bool,
}

/// A form line as typed: blank quantity and rate read as 0, blank GST % as the item default.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GstLineFields {
    pub item: String,
    pub product: Option<ProductId>,
    pub hsn: String,
    pub unit: String,
    pub quantity: String,
    pub rate: String,
    pub gst: String,
    pub packet_qty: String,
}

/// The form header as typed; blank texts take `saveGstBill`'s defaults when saved.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GstBillFields {
    pub id: GstBillId,
    pub number: GstBillNumber,
    pub created: u64,
    pub customer: String,
    pub date: String,
    pub gstin: String,
    pub cust_addr: String,
    pub cust_phone: String,
    pub cust_email: String,
    pub cust_state: String,
    pub cust_state_code: String,
    pub pos_state: String,
    pub pos_code: String,
    pub pay_mode: String,
    pub reverse_charge: String,
    pub eway_number: String,
    pub transporter: String,
    pub vehicle_no: String,
    pub lr_no: String,
    pub terms_delivery: String,
    pub lines: Vec<GstLineFields>,
}

/// Percent text with up to two decimals ("18", "2.5") as basis points.
pub fn parse_gst_percent(text: &str) -> Result<u32, GstBillError> {
    let paise = Money::parse_rupees(text).map_err(|_| GstBillError::InvalidAmount)?;
    u32::try_from(paise.minor_units()).map_err(|_| GstBillError::InvalidAmount)
}

/// Basis points as the shortest percent text (`1800` → "18", `250` → "2.5").
pub fn gst_percent_text(basis_points: u32) -> String {
    let (whole, part) = (basis_points / 100, basis_points % 100);
    match part {
        0 => whole.to_string(),
        p if p % 10 == 0 => format!("{whole}.{}", p / 10),
        p => format!("{whole}.{p:02}"),
    }
}

fn whole(text: &str, blank: u32) -> Result<u32, GstBillError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(blank);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(GstBillError::InvalidAmount);
    }
    text.parse().map_err(|_| GstBillError::InvalidAmount)
}

impl GstLineFields {
    pub fn blank() -> Self {
        Self {
            unit: "Case".into(),
            quantity: "1".into(),
            packet_qty: "1".into(),
            ..Self::default()
        }
    }

    fn from_saved(line: &super::GstBillLine) -> Self {
        Self {
            item: line.item.clone(),
            product: line.product,
            hsn: line.hsn.clone(),
            unit: match line.unit {
                GstUnit::Case => "Case".into(),
                GstUnit::Piece => "Piece".into(),
            },
            quantity: line.quantity.to_string(),
            rate: line.inclusive_rate.to_rupee_text(),
            gst: gst_percent_text(line.gst_basis_points),
            packet_qty: line.packet_qty.to_string(),
        }
    }
}

impl GstBillFields {
    pub fn from_saved(bill: &GstBill) -> Self {
        Self {
            id: bill.id,
            number: bill.number,
            created: bill.created,
            customer: bill.customer.clone(),
            date: bill.date.to_string(),
            gstin: bill.gstin.clone().unwrap_or_default(),
            cust_addr: bill.cust_addr.clone(),
            cust_phone: bill.cust_phone.clone(),
            cust_email: bill.cust_email.clone(),
            cust_state: bill.cust_state.clone(),
            cust_state_code: bill.cust_state_code.clone(),
            pos_state: bill.pos_state.clone(),
            pos_code: bill.pos_code.clone(),
            pay_mode: bill.pay_mode.clone(),
            reverse_charge: bill.reverse_charge.clone(),
            eway_number: bill.eway_number.clone().unwrap_or_default(),
            transporter: bill.transporter.clone(),
            vehicle_no: bill.vehicle_no.clone(),
            lr_no: bill.lr_no.clone(),
            terms_delivery: bill.terms_delivery.clone(),
            lines: bill.lines.iter().map(GstLineFields::from_saved).collect(),
        }
    }
}

impl ConfirmationFixture {
    /// Bills newest `created` first; equal stamps keep ID order (a stable sort, as in the HTML).
    pub fn gst_bill_list(&self, role: Role) -> Result<Vec<GstBillListRow>, GstBillError> {
        owner(role)?;
        let mut rows = Vec::new();
        for bill in self.gst_bills.values() {
            let mut total = 0_i64;
            for line in &bill.lines {
                total = total
                    .checked_add(line.amount.minor_units())
                    .ok_or(GstBillError::Overflow)?;
            }
            rows.push((
                bill.created,
                GstBillListRow {
                    id: bill.id,
                    number: self.gst_invoice(bill)?.invoice_number(),
                    date: bill.date,
                    customer: bill.customer.clone(),
                    items: bill.lines.len(),
                    total: Money::from_minor_units(total),
                    cancelled: bill.cancelled,
                },
            ));
        }
        rows.sort_by_key(|(created, _)| Reverse(*created));
        Ok(rows.into_iter().map(|(_, row)| row).collect())
    }

    /// A fresh record ID above every stored bill and deletion marker.
    pub fn next_gst_bill_id(&self, role: Role) -> Result<GstBillId, GstBillError> {
        owner(role)?;
        self.gst_bills
            .keys()
            .chain(&self.gst_deleted)
            .map(|id| id.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .map(GstBillId)
            .ok_or(GstBillError::Overflow)
    }

    /// `newGstDraft` as form text: next number, `today`, the state/payment defaults, one line.
    pub fn new_gst_bill_fields(
        &self,
        role: Role,
        today: BillDate,
    ) -> Result<GstBillFields, GstBillError> {
        let id = self.next_gst_bill_id(role)?;
        let draft = self.new_gst_bill_draft(role, id, today)?;
        Ok(GstBillFields {
            id,
            number: draft.number,
            date: today.to_string(),
            cust_state: draft.cust_state.into(),
            cust_state_code: draft.cust_state_code.into(),
            pos_state: draft.pos_state.into(),
            pos_code: draft.pos_code.into(),
            pay_mode: draft.pay_mode.into(),
            reverse_charge: draft.reverse_charge.into(),
            lines: vec![GstLineFields::blank()],
            ..GstBillFields::default()
        })
    }

    /// Opens a saved, active bill as form text.
    pub fn edit_gst_bill_fields(
        &self,
        role: Role,
        id: GstBillId,
    ) -> Result<GstBillFields, GstBillError> {
        owner(role)?;
        let bill = self.gst_bills.get(&id).ok_or(GstBillError::NotFound)?;
        if bill.cancelled {
            return Err(GstBillError::Cancelled);
        }
        Ok(GstBillFields::from_saved(bill))
    }

    /// `fillGstItem` for every named line: product, pack, HSN, GST %, and the rate when it is 0.
    pub fn fill_gst_lines(
        &self,
        role: Role,
        fields: &mut GstBillFields,
    ) -> Result<(), GstBillError> {
        for line in &mut fields.lines {
            if line.item.trim().is_empty() {
                continue;
            }
            let fill = self.fill_gst_item(role, &line.item, line.product)?;
            let current = self.gst_line_input(role, line)?;
            let filled = fill.apply(&line.item, current);
            let (product, hsn, pack, rate, gst) = (
                filled.product,
                filled.hsn.to_owned(),
                filled.packet_qty.to_string(),
                filled.inclusive_rate.to_rupee_text(),
                gst_percent_text(filled.gst_basis_points),
            );
            line.product = product;
            line.hsn = hsn;
            line.packet_qty = pack;
            line.rate = rate;
            line.gst = gst;
        }
        Ok(())
    }

    /// Reads one typed line: blank quantity/rate are 0, blank pack is 1, blank GST % is the
    /// item's default (`gstInfo(item).total`); `Piece` selects pieces, anything else cases.
    pub fn gst_line_input<'a>(
        &self,
        role: Role,
        line: &'a GstLineFields,
    ) -> Result<GstBillLineInput<'a>, GstBillError> {
        owner(role)?;
        let inclusive_rate = if line.rate.trim().is_empty() {
            Money::from_minor_units(0)
        } else {
            Money::parse_rupees(&line.rate).map_err(|_| GstBillError::InvalidAmount)?
        };
        let gst_basis_points = if line.gst.trim().is_empty() {
            self.catalog.gst_info(line.item.trim()).0
        } else {
            parse_gst_percent(&line.gst)?
        };
        Ok(GstBillLineInput {
            item: line.item.trim(),
            product: line.product,
            hsn: line.hsn.trim(),
            unit: if line.unit == "Piece" {
                GstUnit::Piece
            } else {
                GstUnit::Case
            },
            quantity: whole(&line.quantity, 0)?,
            inclusive_rate,
            gst_basis_points,
            packet_qty: whole(&line.packet_qty, 1)?,
        })
    }

    /// `gstLineAmt`: the typed line's inclusive rate times its quantity.
    pub fn gst_line_amount(&self, role: Role, line: &GstLineFields) -> Result<Money, GstBillError> {
        let input = self.gst_line_input(role, line)?;
        super::amount(input.inclusive_rate, input.quantity).map(Money::from_minor_units)
    }

    /// Builds the core draft from form text; `now` stamps `created` on a first save.
    pub fn gst_bill_draft<'a>(
        &self,
        role: Role,
        fields: &'a GstBillFields,
        lines: &'a [GstBillLineInput<'a>],
        now: u64,
    ) -> Result<GstBillDraft<'a>, GstBillError> {
        owner(role)?;
        let optional = |text: &'a str| (!text.trim().is_empty()).then(|| text.trim());
        Ok(GstBillDraft {
            id: fields.id,
            number: fields.number,
            customer_id: "",
            customer: fields.customer.trim(),
            date: fields.date.parse().ok(),
            gstin: optional(&fields.gstin),
            cust_addr: fields.cust_addr.trim(),
            cust_phone: fields.cust_phone.trim(),
            cust_email: fields.cust_email.trim(),
            cust_state: fields.cust_state.trim(),
            cust_state_code: fields.cust_state_code.trim(),
            pos_state: fields.pos_state.trim(),
            pos_code: fields.pos_code.trim(),
            pay_mode: fields.pay_mode.trim(),
            pay_terms: "",
            reverse_charge: fields.reverse_charge.trim(),
            transporter: fields.transporter.trim(),
            vehicle_no: fields.vehicle_no.trim(),
            lr_no: fields.lr_no.trim(),
            eway_number: optional(&fields.eway_number),
            terms_delivery: fields.terms_delivery.trim(),
            dispatch: "",
            created: fields.created,
            now,
            lines,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GstBillFields, GstLineFields, gst_percent_text, parse_gst_percent};
    use crate::confirmation::demo_fixture::fixture;
    use crate::confirmation::{ConfirmationFixture, GstBillError, GstBillId, GstBillNumber};
    use crate::{BillDate, Role};

    fn day() -> BillDate {
        BillDate::new(2025, 4, 30).unwrap()
    }

    fn save(fixture: &mut ConfirmationFixture, fields: &GstBillFields, now: u64) {
        let lines: Vec<_> = fields
            .lines
            .iter()
            .map(|line| fixture.gst_line_input(Role::Owner, line).unwrap())
            .collect();
        let draft = fixture
            .gst_bill_draft(Role::Owner, fields, &lines, now)
            .unwrap();
        fixture
            .save_gst_bill(Role::Owner, &draft, &mut |_| {})
            .unwrap();
    }

    fn bill(fixture: &ConfirmationFixture, customer: &str) -> GstBillFields {
        let mut fields = fixture.new_gst_bill_fields(Role::Owner, day()).unwrap();
        fields.customer = customer.into();
        fields.lines = vec![GstLineFields {
            item: "Synthetic Soda".into(),
            quantity: "2".into(),
            rate: "".into(),
            ..GstLineFields::blank()
        }];
        fixture.fill_gst_lines(Role::Owner, &mut fields).unwrap();
        fields
    }

    #[test]
    fn percent_text_round_trips_basis_points() {
        for (text, bp) in [("18", 1_800), ("2.5", 250), ("0", 0), ("0.25", 25)] {
            assert_eq!(parse_gst_percent(text), Ok(bp));
            assert_eq!(gst_percent_text(bp), text);
        }
        assert_eq!(parse_gst_percent("-5"), Err(GstBillError::InvalidAmount));
    }

    #[test]
    fn list_numbers_cancel_and_delete_without_ordinary_effects() {
        let mut fixture = fixture().unwrap();
        let ordinary = fixture.clone();
        let first = bill(&fixture, "Synthetic Buyer A");
        assert_eq!((first.id, first.number), (GstBillId(1), GstBillNumber(1)));
        assert_eq!(first.lines[0].rate, "62.50");
        assert_eq!(first.lines[0].gst, "18");
        assert_eq!(
            fixture.gst_line_amount(Role::Owner, &first.lines[0]),
            Ok(crate::Money::from_minor_units(12_500))
        );
        save(&mut fixture, &first, 10);
        let second = bill(&fixture, "Synthetic Buyer B");
        assert_eq!(second.number, GstBillNumber(2));
        save(&mut fixture, &second, 20);
        let rows = fixture.gst_bill_list(Role::Owner).unwrap();
        let shown: Vec<_> = rows
            .iter()
            .map(|row| (row.number.as_str(), row.total.minor_units(), row.cancelled))
            .collect();
        assert_eq!(
            shown,
            [
                ("ALM/25-26/2", 12_500, false),
                ("ALM/25-26/1", 12_500, false)
            ]
        );
        fixture.cancel_gst_bill(Role::Owner, GstBillId(2)).unwrap();
        assert_eq!(
            fixture.edit_gst_bill_fields(Role::Owner, GstBillId(2)),
            Err(GstBillError::Cancelled)
        );
        assert_eq!(
            fixture.next_gst_bill_number(Role::Owner),
            Ok(GstBillNumber(3))
        );
        fixture.delete_gst_bill(Role::Owner, GstBillId(2)).unwrap();
        let ledger = fixture.gst_bill_ledger(Role::Owner).unwrap();
        assert_eq!(ledger.deleted, [GstBillId(2)]);
        assert_eq!(fixture.next_gst_bill_id(Role::Owner), Ok(GstBillId(3)));
        let mut edit = fixture
            .edit_gst_bill_fields(Role::Owner, GstBillId(1))
            .unwrap();
        edit.lines[0].quantity = "4".into();
        save(&mut fixture, &edit, 99);
        let rows = fixture.gst_bill_list(Role::Owner).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].total.minor_units(), 25_000);
        assert_eq!(fixture.without_gst_bills(), ordinary);
        assert_eq!(
            fixture.gst_bill_list(Role::Manager),
            Err(GstBillError::AccessDenied)
        );
        assert_eq!(
            fixture.new_gst_bill_fields(Role::Manager, day()),
            Err(GstBillError::AccessDenied)
        );
    }
}
