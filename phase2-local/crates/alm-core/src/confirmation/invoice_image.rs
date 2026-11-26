//! The full-invoice bill image (`invoiceHTML(s, simple)` rendered by `htmlToPNG(html, 760)`,
//! HTML L4245–4310 and L4718–4766) as a display list next to the canvas layout. The HTML shares
//! this image on WhatsApp (`waInvoice`: the simple variant) and keeps the canvas only as a
//! fallback. Core lays out the CSS boxes (padding, borders, flex rows, the auto-width items
//! table, right-aligned totals) with the caller's text measure; the caller rasterizes.
use super::bill_image::{
    BILL_IMAGE_SCALE, BILL_IMAGE_WIDTH, BillImage, BillImageParty, DrawOp, FontStyle, TextAlign,
    mrp_savings,
};
use super::estimate::{EstimateDocument, display_date};
use super::image_handoff::format_amount;
use super::{BalanceMode, ConfirmationFixture, EstimateError, EstimateRequest, Money};

impl ConfirmationFixture {
    /// Lays out the full-invoice bill image for an Owner-visible posted bill (`invoiceHTML`).
    ///
    /// # Errors
    /// Denies Managers and rejects missing, unsupported or mislabelled bills.
    pub fn invoice_image(
        &self,
        request: EstimateRequest<'_>,
        party: &BillImageParty<'_>,
        loading_rate: Option<Money>,
        simple: bool,
        measure: Measure<'_>,
    ) -> Result<BillImage, EstimateError> {
        let preview = self.preview_estimate(request)?;
        Ok(invoice_image_layout(
            &super::estimate::estimate_document(&preview, request.display),
            party,
            loading_rate,
            simple,
            measure,
        ))
    }
}

type Measure<'a> = &'a dyn Fn(&str, u32, FontStyle) -> f32;

/// Noto Sans: ascender 1.069 em, descender 0.293 em, no line gap (`line-height: normal`).
const ASCENT: f32 = 1.069;
const LINE: f32 = 1.362;
const INK: u32 = 0x000000;
const BLUE: u32 = 0x1d4ed8;
const RULE: u32 = 0x2563eb;
const GRID: u32 = 0x8b97a8;
const BAND: u32 = 0xeef2fb;
const GREEN: u32 = 0x15803d;
const RED: u32 = 0xb91c1c;

fn line_height(size: u32) -> f32 {
    f32::from(u16::try_from(size).unwrap_or(u16::MAX)) * LINE
}

fn baseline(size: u32) -> f32 {
    f32::from(u16::try_from(size).unwrap_or(u16::MAX)) * ASCENT
}

#[allow(clippy::cast_possible_truncation)]
fn px(value: f32) -> i32 {
    value.round() as i32
}

/// A run of text in one font: `(text, size, style, colour)`.
type Run = (String, u32, FontStyle, u32);

struct Painter<'a> {
    ops: Vec<DrawOp>,
    measure: Measure<'a>,
}

