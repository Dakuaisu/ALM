//! Bill image layout (legacy `billImageCanvas`, HTML L4490–4760) as a display list. Core decides
//! every size, position, text, colour and value; the caller only measures text with its font
//! (for the address wrap and the mobile after the name) and rasterizes the operations.
use super::estimate::display_date;
use super::estimate::{EstimateDocument, estimate_document};
use super::image_handoff::format_amount;
use super::{BalanceMode, ConfirmationFixture, EstimateError, EstimateRequest, Money};
use crate::synthetic_read_model::BusinessProfile;

pub const BILL_IMAGE_WIDTH: i32 = 760;
pub const BILL_IMAGE_SCALE: u32 = 2;
const PAD: i32 = 30;
const ROW_H: i32 = 25;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontStyle {
    Regular,
    SemiBold,
    Bold,
    Italic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// One canvas operation in CSS pixels (before the ×2 scale); colours are 0xRRGGBB.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawOp {
    FillRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: u32,
    },
    /// `width` in tenths of a pixel; `dash` is `setLineDash` in pixels.
    StrokeRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: u32,
        width: u32,
        dash: Option<(u32, u32)>,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: u32,
        width: u32,
    },
    /// Alphabetic baseline at `y`.
    Text {
        x: i32,
        y: i32,
        size: u32,
        style: FontStyle,
        align: TextAlign,
        color: u32,
        text: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillImage {
    pub width: i32,
    pub height: i32,
    pub scale: u32,
    pub ops: Vec<DrawOp>,
}

impl BillImage {
    /// Output pixel size (`cv.width`, `cv.height`).
    pub fn pixels(&self) -> (u32, u32) {
        let px = |v: i32| u32::try_from(v).unwrap_or(0) * self.scale;
        (px(self.width), px(self.height))
    }
}

/// One bill line's product details: the PACK text and what `mrpSavings` needs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BillImageLine {
    pub pack: String,
    pub packet_quantity: u32,
    /// Per-piece MRP; a line without one saves nothing.
    pub mrp: Option<Money>,
}

/// `mrpSavings(s)`: Σ max(0, qty × packetQty × MRP − amount) over lines with an MRP, rounded
/// to whole rupees (`Math.round`). Lines beyond `details` have no MRP.
pub fn mrp_savings(bill: &EstimateDocument<'_>, details: &[BillImageLine]) -> Money {
    let mut total: i128 = 0;
    for (row, detail) in bill.lines.iter().zip(details) {
        let Some(mrp) = detail.mrp.filter(|mrp| mrp.minor_units() > 0) else {
            continue;
        };
        let value = i128::from(row.quantity)
            * i128::from(detail.packet_quantity.max(1))
            * i128::from(mrp.minor_units());
        total += (value - i128::from(row.amount.minor_units())).max(0);
    }
    let paise = i64::try_from(total).unwrap_or(i64::MAX);
    Money::from_minor_units(
        Money::from_minor_units(paise)
            .rounded_rupees()
            .saturating_mul(100),
    )
}

/// Shop header text (`DB.meta`) and the customer's contact (`c.mobile`, `c.addr1/addr2`).
#[derive(Clone, Copy, Debug)]
pub struct BillImageParty<'a> {
    pub business: &'a str,
    pub tagline: &'a str,
    pub address: &'a str,
    pub mobile: &'a str,
    pub profile: Option<&'a BusinessProfile>,
    pub customer_mobile: &'a str,
    pub customer_address: &'a str,
    /// Per bill line, in bill order.
    pub lines: &'a [BillImageLine],
}

fn text(
    ops: &mut Vec<DrawOp>,
    at: (i32, i32),
    font: (u32, FontStyle, TextAlign),
    color: u32,
    value: impl Into<String>,
) {
    ops.push(DrawOp::Text {
        x: at.0,
        y: at.1,
        size: font.0,
        style: font.1,
        align: font.2,
        color,
        text: value.into(),
    });
}

