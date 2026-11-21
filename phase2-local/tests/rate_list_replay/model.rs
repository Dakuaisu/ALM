use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct RateListRow {
    pub id: u8,
    pub company: &'static str,
    pub item: &'static str,
    pub pack: &'static str,
    pub unit: &'static str,
    pub quantity: &'static str,
    pub mrp_mills: u32,
    pub rate_mills: u32,
}

pub struct Replay {
    pub products: Vec<RateListRow>,
    pub selected: BTreeSet<u8>,
    known: BTreeSet<u8>,
    pub brands: BTreeSet<&'static str>,
    pub query: String,
}

impl Replay {
    pub fn new(products: Vec<RateListRow>) -> Self {
        Self {
            products,
            selected: BTreeSet::new(),
            known: BTreeSet::new(),
            brands: BTreeSet::new(),
            query: String::new(),
        }
    }
    pub fn visit(&mut self) {
        for p in &self.products {
            if self.known.insert(p.id) {
                self.selected.insert(p.id);
            }
        }
    }
    pub fn shown(&self) -> Vec<u8> {
        let q = self.query.to_lowercase();
        let mut rows: Vec<_> = self
            .products
            .iter()
            .filter(|p| {
                (self.brands.is_empty() || self.brands.contains(brand(p)))
                    && (p.item.to_lowercase().contains(&q) || p.company.to_lowercase().contains(&q))
            })
            .collect();
        rows.sort_by_key(|p| (p.company, p.item));
        rows.into_iter().map(|p| p.id).collect()
    }
    pub fn shown_set(&mut self, on: bool) {
        for id in self.shown() {
            if on {
                self.selected.insert(id);
            } else {
                self.selected.remove(&id);
            }
        }
    }
    pub fn group_toggle(&mut self, group: &str) {
        let ids: Vec<_> = self
            .shown()
            .into_iter()
            .filter(|id| {
                self.products
                    .iter()
                    .any(|p| p.id == *id && brand(p) == group)
            })
            .collect();
        let all = ids.iter().all(|id| self.selected.contains(id));
        for id in ids {
            if all {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
    }
    pub fn chosen(&mut self) -> Vec<RateListRow> {
        self.visit();
        self.products
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .cloned()
            .collect()
    }
    pub fn preview(&mut self) -> Option<String> {
        let list = self.chosen();
        (!list.is_empty()).then(|| html(&list, &self.products))
    }
}

fn brand(p: &RateListRow) -> &str {
    if p.company.is_empty() {
        "Other"
    } else {
        p.company
    }
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn number(mills: u32) -> String {
    let cents = (mills + 5) / 10;
    let whole = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        let remaining = whole.len() - i;
        if i > 0 && (remaining == 3 || (remaining > 3 && (remaining - 3).is_multiple_of(2))) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let fraction = cents % 100;
    if fraction > 0 {
        grouped.push_str(format!(".{fraction:02}").trim_end_matches('0'));
    }
    grouped
}
pub fn picker_rate(row: &RateListRow) -> String {
    format!("₹{}", number(((row.rate_mills + 500) / 1000) * 1000))
}
fn groups(list: &[RateListRow]) -> BTreeMap<&str, Vec<&RateListRow>> {
    let mut result: BTreeMap<&str, Vec<&RateListRow>> = BTreeMap::new();
    for p in list {
        result.entry(brand(p)).or_default().push(p);
    }
    result
}
fn subtitle(list: &[RateListRow], all: &[RateListRow]) -> String {
    let selected = groups(list);
    let total = groups(all).len();
    if !selected.is_empty() && selected.len() <= 3 && selected.len() < total {
        selected.keys().copied().collect::<Vec<_>>().join(", ")
    } else {
        String::new()
    }
}

pub fn html(list: &[RateListRow], all: &[RateListRow]) -> String {
    let sub = subtitle(list, all);
    let sub = if sub.is_empty() {
        String::new()
    } else {
        format!("— {}", esc(&sub))
    };
    let mut h = format!(
        "<div style=\"background:#fff;color:#1a1a1a;padding:22px;max-width:760px;margin:0 auto\">\n    <div style=\"text-align:center;border-bottom:3px solid #2563eb;padding-bottom:10px;margin-bottom:14px\">\n      <div style=\"font-size:22px;font-weight:800;color:#1d4ed8\">Synthetic &amp; Co</div>\n      <div style=\"font-size:12px;color:#555\">Test &lt;only&gt; · Mob: </div>\n      <div style=\"font-size:13px;font-weight:700;margin-top:6px\">RATE LIST {sub} <span style=\"font-weight:400;color:#888\">(21-Feb-2025)</span></div>\n    </div>"
    );
    for (co, products) in groups(list) {
        h.push_str(&format!("<div style=\"font-weight:700;color:#1d4ed8;margin:10px 0 4px;font-size:14px\">{}</div>\n    <table style=\"width:100%;border-collapse:collapse;font-size:12.5px;margin-bottom:8px\">\n    <thead><tr style=\"background:#eef2fb\"><th style=\"text-align:left;padding:5px 8px;border:1px solid #dbe3f4\">Item</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Pack</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">MRP</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Our Rate</th></tr></thead><tbody>", esc(co)));
        for p in products {
            let mrp = if p.mrp_mills == 0 {
                String::new()
            } else {
                format!("₹{}", number(p.mrp_mills))
            };
            h.push_str(&format!("<tr><td style=\"padding:4px 8px;border:1px solid #eee\">{}</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:center\">{} ×{}</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right\">{mrp}</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right;font-weight:700\">₹{}</td></tr>", esc(p.item), esc(&format!("{} {}", p.pack, p.unit)), esc(p.quantity), number(p.rate_mills)));
        }
        h.push_str("</tbody></table>");
    }
    h.push_str("<div style=\"font-size:11px;color:#888;margin-top:8px\">Rates are per case/packet and subject to change. * E.&O.E.</div></div>");
    h
}

pub fn text(list: &[RateListRow], all: &[RateListRow]) -> String {
    let sub = subtitle(list, all);
    let sub = if sub.is_empty() {
        String::new()
    } else {
        format!(" ({sub})")
    };
    let mut t = format!("*Synthetic & Co — Rate List{sub}*\n21-Feb-2025\n\n");
    for (co, products) in groups(list) {
        t.push_str(&format!("*{co}*\n"));
        for p in products {
            t.push_str(&format!("{} — ₹{}\n", p.item, number(p.rate_mills)));
        }
        t.push('\n');
    }
    t
}
pub fn uri(text: &str) -> String {
    let mut result = String::from("https://wa.me/?text=");
    for &b in text.as_bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            result.push(char::from(b));
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}

#[derive(Default)]
pub struct Sinks {
    pub attempts: Vec<(&'static str, &'static str, &'static str)>,
    pub documents: Vec<String>,
    pub chats: Vec<String>,
    pub notices: Vec<&'static str>,
    pub hint_durations: Vec<u32>,
}
impl Sinks {
    pub fn print(&mut self, replay: &mut Replay, popup: bool) {
        let list = replay.chosen();
        if list.is_empty() {
            self.notices.push("Tick at least one item first");
        } else {
            self.attempts.push(("", "_rl", "width=820,height=900"));
            if !popup {
                self.notices
                    .push("Pop-up blocked — allow pop-ups for this page to print");
            } else {
                self.documents.push(format!("<html><head><title>Rate List</title></head><body style=\"margin:0\">{}<script type=\"application/x-alm-inert\">window.onload=()=>window.print()</script></body></html>", html(&list, &replay.products)));
            }
        }
    }
    pub fn pdf(&mut self, replay: &mut Replay, popup: bool) {
        self.print(replay, popup);
        self.notices.push("In the dialog, choose 'Save as PDF'");
        self.hint_durations.push(3800);
    }
    pub fn whatsapp(&mut self, replay: &mut Replay) {
        let list = replay.chosen();
        if list.is_empty() {
            self.notices.push("Tick at least one item first");
        } else {
            self.chats.push(uri(&text(&list, &replay.products)));
        }
    }
}