impl Painter<'_> {
    fn width(&self, runs: &[Run]) -> f32 {
        runs.iter()
            .map(|(text, size, style, _)| (self.measure)(text, *size, *style))
            .sum()
    }

    /// Draws `runs` on one baseline, starting at `x`.
    fn runs(&mut self, x: f32, y: f32, runs: &[Run]) {
        let mut caret = x;
        for (text, size, style, color) in runs {
            if !text.is_empty() {
                self.ops.push(DrawOp::Text {
                    x: px(caret),
                    y: px(y),
                    size: *size,
                    style: *style,
                    align: TextAlign::Left,
                    color: *color,
                    text: text.clone(),
                });
            }
            caret += (self.measure)(text, *size, *style);
        }
    }

    /// Draws `runs` aligned within `[left, right]` (`text-align`).
    fn aligned(&mut self, (left, right): (f32, f32), y: f32, align: TextAlign, runs: &[Run]) {
        if let ([(text, size, style, color)], TextAlign::Center | TextAlign::Right) = (runs, align)
        {
            if !text.is_empty() {
                self.ops.push(DrawOp::Text {
                    x: px(if align == TextAlign::Right {
                        right
                    } else {
                        f32::midpoint(left, right)
                    }),
                    y: px(y),
                    size: *size,
                    style: *style,
                    align,
                    color: *color,
                    text: text.clone(),
                });
            }
            return;
        }
        let width = self.width(runs);
        let x = match align {
            TextAlign::Left => left,
            TextAlign::Center => left + (right - left - width) / 2.0,
            TextAlign::Right => right - width,
        };
        self.runs(x, y, runs);
    }

    fn fill(&mut self, (x, y, w, h): (f32, f32, f32, f32), color: u32) {
        self.ops.push(DrawOp::FillRect {
            x: px(x),
            y: px(y),
            w: px(x + w) - px(x),
            h: px(y + h) - px(y),
            color,
        });
    }

    fn stroke(&mut self, (x, y, w, h): (f32, f32, f32, f32), color: u32, width_tenths: u32) {
        self.ops.push(DrawOp::StrokeRect {
            x: px(x),
            y: px(y),
            w: px(x + w) - px(x),
            h: px(y + h) - px(y),
            color,
            width: width_tenths,
            dash: None,
        });
    }

    /// Word-wraps `text` to `max` pixels (CSS `overflow-wrap: break-word` keeps long words).
    fn wrap(&self, text: &str, max: f32, size: u32, style: FontStyle) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in text.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if (self.measure)(&candidate, size, style) > max && !current.is_empty() {
                lines.push(std::mem::replace(&mut current, word.to_owned()));
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        lines
    }
}

fn run(text: impl Into<String>, size: u32, style: FontStyle, color: u32) -> Run {
    (text.into(), size, style, color)
}

fn rupees(amount: Money) -> String {
    format!("₹{}", format_amount(amount))
}

/// One table cell: its lines (each a list of runs) and alignment.
struct Cell {
    lines: Vec<Vec<Run>>,
    align: TextAlign,
}

impl Cell {
    fn text(text: impl Into<String>, size: u32, style: FontStyle, align: TextAlign) -> Self {
        Self {
            lines: vec![vec![run(text, size, style, INK)]],
            align,
        }
    }
}

