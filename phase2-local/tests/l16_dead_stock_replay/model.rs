use std::collections::BTreeMap;

pub const CUTOFF: &str = "2024-10-04"; // L16-02 verified clock fixture.

#[derive(Clone, Debug)]
pub struct Product {
    pub id: &'static str,
    pub name: &'static str,
    pub packet_qty: f64,
    pub opening: f64,
    pub buy: Option<f64>,
    pub layers: Vec<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub pid: Option<&'static str>,
    pub name: &'static str,
    pub qty: f64,
    pub pieces: bool,
    pub packet_qty: Option<f64>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Line(Line),
    Crate { qty: f64, components: Vec<Line> },
}

#[derive(Clone, Debug)]
pub struct Sale {
    pub id: Option<&'static str>,
    pub date: &'static str,
    pub cancelled: bool,
    pub items: Vec<Item>,
}

#[derive(Debug, PartialEq)]
pub struct Row {
    pub id: &'static str,
    pub on_hand: f64,
    pub last: Option<&'static str>,
    pub raw: f64,
    pub displayed: String,
    pub fifo: f64,
}

#[derive(Debug)]
pub struct Report {
    pub keys: BTreeMap<String, (f64, &'static str)>,
    pub candidates: BTreeMap<&'static str, (f64, Option<&'static str>)>,
    pub rows: Vec<Row>,
    pub empty: Option<&'static str>,
}

fn expand(line: &Line, multiplier: f64, products: &[Product]) -> (String, f64) {
    let product = line
        .pid
        .and_then(|id| products.iter().find(|p| p.id == id))
        .or_else(|| products.iter().find(|p| p.name == line.name));
    let key = line
        .pid
        .map(str::to_owned)
        .or_else(|| product.map(|p| p.id.to_owned()))
        .unwrap_or_else(|| format!("n:{}", line.name));
    let packet_qty = line
        .packet_qty
        .or_else(|| product.map(|p| p.packet_qty))
        .unwrap_or(1.0);
    let qty = line.qty * multiplier / if line.pieces { packet_qty } else { 1.0 };
    (key, qty)
}

pub fn report(products: &[Product], sales: &[Sale]) -> Report {
    let mut keys = BTreeMap::<String, (f64, &'static str)>::new();
    for sale in sales {
        println!(
            "model bill id={:?} date={} cancelled={}",
            sale.id, sale.date, sale.cancelled
        );
    }
    for sale in sales.iter().filter(|s| !s.cancelled) {
        for item in &sale.items {
            let (multiplier, lines): (f64, &[Line]) = match item {
                Item::Line(line) => (1.0, std::slice::from_ref(line)),
                Item::Crate { qty, components } => (*qty, components),
            };
            for line in lines {
                let (key, qty) = expand(line, multiplier, products);
                let entry = keys.entry(key).or_insert((0.0, sale.date));
                entry.0 += qty;
                entry.1 = entry.1.max(sale.date);
            }
        }
    }
    let mut rows = Vec::new();
    let mut candidates = BTreeMap::new();
    for product in products {
        let (sold, last) = keys
            .get(product.id)
            .map_or((0.0, None), |(qty, date)| (*qty, Some(*date)));
        let on_hand = product.opening - sold;
        candidates.insert(product.id, (on_hand, last));
        let raw = on_hand * product.buy.unwrap_or(0.0);
        let mut consumed = sold.max(0.0);
        let fifo = product
            .layers
            .iter()
            .map(|(qty, rate)| {
                let used = consumed.min(*qty);
                consumed -= used;
                (qty - used) * rate
            })
            .sum();
        println!(
            "model candidate id={} cutoff={CUTOFF} sold={sold} on_hand={on_hand} last={last:?} raw={raw} rounded=₹{} fifo={fifo}",
            product.id,
            raw.round()
        );
        if on_hand > 0.0 && last.is_none_or(|date| date < CUTOFF) {
            rows.push(Row {
                id: product.id,
                on_hand,
                last,
                raw,
                displayed: format!("₹{}", raw.round()),
                fifo,
            });
        }
    }
    rows.sort_by(|a, b| b.raw.total_cmp(&a.raw)); // stable like JS Array.sort.
    rows.truncate(15);
    let empty = rows.is_empty().then_some("No dead stock 👍");
    println!("model keys={keys:?} rows={rows:?} empty={empty:?}");
    Report {
        keys,
        candidates,
        rows,
        empty,
    }
}
