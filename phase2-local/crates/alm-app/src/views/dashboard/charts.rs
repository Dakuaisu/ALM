//! Inline SVG for the Dashboard charts. Every coordinate, label and colour comes from
//! `alm_core::confirmation::{LineChart, Donut}`; this only writes the elements.
use std::fmt::Write as _;

use alm_core::confirmation::{ChartPoint, Donut, LineChart};

use crate::gst_print::esc;

fn pt(point: ChartPoint) -> String {
    format!("{:.1},{:.1}", point.x, point.y)
}

pub fn line_chart(chart: &LineChart, title: &str) -> String {
    let mut svg = format!(
        r#"<svg class="chart" viewBox="0 0 {w} {h}" width="100%" height="230" role="img" aria-label="{}" xmlns="http://www.w3.org/2000/svg" font-size="11" font-family="Segoe UI, sans-serif">"#,
        esc(title),
        w = chart.width,
        h = chart.height
    );
    for (y, label) in &chart.grid {
        let _ = write!(
            svg,
            r#"<line x1="46" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="var(--line)" stroke-width="1"/><text x="4" y="{:.1}" fill="var(--mut)" class="priv">{}</text>"#,
            f64::from(chart.width) - 12.0,
            y + 3.0,
            esc(label)
        );
    }
    if let (Some(first), Some(last)) = (chart.points.first(), chart.points.last()) {
        let line: Vec<String> = chart.points.iter().copied().map(pt).collect();
        let _ = write!(
            svg,
            r##"<defs><linearGradient id="trend-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2563eb" stop-opacity="0.2"/><stop offset="1" stop-color="#2563eb" stop-opacity="0"/></linearGradient></defs><polygon points="{} {:.1},{:.1} {:.1},{:.1}" fill="url(#trend-fill)"/><polyline points="{}" fill="none" stroke="#2563eb" stroke-width="2.4" stroke-linejoin="round" stroke-linecap="round"/>"##,
            line.join(" "),
            last.x,
            chart.baseline,
            first.x,
            chart.baseline,
            line.join(" ")
        );
        for point in &chart.points {
            let _ = write!(
                svg,
                r##"<circle cx="{:.1}" cy="{:.1}" r="2.6" fill="#2563eb"/>"##,
                point.x, point.y
            );
        }
    }
    for (x, label) in &chart.x_labels {
        let _ = write!(
            svg,
            r#"<text x="{x:.1}" y="{}" text-anchor="middle" fill="var(--mut)">{}</text>"#,
            chart.height - 7,
            esc(label)
        );
    }
    svg + "</svg>"
}

pub fn donut(chart: &Donut, title: &str) -> String {
    let mut svg = format!(
        r#"<svg class="chart" viewBox="0 0 {w} {h}" width="100%" height="230" role="img" aria-label="{}" xmlns="http://www.w3.org/2000/svg" font-size="12" font-family="Segoe UI, sans-serif">"#,
        esc(title),
        w = chart.width,
        h = chart.height
    );
    let center = pt(chart.center);
    for slice in &chart.slices {
        for (from, to) in &slice.arcs {
            let _ = write!(
                svg,
                r#"<path d="M{center} L{} A{r:.1},{r:.1} 0 0 1 {} Z" fill="{}"/>"#,
                pt(*from),
                pt(*to),
                slice.color,
                r = chart.radius
            );
        }
    }
    let _ = write!(
        svg,
        r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="var(--panel)"/>"#,
        chart.center.x, chart.center.y, chart.inner
    );
    for slice in &chart.slices {
        let _ = write!(
            svg,
            r#"<rect x="{:.1}" y="{:.1}" width="11" height="11" rx="3" fill="{}"/><text x="{:.1}" y="{:.1}" fill="var(--txt)">{}</text>"#,
            chart.legend_x,
            slice.legend_y - 8.0,
            slice.color,
            chart.legend_x + 16.0,
            slice.legend_y + 1.0,
            esc(&slice.label)
        );
    }
    svg + "</svg>"
}