/// `invoiceHTML(s, simple)` laid out at 760 CSS pixels and drawn at ×2 (`htmlToPNG`).
/// `loading_rate` is the bill's per-packet rate (`s.loadingRate`), when known.
#[allow(clippy::too_many_lines)]
pub fn invoice_image_layout(
    bill: &EstimateDocument<'_>,
    party: &BillImageParty<'_>,
    loading_rate: Option<Money>,
    simple: bool,
    measure: Measure<'_>,
) -> BillImage {
    use FontStyle::{Bold, Italic, Regular};
    use TextAlign::{Center, Left, Right};
    let mut p = Painter {
        ops: Vec::new(),
        measure,
    };
    let width = f32::from(u16::try_from(BILL_IMAGE_WIDTH).unwrap_or(760));
    // `.invoice`: max-width 720 centred, 1.5px border, padding 26px 30px.
    let (outer_left, outer_width) = ((width - 720.0) / 2.0, 720.0);
    let (left, right) = (
        outer_left + 1.5 + 30.0,
        outer_left + outer_width - 1.5 - 30.0,
    );
    let content = right - left;
    let mut y = 1.5 + 26.0;

    // Header: shop on the left, ESTIMATE / No / Date on the right; 3px rule under it.
    let top = y;
    let mut ly = top;
    p.runs(
        left,
        ly + baseline(26),
        &[run(party.business, 26, Bold, BLUE)],
    );
    ly += line_height(26) + 2.0;
    if !party.tagline.is_empty() {
        p.runs(
            left,
            ly + baseline(13),
            &[run(party.tagline, 13, Regular, INK)],
        );
        ly += line_height(13);
    }
    ly += 5.0;
    for line in p.wrap(party.address, 330.0, 12, Regular) {
        p.runs(left, ly + baseline(12), &[run(line, 12, Regular, INK)]);
        ly += line_height(12);
    }
    for line in party
        .profile
        .map(crate::synthetic_read_model::BusinessProfile::estimate_lines)
        .unwrap_or_default()
    {
        p.runs(left, ly + baseline(12), &[run(line, 12, Regular, INK)]);
        ly += line_height(12);
    }
    p.runs(
        left,
        ly + baseline(12),
        &[run(format!("Mob: {}", party.mobile), 12, Bold, INK)],
    );
    ly += line_height(12);

    let mut ry = top;
    // `letter-spacing: 1px` after every letter.
    let letters: Vec<String> = "ESTIMATE".chars().map(String::from).collect();
    let spaced: f32 = letters
        .iter()
        .map(|letter| measure(letter, 19, Bold) + 1.0)
        .sum();
    let mut caret = right - spaced;
    for letter in &letters {
        p.runs(
            caret,
            ry + baseline(19),
            &[run(letter.clone(), 19, Bold, BLUE)],
        );
        caret += measure(letter, 19, Bold) + 1.0;
    }
    ry += line_height(19);
    if bill.cancelled {
        let label = "CANCELLED";
        let (w, h) = (
            measure(label, 12, Bold) + 2.0 * 10.0 + 4.0,
            line_height(12) + 2.0 * 2.0 + 4.0,
        );
        let (bx, by) = (right - w, ry + 6.0);
        p.ops.push(DrawOp::RoundRect {
            x: px(bx + 1.0),
            y: px(by + 1.0),
            w: px(w - 2.0),
            h: px(h - 2.0),
            radius: 6,
            fill: None,
            stroke: Some((RED, 20)),
            dash: None,
        });
        p.runs(
            bx + 2.0 + 10.0,
            by + 2.0 + 2.0 + baseline(12),
            &[run(label, 12, Bold, RED)],
        );
        ry = by + h;
    }
    ry += 8.0;
    p.aligned(
        (left, right),
        ry + baseline(13),
        Right,
        &[
            run("No:", 13, Bold, INK),
            run(format!(" {}", bill.invoice), 13, Regular, INK),
        ],
    );
    ry += line_height(13);
    p.aligned(
        (left, right),
        ry + baseline(13),
        Right,
        &[
            run("Date:", 13, Bold, INK),
            run(format!(" {}", display_date(bill.date)), 13, Regular, INK),
        ],
    );
    ry += line_height(13);
    y = ly.max(ry) + 12.0;
    p.fill((left, y, content, 3.0), RULE);
    y += 3.0;

    // Bill To: the name in bold, the mobile after it, the address below.
    y += 13.0;
    let mut bill_to = vec![
        run("Bill To:", 14, Regular, INK),
        run(" ", 14, Regular, INK),
        run(bill.customer, 14, Bold, INK),
    ];
    if !party.customer_mobile.is_empty() {
        bill_to.push(run(
            format!(" · {}", party.customer_mobile),
            14,
            Regular,
            INK,
        ));
    }
    p.runs(left, y + baseline(14), &bill_to);
    y += line_height(14);
    for line in p.wrap(party.customer_address, 380.0, 12, Regular) {
        p.runs(left, y + baseline(12), &[run(line, 12, Regular, INK)]);
        y += line_height(12);
    }
    y += 13.0;

    // The items table (`.estbl`, collapsed 1px borders, auto column widths over 100%).
    let num = if simple { Right } else { Left };
    let pack_align = if simple { Right } else { Center };
    let mut head: Vec<(&str, TextAlign)> = vec![("#", Left), ("Item", Left)];
    let rest = if simple { Right } else { Center };
    head.extend([("Pack", rest), ("MRP", rest)]);
    if !simple {
        head.push(("Pcs/Pkt", rest));
    }
    head.extend([("Rate", rest), ("Pkt", rest), ("Amount", rest)]);
    let header: Vec<Cell> = head
        .iter()
        .map(|(label, align)| Cell::text(*label, 14, Bold, *align))
        .collect();
    let mut body: Vec<Vec<Cell>> = Vec::new();
    for (index, line) in bill.lines.iter().enumerate() {
        let detail = party.lines.get(index);
        let crate_line =
            detail.is_some_and(|d| !d.contents.is_empty()) || line.label.starts_with("Crate: ");
        let mut row = vec![
            Cell::text((index + 1).to_string(), 14, Regular, Left),
            Cell::text(line.label.as_ref(), 14, Regular, Left),
        ];
        if crate_line {
            row[1].lines.push(vec![run(
                detail.map_or("", |d| d.contents.as_str()),
                10,
                Regular,
                INK,
            )]);
            row.push(Cell::text("crate", 14, Regular, pack_align));
            row.push(Cell::text("—", 14, Regular, num));
            if !simple {
                row.push(Cell::text("", 14, Regular, num));
            }
        } else {
            row.push(Cell::text(
                detail.map_or("", |d| d.pack.as_str()),
                14,
                Regular,
                pack_align,
            ));
            row.push(Cell::text(
                detail
                    .and_then(|d| d.mrp)
                    .filter(|mrp| mrp.minor_units() > 0)
                    .map_or_else(String::new, rupees),
                14,
                Regular,
                num,
            ));
            if !simple {
                row.push(Cell::text(
                    detail.map_or_else(String::new, |d| d.packet_quantity.to_string()),
                    14,
                    Regular,
                    num,
                ));
            }
        }
        row.push(Cell::text(format_amount(line.price), 14, Regular, num));
        row.push(Cell::text(line.quantity.to_string(), 14, Regular, num));
        row.push(Cell::text(rupees(line.amount), 14, Regular, num));
        body.push(row);
    }
    let columns = header.len();
    // Max- and min-content widths per column, padding included (th 7px, td 8px each side).
    let mut max = vec![0.0_f32; columns];
    let mut min = vec![0.0_f32; columns];
    let mut measure_cell = |cell: &Cell, column: usize, pad: f32, p: &Painter<'_>| {
        for line in &cell.lines {
            max[column] = max[column].max(p.width(line) + pad);
            for (text, size, style, _) in line {
                for word in text.split_whitespace() {
                    min[column] = min[column].max(measure(word, *size, *style) + pad);
                }
            }
        }
    };
    for (column, cell) in header.iter().enumerate() {
        measure_cell(cell, column, 14.0, &p);
    }
    for row in &body {
        for (column, cell) in row.iter().enumerate() {
            measure_cell(cell, column, 16.0, &p);
        }
    }
    let (sum_max, sum_min): (f32, f32) = (max.iter().sum(), min.iter().sum());
    let widths: Vec<f32> = if sum_max <= content {
        max.iter()
            .map(|w| w + (content - sum_max) * w / sum_max.max(1.0))
            .collect()
    } else if sum_min < content {
        let share = (content - sum_min) / (sum_max - sum_min).max(1.0);
        max.iter()
            .zip(&min)
            .map(|(max, min)| min + (max - min) * share)
            .collect()
    } else {
        min
    };
    let edges: Vec<f32> = std::iter::once(left)
        .chain(widths.iter().scan(left, |x, w| {
            *x += w;
            Some(*x)
        }))
        .collect();
    let draw_row = |p: &mut Painter<'_>, y: f32, cells: &[Cell], pad: (f32, f32), band: bool| {
        // Wrapped lines per cell, then the row height (vertical-align: middle).
        let wrapped: Vec<Vec<Vec<Run>>> = cells
            .iter()
            .enumerate()
            .map(|(column, cell)| {
                let room = widths[column] - 2.0 * pad.1;
                cell.lines
                    .iter()
                    .flat_map(|line| match line.as_slice() {
                        [(text, size, style, color)] if p.width(line) > room => p
                            .wrap(text, room, *size, *style)
                            .into_iter()
                            .map(|text| vec![run(text, *size, *style, *color)])
                            .collect(),
                        _ => vec![line.clone()],
                    })
                    .collect()
            })
            .collect();
        let heights: Vec<f32> = wrapped
            .iter()
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| line.iter().map(|r| line_height(r.1)).fold(0.0, f32::max))
                    .sum::<f32>()
            })
            .collect();
        let height = heights.iter().copied().fold(0.0, f32::max) + 2.0 * pad.0;
        if band {
            p.fill((left, y, content, height), BAND);
        }
        for (column, lines) in wrapped.iter().enumerate() {
            let (x0, x1) = (edges[column], edges[column + 1]);
            p.stroke((x0, y, x1 - x0, height), GRID, 10);
            let mut ty = y + (height - heights[column]) / 2.0;
            for line in lines {
                let size = line.iter().map(|r| r.1).max().unwrap_or(14);
                p.aligned(
                    (x0 + pad.1, x1 - pad.1),
                    ty + baseline(size),
                    cells[column].align,
                    line,
                );
                ty += line_height(size);
            }
        }
        height
    };
    y += draw_row(&mut p, y, &header, (7.0, 7.0), true);
    for row in &body {
        y += draw_row(&mut p, y, row, (6.0, 8.0), false);
    }
    // tfoot: "Items Total" over the first columns, then the quantity and the amount (800).
    let span = columns - 2;
    let foot_h = 2.0 * 8.0 + line_height(14);
    p.fill((left, y, content, foot_h), BAND);
    for (x0, x1) in [
        (edges[0], edges[span]),
        (edges[span], edges[span + 1]),
        (edges[span + 1], edges[span + 2]),
    ] {
        p.stroke((x0, y, x1 - x0, foot_h), GRID, 10);
    }
    let fy = y + 8.0 + baseline(14);
    p.aligned(
        (edges[0] + 8.0, edges[span] - 8.0),
        fy,
        Right,
        &[run("Items Total", 14, Bold, INK)],
    );
    p.aligned(
        (edges[span] + 8.0, edges[span + 1] - 8.0),
        fy,
        num,
        &[run(bill.items_quantity.to_string(), 14, Bold, INK)],
    );
    p.aligned(
        (edges[span + 1] + 8.0, edges[span + 2] - 8.0),
        fy,
        num,
        &[run(rupees(bill.items_total), 14, Bold, INK)],
    );
    y += foot_h;

    // Totals: a right-aligned table (min-width 300, 2px spacing, cells padded 3px 10px).
    y += 10.0;
    let use_live = bill.balance_mode == BalanceMode::Latest;
    let rate = loading_rate.filter(|rate| rate.minor_units() > 0);
    let mut loading_label = vec![run("Loading/Delivery", 14, Regular, INK)];
    if let Some(rate) = rate {
        // `Math.round(loadAmt / loadRate * 100) / 100`, in paise.
        let packets = Money::from_minor_units(
            (i128::from(bill.loading.minor_units()) * 100 * 2 / i128::from(rate.minor_units()) + 1)
                .div_euclid(2)
                .try_into()
                .unwrap_or(i64::MAX),
        );
        loading_label.push(run(
            format!(
                " ({} pkt × ₹{}/pkt)",
                format_amount(packets),
                format_amount(rate)
            ),
            11,
            Regular,
            INK,
        ));
    }
    let mut rows: Vec<(Vec<Run>, Run, f32)> = vec![
        (
            loading_label,
            run(rupees(bill.loading), 14, Regular, INK),
            3.0,
        ),
        (
            vec![run("Bill Amount", 14, Bold, INK)],
            run(rupees(bill.total), 14, Bold, INK),
            3.0,
        ),
    ];
    if !use_live && let Some(previous) = bill.previous_balance.filter(|p| !p.amount.is_zero()) {
        let mut label = vec![run("Prev Balance", 14, Regular, INK)];
        if let Some(date) = previous.date {
            label.push(run(
                format!(" (as on {})", display_date(date)),
                11,
                Regular,
                INK,
            ));
        }
        rows.push((label, run(rupees(previous.amount), 14, Regular, INK), 3.0));
    }
    rows.push((
        vec![run(
            if use_live {
                "Paid on this bill"
            } else {
                "Paid"
            },
            14,
            Regular,
            INK,
        )],
        run(rupees(bill.paid), 14, Regular, GREEN),
        3.0,
    ));
    let mut due = vec![run(
        if use_live { "Total Due" } else { "Balance Due" },
        14,
        Bold,
        INK,
    )];
    if use_live {
        due.push(run(
            format!(" (as on {})", display_date(bill.today)),
            11,
            Bold,
            INK,
        ));
    }
    let due_color = if bill.balance.minor_units() > 0 {
        RED
    } else {
        GREEN
    };
    rows.push((due, run(rupees(bill.balance), 14, Bold, due_color), 7.0));
    let label_w = rows
        .iter()
        .map(|(label, _, _)| p.width(label))
        .fold(0.0, f32::max)
        + 20.0;
    let value_w = rows
        .iter()
        .map(|(_, value, _)| p.width(std::slice::from_ref(value)))
        .fold(0.0, f32::max)
        + 20.0;
    let natural = label_w + value_w + 3.0 * 2.0;
    let (label_w, value_w) = if natural < 300.0 {
        let extra = 300.0 - natural;
        (
            label_w + extra * label_w / (label_w + value_w),
            value_w + extra * value_w / (label_w + value_w),
        )
    } else {
        (label_w, value_w)
    };
    let table_left = right - (label_w + value_w + 6.0);
    let last = rows.len() - 1;
    y += 2.0;
    for (index, (label, value, pad)) in rows.iter().enumerate() {
        let h = 2.0 * pad + line_height(14);
        if index == last {
            p.fill((table_left + 2.0, y, label_w, h), BAND);
            p.fill((table_left + 4.0 + label_w, y, value_w, h), BAND);
        }
        let by = y + pad + baseline(14);
        p.runs(table_left + 2.0 + 10.0, by, label);
        p.aligned(
            (table_left + 4.0 + label_w, right - 2.0 - 10.0),
            by,
            Right,
            std::slice::from_ref(value),
        );
        y += h + 2.0;
    }
    if use_live {
        y += 7.0;
        p.aligned(
            (left, right),
            y + baseline(11),
            Right,
            &[run(
                "Balance is the running total today, including every bill and payment to date.",
                11,
                Italic,
                INK,
            )],
        );
        y += line_height(11);
    }
    let save = mrp_savings(bill, party.lines);
    if simple && save.minor_units() > 0 {
        y += 12.0;
        let h = 2.0 * 1.5 + 2.0 * 9.0 + line_height(16);
        p.ops.push(DrawOp::RoundRect {
            x: px(left + 0.75),
            y: px(y + 0.75),
            w: px(content - 1.5),
            h: px(h - 1.5),
            radius: 6,
            fill: Some(0xf0fdf4),
            stroke: Some((GREEN, 15)),
            dash: Some((4, 3)),
        });
        p.aligned(
            (left, right),
            y + 1.5 + 9.0 + baseline(16),
            Center,
            &[run(
                format!("You saved ₹{} on MRP!", format_amount(save)),
                16,
                Bold,
                GREEN,
            )],
        );
        y += h;
    }

    // Footer: slogan and E& OE on the left; "For <shop>", a blank line, the rule, Proprietor.
    y += 26.0;
    let lh = line_height(11);
    p.runs(
        left,
        y + baseline(11),
        &[run(
            "Go paperless, save trees & reduce carbon emission",
            11,
            Italic,
            INK,
        )],
    );
    p.runs(
        left,
        y + lh + baseline(11),
        &[run("E& OE", 11, Regular, INK)],
    );
    p.aligned(
        (left, right),
        y + baseline(11),
        Right,
        &[
            run("For ", 11, Regular, INK),
            run(party.business, 11, Bold, INK),
        ],
    );
    for (row, text) in [(2.0, "______________________"), (3.0, "Proprietor")] {
        p.aligned(
            (left, right),
            y + row * lh + baseline(11),
            Right,
            &[run(text, 11, Regular, INK)],
        );
    }
    y += 4.0 * lh;
    let invoice_bottom = y + 26.0 + 1.5;
    // `.invoice` border (1.5px #334155, radius 6), then `htmlToPNG`'s height: +8, at least 128.
    p.ops.push(DrawOp::RoundRect {
        x: px(outer_left + 0.75),
        y: 1,
        w: px(outer_width - 1.5),
        h: px(invoice_bottom - 1.5),
        radius: 6,
        fill: None,
        stroke: Some((0x334155, 15)),
        dash: None,
    });
    let height = px(invoice_bottom.max(120.0)) + 8;
    let mut ops = vec![DrawOp::FillRect {
        x: 0,
        y: 0,
        w: BILL_IMAGE_WIDTH,
        h: height,
        color: 0xffffff,
    }];
    ops.append(&mut p.ops);
    BillImage {
        width: BILL_IMAGE_WIDTH,
        height,
        scale: BILL_IMAGE_SCALE,
        ops,
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::super::bill_image::{BillImageLine, BillImageParty, DrawOp, FontStyle};
    use super::super::{BalanceMode, PreviousBalance};
    use super::invoice_image_layout;
    use crate::confirmation::estimate::{EstimateDocument, EstimateRow};
    use crate::{BillDate, Money};

    fn rs(paise: i64) -> Money {
        Money::from_minor_units(paise)
    }

    #[allow(clippy::cast_precision_loss)]
    fn measure(text: &str, size: u32, _: FontStyle) -> f32 {
        text.chars().count() as f32 * size as f32 * 0.5
    }

    fn texts(ops: &[DrawOp]) -> Vec<&str> {
        ops.iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn bill(mode: BalanceMode, cancelled: bool) -> EstimateDocument<'static> {
        let day = BillDate::new(2025, 4, 30).unwrap();
        EstimateDocument {
            invoice: 7,
            date: day,
            customer: "Synthetic Customer",
            lines: vec![
                EstimateRow {
                    label: Cow::Borrowed("Synthetic Lime"),
                    quantity: 2,
                    price: rs(12_050),
                    amount: rs(24_100),
                },
                EstimateRow {
                    label: Cow::Borrowed("Crate: Party"),
                    quantity: 1,
                    price: rs(40_000),
                    amount: rs(40_000),
                },
            ],
            items_quantity: 3,
            items_total: rs(64_100),
            loading: rs(1_050),
            total: rs(65_150),
            paid: rs(10_000),
            cancelled,
            balance_mode: mode,
            balance: rs(75_150),
            previous_balance: Some(PreviousBalance {
                amount: rs(20_000),
                date: Some(BillDate::new(2025, 4, 1).unwrap()),
            }),
            today: day,
        }
    }

    const LINES: [BillImageLine; 2] = [
        BillImageLine {
            pack: String::new(),
            packet_quantity: 12,
            mrp: Some(Money::from_minor_units(1_500)),
            contents: String::new(),
        },
        BillImageLine {
            pack: String::new(),
            packet_quantity: 1,
            mrp: None,
            contents: String::new(),
        },
    ];

    fn party(lines: &[BillImageLine]) -> BillImageParty<'_> {
        BillImageParty {
            business: "ALM Synthetic",
            tagline: "Wholesale",
            address: "1 Synthetic Road",
            mobile: "90000 00000",
            profile: None,
            customer_mobile: "90000 00001",
            customer_address: "",
            lines,
        }
    }

    #[test]
    fn the_simple_invoice_has_seven_columns_crate_contents_loading_and_balance_rows() {
        let mut lines = LINES.to_vec();
        lines[1].contents = "2cs Synthetic Lime + 3pc Synthetic Soda".into();
        let image = invoice_image_layout(
            &bill(BalanceMode::AsOnBillDate, false),
            &party(&lines),
            Some(rs(350)),
            true,
            &measure,
        );
        let all = texts(&image.ops);
        for expected in [
            "Amount",
            "Crate: Party",
            "2cs Synthetic Lime + 3pc Synthetic Soda",
            "crate",
            "—",
            " (3 pkt × ₹3.5/pkt)",
            "Prev Balance",
            " (as on 01-Apr-2025)",
            "₹200",
            "Balance Due",
            "₹751.5",
            "You saved ₹119 on MRP!",
            "Items Total",
            "₹641",
        ] {
            assert!(all.contains(&expected), "{expected}: {all:?}");
        }
        assert!(!all.contains(&"Pcs/Pkt") && !all.contains(&"CANCELLED"));
        assert_eq!((image.width, image.scale), (760, 2));
        assert!(image.height >= 128);
    }

    #[test]
    fn the_full_invoice_adds_pieces_per_packet_and_latest_shows_the_running_total() {
        let image = invoice_image_layout(
            &bill(BalanceMode::Latest, true),
            &party(&LINES),
            None,
            false,
            &measure,
        );
        let all = texts(&image.ops);
        for expected in [
            "Pcs/Pkt",
            "12",
            "CANCELLED",
            "Paid on this bill",
            "Total Due",
            " (as on 30-Apr-2025)",
            "Balance is the running total today, including every bill and payment to date.",
        ] {
            assert!(all.contains(&expected), "{expected}: {all:?}");
        }
        assert!(!all.contains(&"Prev Balance"));
        assert!(!all.iter().any(|t| t.contains("You saved")));
        assert!(!all.iter().any(|t| t.contains("pkt ×")));
    }
}
