use std::fmt;
use std::io::{self, Write};

use alm_phase2_local::{DraftCustomer, Money, SaleUnit, provisional_preview};

use super::CurrentDraft;

struct Rupees(Money);

impl fmt::Display for Rupees {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let minor_units = self.0.minor_units();
        let sign = if minor_units < 0 { "-" } else { "" };
        let magnitude = minor_units.unsigned_abs();
        write!(
            formatter,
            "{sign}₹{}.{:02}",
            magnitude / 100,
            magnitude % 100
        )
    }
}

pub(super) fn render(output: &mut impl Write, current: &CurrentDraft) -> io::Result<()> {
    writeln!(output, "PROVISIONAL — NOT POSTED")?;
    writeln!(output, "Draft ID: {}", current.id)?;
    let state = if current.saved { "SAVED" } else { "UNSAVED" };
    writeln!(output, "State: {state}")?;
    let customer = match current.draft.customer() {
        DraftCustomer::Unassigned => "Unassigned",
        DraftCustomer::Typed(name) | DraftCustomer::Selected { name, .. } => name,
    };
    writeln!(output, "Customer: {customer}")?;

    let preview = match provisional_preview::preview(&current.draft) {
        Ok(preview) => preview,
        Err(error) => {
            for (index, line) in current.draft.lines().iter().enumerate() {
                writeln!(
                    output,
                    "Line {}: Product ID {} | Quantity: {}",
                    index + 1,
                    line.product_id(),
                    line.quantity()
                )?;
            }
            if let Some(row) = current.draft.crate_row() {
                writeln!(output, "Crate quantity: {}", row.quantity())?;
                for component in row.components() {
                    let unit = match component.unit {
                        SaleUnit::Piece => "Piece",
                        SaleUnit::Case => "Case",
                    };
                    writeln!(
                        output,
                        "Crate component: Product ID {} | Quantity per crate: {} | Unit: {unit}",
                        component.product, component.quantity
                    )?;
                }
            }
            return writeln!(output, "Bill preview unavailable: {error:?}");
        }
    };
    for line in preview.lines {
        writeln!(output, "Item: {} | Quantity: {}", line.name, line.quantity)?;
        writeln!(
            output,
            "Unit price (inclusive): {} | Line total (inclusive): {}",
            Rupees(line.inclusive_unit_price),
            Rupees(line.amounts.inclusive),
        )?;
    }
    writeln!(output, "Taxable: {}", Rupees(preview.totals.taxable))?;
    writeln!(output, "CGST: {}", Rupees(preview.totals.cgst))?;
    writeln!(output, "SGST: {}", Rupees(preview.totals.sgst))?;
    writeln!(
        output,
        "Inclusive total: {}",
        Rupees(preview.totals.inclusive)
    )
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use alm_phase2_local::{DraftId, DraftLine, Money, ProductId, ProvisionalDraft};

    use super::{Rupees, render};
    use crate::CurrentDraft;

    #[test]
    fn formats_minor_units_without_floating_point_rounding() {
        assert_eq!(Rupees(Money::from_minor_units(5)).to_string(), "₹0.05");
        assert_eq!(
            Rupees(Money::from_minor_units(-1001)).to_string(),
            "-₹10.01"
        );
        assert_eq!(
            Rupees(Money::from_minor_units(i64::MIN)).to_string(),
            "-₹92233720368547758.08"
        );
    }

    #[test]
    fn unsupported_product_keeps_draft_visible_without_partial_totals() {
        // Given a saved draft containing a product outside the preview catalog.
        let draft = ProvisionalDraft::new(
            DraftId::new(1),
            vec![DraftLine::new(ProductId::new(2), NonZeroU32::MIN)],
        );
        let current = CurrentDraft {
            id: NonZeroU64::MIN,
            draft,
            saved: true,
        };
        let mut output = Vec::new();

        // When the draft is reviewed.
        render(&mut output, &current).expect("render unavailable preview");

        // Then the draft remains visible, but no partial amounts are presented.
        let output = String::from_utf8(output).expect("UTF-8 output");
        assert!(output.contains("PROVISIONAL — NOT POSTED\n"));
        assert!(output.contains("State: SAVED\n"));
        assert!(output.contains("Line 1: Product ID 2 | Quantity: 1\n"));
        assert!(output.contains("Bill preview unavailable: UnknownProduct"));
        assert!(!output.contains("Inclusive total:"));
        assert!(!output.contains("Unit price"));
    }
}
