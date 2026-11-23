//! Stroke icons copied from ALM_Business_Manager.html L2035–2037 (ICONS) and L2140–2161 (navIcon).

pub fn nav(id: &str) -> String {
    let path = match id {
        "counter" => {
            r#"<path d="M2.4 5.6h11.2l-.9 8a1 1 0 0 1-1 .9H4.3a1 1 0 0 1-1-.9l-.9-8Z"/><path d="M5.6 7.4V4.3a2.4 2.4 0 0 1 4.8 0v3.1"/>"#
        }
        "dashboard" => {
            r#"<rect x="2" y="2" width="5.2" height="5.2" rx="1"/><rect x="8.8" y="2" width="5.2" height="5.2" rx="1"/><rect x="2" y="8.8" width="5.2" height="5.2" rx="1"/><rect x="8.8" y="8.8" width="5.2" height="5.2" rx="1"/>"#
        }
        "bill" => {
            r#"<path d="M4 1.8h5.2L12 4.5V14l-1.4-1-1.3 1-1.3-1-1.3 1L4 13.8V1.8z"/><path d="M6 6h4M6 8.6h4"/>"#
        }
        "sales" => r#"<path d="M2 12.5l3.6-4 2.5 2.2 4.3-4.7"/><path d="M10 6h2.8v2.8"/>"#,
        "purchases" => {
            r#"<path d="M8 1.8l5.5 2.8L8 7.4 2.5 4.6 8 1.8z"/><path d="M2.5 4.6v6.3L8 13.8l5.5-2.9V4.6"/><path d="M8 7.6v6"/>"#
        }
        "expenses" => {
            r#"<path d="M4 2h6.5a1 1 0 011 1v11l-2-1.2-1.7 1.2-1.7-1.2L4 14V2z"/><path d="M6 5.2h4M6 7.8h4"/>"#
        }
        "customers" => {
            r#"<circle cx="6" cy="6" r="2.2"/><path d="M2.4 13c0-2.1 1.7-3.3 3.6-3.3S9.6 10.9 9.6 13"/><path d="M10.6 6.3a2 2 0 010 3.4M11.4 13c0-1.6-.7-2.7-1.7-3.2"/>"#
        }
        "stock" => {
            r#"<path d="M8 1.8l5.6 2.8L8 7.4 2.4 4.6 8 1.8z"/><path d="M2.4 8l5.6 2.8L13.6 8"/><path d="M2.4 11.2L8 14l5.6-2.8"/>"#
        }
        "ratelist" => {
            r#"<path d="M7.4 2H3v4.4l6.6 6.6 4.4-4.4L7.4 2z"/><circle cx="5.2" cy="4.2" r=".9"/>"#
        }
        _ => r#"<circle cx="8" cy="8" r="2"/>"#,
    };
    format!(
        r#"<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"#
    )
}

pub fn icon(name: &str, size: u8) -> String {
    let path = match name {
        "cash" => {
            r#"<rect x="1.7" y="4" width="12.6" height="8" rx="1.4"/><circle cx="8" cy="8" r="1.9"/><path d="M4.3 6.3v3.4M11.7 6.3v3.4"/>"#
        }
        "alert" => r#"<path d="M8 2.5 2 13.2h12L8 2.5Z"/><path d="M8 6.6v3.1M8 11.4v.05"/>"#,
        "check" => r#"<path d="M3.2 8.4 6.5 11.7 12.9 5.2"/>"#,
        _ => "",
    };
    format!(
        r#"<svg viewBox="0 0 16 16" width="{size}" height="{size}" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"#
    )
}
