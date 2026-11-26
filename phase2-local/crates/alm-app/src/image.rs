//! Rasterizes a core bill-image display list to PNG bytes with the bundled Noto Sans Regular
//! and Bold (OFL, `assets/fonts/OFL.txt`). Only drawing happens here: positions, text and colours come from core.
use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use alm_core::confirmation::bill_image::{BillImage, DrawOp, FontStyle, TextAlign};
use tiny_skia::{
    Color, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Rect, Stroke, StrokeDash, Transform,
};

const FONT: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

/// The face for a style: Noto Sans Bold for bold and semibold (the nearest bundled weight),
/// Regular otherwise; italic is slanted from Regular.
fn font(style: FontStyle) -> Result<FontRef<'static>, String> {
    let bytes = match style {
        FontStyle::Bold | FontStyle::SemiBold => BOLD,
        FontStyle::Regular | FontStyle::Italic => FONT,
    };
    FontRef::try_from_slice(bytes).map_err(|error| error.to_string())
}

/// CSS `Npx` font size as an ab_glyph scale (em size, not line height).
fn scale(font: &FontRef<'_>, size_px: f32) -> PxScale {
    let em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size_px * font.height_unscaled() / em)
}

/// Text advance width in CSS pixels, as `measureText(text).width`, in the style's face.
pub fn measure(text: &str, size: u32, style: FontStyle) -> f32 {
    let Ok(font) = font(style) else { return 0.0 };
    #[allow(clippy::cast_precision_loss)]
    let scaled = font.as_scaled(scale(&font, size as f32));
    let mut width = 0.0;
    let mut previous = None;
    for c in text.chars() {
        let id = scaled.glyph_id(c);
        if let Some(prev) = previous {
            width += scaled.kern(prev, id);
        }
        width += scaled.h_advance(id);
        previous = Some(id);
    }
    width
}

fn color(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color::from_rgba8(r, g, b, 255)
}

fn paint(rgb: u32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color(rgb));
    paint.anti_alias = true;
    paint
}

#[allow(clippy::cast_precision_loss)]
fn px(value: i32) -> f32 {
    value as f32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn draw_text(
    pixmap: &mut Pixmap,
    factor: f32,
    op: (i32, i32, u32, FontStyle, TextAlign, u32, &str),
) -> Result<(), String> {
    let (x, y, size, style, align, rgb, text) = op;
    let font = &font(style)?;
    let width = measure(text, size, style);
    let left = match align {
        TextAlign::Left => px(x),
        TextAlign::Center => px(x) - width / 2.0,
        TextAlign::Right => px(x) - width,
    };
    let scaled = font.as_scaled(scale(font, size as f32 * factor));
    let skew = if matches!(style, FontStyle::Italic) {
        0.2
    } else {
        0.0
    };
    let [_, r, g, b] = rgb.to_be_bytes();
    let (w, h) = (pixmap.width(), pixmap.height());
    {
        let mut caret = left * factor;
        let baseline = px(y) * factor;
        let mut previous = None;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            if let Some(prev) = previous {
                caret += scaled.kern(prev, id);
            }
            let glyph = id.with_scale_and_position(scaled.scale(), point(caret, baseline));
            caret += scaled.h_advance(id);
            previous = Some(id);
            let Some(outline) = font.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outline.px_bounds();
            let pixels = pixmap.pixels_mut();
            outline.draw(|gx, gy, coverage| {
                let row = bounds.min.y + gy as f32;
                let shift = (baseline - row) * skew;
                let col = (bounds.min.x + gx as f32 + shift).round();
                let row = row.round();
                if col < 0.0 || row < 0.0 || col >= w as f32 || row >= h as f32 {
                    return;
                }
                let index = row as usize * w as usize + col as usize;
                let dst = pixels[index];
                let a = coverage.clamp(0.0, 1.0);
                let mix =
                    |s: u8, d: u8| (f32::from(s) * a + f32::from(d) * (1.0 - a)).round() as u8;
                if let Some(blended) = PremultipliedColorU8::from_rgba(
                    mix(r, dst.red()),
                    mix(g, dst.green()),
                    mix(b, dst.blue()),
                    255,
                ) {
                    pixels[index] = blended;
                }
            });
        }
    }
    Ok(())
}