fn wrap(
    value: &str,
    max: f32,
    size: u32,
    measure: &dyn Fn(&str, u32, FontStyle) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if measure(&candidate, size, FontStyle::Regular) > max && !current.is_empty() {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.truncate(2);
    lines
}

impl ConfirmationFixture {
    /// Lays out the bill image for an Owner-visible posted bill.
    ///
    /// # Errors
    /// Denies Managers and rejects missing, unsupported or mislabelled bills.
    pub fn bill_image(
        &self,
        request: EstimateRequest<'_>,
        party: &BillImageParty<'_>,
        measure: &dyn Fn(&str, u32, FontStyle) -> f32,
    ) -> Result<BillImage, EstimateError> {
        let preview = self.preview_estimate(request)?;
        Ok(bill_image_layout(
            &estimate_document(&preview, request.display),
            party,
            measure,
        ))
    }
}

/// Lays out the bill image for an estimate document, from the live fixture or a reopened
/// archive (`ReadModel::latest_estimate`).
pub fn bill_image_layout(
    bill: &EstimateDocument<'_>,
    party: &BillImageParty<'_>,
    measure: &dyn Fn(&str, u32, FontStyle) -> f32,
) -> BillImage {
    use FontStyle::{Bold, Italic, Regular, SemiBold};
    use TextAlign::{Center, Left, Right};
    {
        let w = BILL_IMAGE_WIDTH;
        let rupees = |m| format!("₹{}", format_amount(m));
        let use_live = bill.balance_mode == BalanceMode::Latest;
        let mut summary = vec![
            ("Loading/Delivery".to_owned(), rupees(bill.loading)),
            ("Bill Amount".to_owned(), rupees(bill.total)),
        ];
        if !use_live && let Some(previous) = bill.previous_balance.filter(|p| !p.amount.is_zero()) {
            let dated = previous
                .date
                .map_or_else(String::new, |d| format!(" (as on {})", display_date(d)));
            summary.push((format!("Prev Balance{dated}"), rupees(previous.amount)));
        }
        summary.push((
            if use_live {
                "Paid on this bill"
            } else {
                "Paid"
            }
            .to_owned(),
            rupees(bill.paid),
        ));
        let due_label = if use_live {
            format!("Total Due (as on {})", display_date(bill.today))
        } else {
            "Balance Due".to_owned()
        };
        summary.push((due_label.clone(), rupees(bill.balance)));

        let company = wrap(party.address, 290.0, 11, measure);
        let customer = wrap(party.customer_address, 470.0, 12, measure);
        let (divider, bill_y, name_y, addr_h) = (124, 144, 162, 16);
        let head_y = name_y + i32::try_from(customer.len()).unwrap_or(0) * addr_h + 24;
        let rows_y = head_y + 22;
        let items_bottom = rows_y + i32::try_from(bill.lines.len()).unwrap_or(0) * ROW_H;
        let total_y = items_bottom + 6;
        let sum_top = total_y + 30;
        let sum_bottom = sum_top + i32::try_from(summary.len()).unwrap_or(0) * 23;
        let save = mrp_savings(bill, party.lines);
        let save_y = sum_bottom + 8;
        let foot_y = sum_bottom + 22 + if save.minor_units() > 0 { 44 } else { 0 };
        let height = foot_y + 44;

        let mut ops = vec![DrawOp::FillRect {
            x: 0,
            y: 0,
            w,
            h: height,
            color: 0xffffff,
        }];
        text(
            &mut ops,
            (PAD, 40),
            (24, Bold, Left),
            0x1d4ed8,
            party.business,
        );
        text(
            &mut ops,
            (PAD, 58),
            (12, Regular, Left),
            0x000000,
            party.tagline,
        );
        let mut hy = 74;
        for line in company {
            text(&mut ops, (PAD, hy), (11, Regular, Left), 0x000000, line);
            hy += 14;
        }
        for line in party
            .profile
            .map(BusinessProfile::estimate_lines)
            .unwrap_or_default()
        {
            text(&mut ops, (PAD, hy), (11, Regular, Left), 0x000000, line);
            hy += 14;
        }
        text(
            &mut ops,
            (PAD, hy),
            (11, Bold, Left),
            0x000000,
            format!("Mob: {}", party.mobile),
        );
        text(
            &mut ops,
            (w - PAD, 40),
            (17, Bold, Right),
            0x1d4ed8,
            "ESTIMATE",
        );
        text(
            &mut ops,
            (w - PAD, 62),
            (12, Regular, Right),
            0x000000,
            format!("No: {}", bill.invoice),
        );
        text(
            &mut ops,
            (w - PAD, 78),
            (12, Regular, Right),
            0x000000,
            format!("Date: {}", display_date(bill.date)),
        );
        ops.push(DrawOp::Line {
            x1: PAD,
            y1: divider,
            x2: w - PAD,
            y2: divider,
            color: 0x2563eb,
            width: 20,
        });
        text(
            &mut ops,
            (PAD, bill_y),
            (11, Regular, Left),
            0x000000,
            "BILL TO",
        );
        text(
            &mut ops,
            (PAD, name_y),
            (14, Bold, Left),
            0x000000,
            bill.customer,
        );
        if !party.customer_mobile.is_empty() {
            let after = measure(bill.customer, 14, Bold);
            #[allow(clippy::cast_possible_truncation)]
            let x = PAD + after.round() as i32;
            text(
                &mut ops,
                (x, name_y),
                (13, Regular, Left),
                0x000000,
                format!("   {}", party.customer_mobile),
            );
        }
        for (i, line) in customer.into_iter().enumerate() {
            let y = name_y + (i32::try_from(i).unwrap_or(0) + 1) * addr_h;
            text(&mut ops, (PAD, y), (12, Regular, Left), 0x000000, line);
        }
        let (c_rate, c_qty, c_amount, c_item, c_pack) = (520, 605, w - PAD, PAD + 24, 420);
        ops.push(DrawOp::FillRect {
            x: PAD,
            y: head_y - 16,
            w: w - 2 * PAD,
            h: 24,
            color: 0xeef2fb,
        });
        for (x, label, align) in [
            (PAD + 4, "#", Left),
            (c_item, "ITEM", Left),
            (c_pack, "PACK", Center),
            (c_rate, "RATE", Right),
            (c_qty, "QTY", Right),
            (c_amount, "AMOUNT", Right),
        ] {
            text(
                &mut ops,
                (x, head_y),
                (11, SemiBold, align),
                0x000000,
                label,
            );
        }
        let mut ty = rows_y;
        for (i, line) in bill.lines.iter().enumerate() {
            let name: String = line.label.chars().take(40).collect();
            text(
                &mut ops,
                (PAD + 4, ty),
                (13, Regular, Left),
                0x000000,
                (i + 1).to_string(),
            );
            text(&mut ops, (c_item, ty), (13, Regular, Left), 0x000000, name);
            let pack = party.lines.get(i).map_or("", |line| line.pack.as_str());
            text(
                &mut ops,
                (c_pack, ty),
                (11, Regular, Center),
                0x000000,
                pack,
            );
            text(
                &mut ops,
                (c_rate, ty),
                (13, Regular, Right),
                0x000000,
                format_amount(line.price),
            );
            text(
                &mut ops,
                (c_qty, ty),
                (13, Regular, Right),
                0x000000,
                line.quantity.to_string(),
            );
            text(
                &mut ops,
                (c_amount, ty),
                (13, Regular, Right),
                0x000000,
                rupees(line.amount),
            );
            ops.push(DrawOp::Line {
                x1: PAD,
                y1: ty + 8,
                x2: w - PAD,
                y2: ty + 8,
                color: 0xe2e6ea,
                width: 10,
            });
            ty += ROW_H;
        }
        ops.push(DrawOp::FillRect {
            x: PAD,
            y: total_y - 16,
            w: w - 2 * PAD,
            h: 25,
            color: 0xeef2fb,
        });
        text(
            &mut ops,
            (c_item, total_y),
            (13, Bold, Left),
            0x000000,
            "Items Total",
        );
        text(
            &mut ops,
            (c_qty, total_y),
            (13, Bold, Right),
            0x000000,
            bill.items_quantity.to_string(),
        );
        text(
            &mut ops,
            (c_amount, total_y),
            (13, Bold, Right),
            0x000000,
            rupees(bill.items_total),
        );
        let lx = w - 320;
        let mut sy = sum_top;
        let due_color = if bill.balance.minor_units() > 0 {
            0xb91c1c
        } else {
            0x15803d
        };
        for (label, value) in summary {
            let due = label == due_label;
            if due {
                ops.push(DrawOp::FillRect {
                    x: lx - 10,
                    y: sy - 15,
                    w: 320,
                    h: 24,
                    color: 0xeef2fb,
                });
            }
            let (style, color) = if due {
                (Bold, due_color)
            } else {
                (Regular, 0x000000)
            };
            text(&mut ops, (lx, sy), (13, style, Left), color, label);
            text(&mut ops, (w - PAD, sy), (13, style, Right), color, value);
            sy += 23;
        }
        if save.minor_units() > 0 {
            ops.push(DrawOp::FillRect {
                x: PAD,
                y: save_y,
                w: w - 2 * PAD,
                h: 32,
                color: 0xf0fdf4,
            });
            ops.push(DrawOp::StrokeRect {
                x: PAD,
                y: save_y,
                w: w - 2 * PAD,
                h: 32,
                color: 0x15803d,
                width: 15,
                dash: Some((5, 3)),
            });
            text(
                &mut ops,
                (w / 2, save_y + 22),
                (16, Bold, Center),
                0x15803d,
                format!("You saved ₹{} on MRP!", format_amount(save)),
            );
        }
        text(
            &mut ops,
            (PAD, foot_y),
            (11, Italic, Left),
            0x000000,
            "Go paperless, save trees & reduce carbon emission",
        );
        text(
            &mut ops,
            (PAD, foot_y + 15),
            (11, Regular, Left),
            0x000000,
            "E& OE",
        );
        text(
            &mut ops,
            (w - PAD, foot_y),
            (11, Regular, Right),
            0x000000,
            format!("For {}", party.business),
        );
        text(
            &mut ops,
            (w - PAD, foot_y + 15),
            (11, Regular, Right),
            0x000000,
            "Proprietor",
        );
        BillImage {
            width: w,
            height,
            scale: BILL_IMAGE_SCALE,
            ops,
        }
    }
}

#[cfg(test)]
#[path = "bill_image_tests.rs"]
mod tests;