/// A rectangle with quarter-circle corners of `r` (approximated by quadratic curves).
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut b = PathBuilder::new();
    b.move_to(x + r, y);
    b.line_to(x + w - r, y);
    b.quad_to(x + w, y, x + w, y + r);
    b.line_to(x + w, y + h - r);
    b.quad_to(x + w, y + h, x + w - r, y + h);
    b.line_to(x + r, y + h);
    b.quad_to(x, y + h, x, y + h - r);
    b.line_to(x, y + r);
    b.quad_to(x, y, x + r, y);
    b.close();
    b.finish()
}

/// Draws every operation at the image's scale and encodes the result as PNG.
pub fn render_png(image: &BillImage) -> Result<Vec<u8>, String> {
    let (width, height) = image.pixels();
    let mut pixmap = Pixmap::new(width, height).ok_or("Image size out of range.")?;
    pixmap.fill(Color::WHITE);
    #[allow(clippy::cast_precision_loss)]
    let factor = image.scale as f32;
    let transform = Transform::from_scale(factor, factor);
    for op in &image.ops {
        match op {
            DrawOp::FillRect { x, y, w, h, color } => {
                if let Some(rect) = Rect::from_xywh(px(*x), px(*y), px(*w), px(*h)) {
                    pixmap.fill_rect(rect, &paint(*color), transform, None);
                }
            }
            DrawOp::StrokeRect {
                x,
                y,
                w,
                h,
                color,
                width,
                dash,
            } => {
                if let Some(rect) = Rect::from_xywh(px(*x), px(*y), px(*w), px(*h)) {
                    let path = PathBuilder::from_rect(rect);
                    #[allow(clippy::cast_precision_loss)]
                    let mut stroke = Stroke {
                        width: *width as f32 / 10.0,
                        ..Stroke::default()
                    };
                    #[allow(clippy::cast_precision_loss)]
                    if let Some((on, off)) = dash {
                        stroke.dash = StrokeDash::new(vec![*on as f32, *off as f32], 0.0);
                    }
                    pixmap.stroke_path(&path, &paint(*color), &stroke, transform, None);
                }
            }
            DrawOp::RoundRect {
                x,
                y,
                w,
                h,
                radius,
                fill,
                stroke,
                dash,
            } => {
                if let Some(path) = rounded(px(*x), px(*y), px(*w), px(*h), px(*radius as i32)) {
                    if let Some(color) = fill {
                        pixmap.fill_path(
                            &path,
                            &paint(*color),
                            tiny_skia::FillRule::Winding,
                            transform,
                            None,
                        );
                    }
                    if let Some((color, width)) = stroke {
                        #[allow(clippy::cast_precision_loss)]
                        let mut line = Stroke {
                            width: *width as f32 / 10.0,
                            ..Stroke::default()
                        };
                        #[allow(clippy::cast_precision_loss)]
                        if let Some((on, off)) = dash {
                            line.dash = StrokeDash::new(vec![*on as f32, *off as f32], 0.0);
                        }
                        pixmap.stroke_path(&path, &paint(*color), &line, transform, None);
                    }
                }
            }
            DrawOp::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                let mut builder = PathBuilder::new();
                builder.move_to(px(*x1), px(*y1));
                builder.line_to(px(*x2), px(*y2));
                if let Some(path) = builder.finish() {
                    #[allow(clippy::cast_precision_loss)]
                    let stroke = Stroke {
                        width: *width as f32 / 10.0,
                        ..Stroke::default()
                    };
                    pixmap.stroke_path(&path, &paint(*color), &stroke, transform, None);
                }
            }
            DrawOp::Text {
                x,
                y,
                size,
                style,
                align,
                color,
                text,
            } => {
                draw_text(
                    &mut pixmap,
                    factor,
                    (*x, *y, *size, *style, *align, *color, text),
                )?;
            }
        }
    }
    pixmap.encode_png().map_err(|error| error.to_string())
}
